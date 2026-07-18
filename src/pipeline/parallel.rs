use core::marker::PhantomData;
use rayon::prelude::*;

use crate::{
    graphics::{
        rasterizer::{Fragment, Rasterizer},
        topology::Primitive,
    },
    pipeline::{
        Pipeline,
        shader::{FragmentShader, VertexShader},
        varying::Varying,
    },
};

const ROW_HEIGHT: usize = 32;
const MIN_BINNED_PRIMITIVES: usize = 512;

#[allow(clippy::too_many_arguments)]
fn build_row_bins<Var, R>(
    rasterizer: &R,
    primitives: &[R::Primitive<Var>],
    width: usize,
    height: usize,
    row_counts: &mut Vec<usize>,
    row_offsets: &mut Vec<usize>,
    row_indices: &mut Vec<usize>,
    primitive_row_ranges: &mut Vec<Option<[usize; 2]>>,
) where
    Var: Varying + Send + Sync,
    R: Rasterizer<Var> + Sync,
    R::Primitive<Var>: Sync,
{
    let row_count = height.div_ceil(ROW_HEIGHT);

    row_counts.clear();
    row_counts.resize(row_count, 0);
    primitive_row_ranges.clear();
    primitive_row_ranges.par_extend(primitives.par_iter().map(|primitive| {
        rasterizer
            .primitive_bounds(primitive, width, height)
            .and_then(|[_, min_y, _, max_y]| {
                let min_row = (min_y / ROW_HEIGHT).min(row_count);
                let max_row = max_y.div_ceil(ROW_HEIGHT).min(row_count);
                (min_row < max_row).then_some([min_row, max_row])
            })
    }));

    for [min_row, max_row] in primitive_row_ranges.iter().flatten().copied() {
        for count in &mut row_counts[min_row..max_row] {
            *count += 1;
        }
    }

    row_offsets.clear();
    row_offsets.reserve(row_count + 1);
    row_offsets.push(0);
    for &count in row_counts.iter() {
        row_offsets.push(row_offsets.last().copied().unwrap_or(0) + count);
    }

    row_indices.clear();
    row_indices.resize(row_offsets.last().copied().unwrap_or(0), 0);
    row_counts.fill(0);

    for (primitive_index, row_range) in primitive_row_ranges.iter().copied().enumerate() {
        let Some([min_row, max_row]) = row_range else {
            continue;
        };

        for row in min_row..max_row {
            let write_index = row_offsets[row] + row_counts[row];
            row_indices[write_index] = primitive_index;
            row_counts[row] += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn rasterize_row<Var, R>(
    rasterizer: &R,
    primitives: &[R::Primitive<Var>],
    row_offsets: &[usize],
    row_indices: &[usize],
    width: usize,
    height: usize,
    row: usize,
    use_binning: bool,
    process_fragment: impl FnMut(Fragment<Var>),
) where
    Var: Varying,
    R: Rasterizer<Var>,
{
    let row_y = row * ROW_HEIGHT;
    let current_row_height = (height - row_y).min(ROW_HEIGHT);

    if use_binning {
        let primitive_indices = &row_indices[row_offsets[row]..row_offsets[row + 1]];
        rasterizer
            .rasterize_tile(
                primitive_indices
                    .iter()
                    .map(|&primitive_index| primitives[primitive_index]),
                width,
                height,
                [0, row_y, width, current_row_height],
            )
            .for_each(process_fragment);
    } else {
        rasterizer
            .rasterize_tile(
                primitives.iter().copied(),
                width,
                height,
                [0, row_y, width, current_row_height],
            )
            .for_each(process_fragment);
    }
}

impl<T: Primitive<V::Varying>, V: VertexShader, F> Pipeline<T, V, F> {
    pub(crate) fn new(rasterizer: T::Rasterizer, vertex_shader: V, fragment_shader: F) -> Self {
        Self {
            _marker: PhantomData,
            rasterizer,
            vertex_shader,
            fragment_shader,
            vertex_cache: Vec::new(),
            index_cache: Vec::new(),
            primitive_cache: Vec::new(),
            row_counts: Vec::new(),
            row_offsets: Vec::new(),
            row_indices: Vec::new(),
            primitive_row_ranges: Vec::new(),
        }
    }

    #[inline]
    pub(crate) fn draw_indexed_without_depth<Var, C, U>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [C],
        width: usize,
        height: usize,
        uniform: &U,
    ) where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        assert_eq!(framebuffer.len(), width * height);

        self.index_cache.clear();
        self.index_cache.extend(indexed);
        if width == 0 || height == 0 {
            return;
        }

        self.vertex_cache.clear();
        self.vertex_cache.par_extend(
            vertices
                .par_iter()
                .enumerate()
                .map(|(idx, vertex)| self.vertex_shader.vs_main(idx, vertex, uniform)),
        );

        self.primitive_cache.clear();
        self.primitive_cache
            .extend(T::assemble_indexed(&self.vertex_cache, &self.index_cache));
        let primitive_cache = &self.primitive_cache;
        if primitive_cache.is_empty() {
            return;
        }

        let use_binning = primitive_cache.len() >= MIN_BINNED_PRIMITIVES;
        if use_binning {
            build_row_bins(
                &self.rasterizer,
                primitive_cache,
                width,
                height,
                &mut self.row_counts,
                &mut self.row_offsets,
                &mut self.row_indices,
                &mut self.primitive_row_ranges,
            );
        }
        if use_binning && self.row_indices.is_empty() {
            return;
        }

        let row_offsets = &self.row_offsets;
        let row_indices = &self.row_indices;
        let rasterizer = &self.rasterizer;
        let fragment_shader = &self.fragment_shader;
        let chunk_size = width * ROW_HEIGHT;

        framebuffer
            .par_chunks_mut(chunk_size)
            .enumerate()
            .for_each(|(row, fb_chunk)| {
                let row_y = row * ROW_HEIGHT;
                rasterize_row(
                    rasterizer,
                    primitive_cache,
                    row_offsets,
                    row_indices,
                    width,
                    height,
                    row,
                    use_binning,
                    |f| {
                        let local_y = f.y - row_y;
                        let local_idx = f.x + local_y * width;
                        let Some(output) = fragment_shader.fs_main(&f.varying, uniform) else {
                            return;
                        };
                        fb_chunk[local_idx] = output.into();
                    },
                );
            });
    }

    #[inline]
    pub(crate) fn draw_indexed_without_depth_blend<Var, C, O, U>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [O],
        width: usize,
        height: usize,
        uniform: &U,
    ) where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        assert_eq!(framebuffer.len(), width * height);
        if width == 0 || height == 0 {
            return;
        }

        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.par_extend(
            vertices
                .par_iter()
                .enumerate()
                .map(|(idx, vertex)| self.vertex_shader.vs_main(idx, vertex, uniform)),
        );

        self.primitive_cache.clear();
        self.primitive_cache
            .extend(T::assemble_indexed(&self.vertex_cache, &self.index_cache));
        let primitive_cache = &self.primitive_cache;
        if primitive_cache.is_empty() {
            return;
        }

        let use_binning = primitive_cache.len() >= MIN_BINNED_PRIMITIVES;
        if use_binning {
            build_row_bins(
                &self.rasterizer,
                primitive_cache,
                width,
                height,
                &mut self.row_counts,
                &mut self.row_offsets,
                &mut self.row_indices,
                &mut self.primitive_row_ranges,
            );
        }
        if use_binning && self.row_indices.is_empty() {
            return;
        }

        let row_offsets = &self.row_offsets;
        let row_indices = &self.row_indices;
        let rasterizer = &self.rasterizer;
        let fragment_shader = &self.fragment_shader;
        let chunk_size = width * ROW_HEIGHT;

        framebuffer
            .par_chunks_mut(chunk_size)
            .enumerate()
            .for_each(|(row, fb_chunk)| {
                let row_y = row * ROW_HEIGHT;
                rasterize_row(
                    rasterizer,
                    primitive_cache,
                    row_offsets,
                    row_indices,
                    width,
                    height,
                    row,
                    use_binning,
                    |f| {
                        let local_y = f.y - row_y;
                        let local_idx = f.x + local_y * width;
                        let Some(output) = fragment_shader.fs_main(&f.varying, uniform) else {
                            return;
                        };
                        fb_chunk[local_idx] = F::blend(output, C::from(fb_chunk[local_idx])).into();
                    },
                );
            });
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub(crate) fn draw_indexed<Var, C, U>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        depth_buffer: &mut [f32],
        framebuffer: &mut [C],
        width: usize,
        height: usize,
        uniform: &U,
    ) where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        assert_eq!(framebuffer.len(), width * height);
        assert_eq!(depth_buffer.len(), width * height);
        if width == 0 || height == 0 {
            return;
        }

        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.par_extend(
            vertices
                .par_iter()
                .enumerate()
                .map(|(idx, vertex)| self.vertex_shader.vs_main(idx, vertex, uniform)),
        );

        self.primitive_cache.clear();
        self.primitive_cache
            .extend(T::assemble_indexed(&self.vertex_cache, &self.index_cache));
        let primitive_cache = &self.primitive_cache;
        if primitive_cache.is_empty() {
            return;
        }

        let use_binning = primitive_cache.len() >= MIN_BINNED_PRIMITIVES;
        if use_binning {
            build_row_bins(
                &self.rasterizer,
                primitive_cache,
                width,
                height,
                &mut self.row_counts,
                &mut self.row_offsets,
                &mut self.row_indices,
                &mut self.primitive_row_ranges,
            );
        }
        if use_binning && self.row_indices.is_empty() {
            return;
        }

        let row_offsets = &self.row_offsets;
        let row_indices = &self.row_indices;
        let rasterizer = &self.rasterizer;
        let fragment_shader = &self.fragment_shader;
        let chunk_size = width * ROW_HEIGHT;

        framebuffer
            .par_chunks_mut(chunk_size)
            .zip(depth_buffer.par_chunks_mut(chunk_size))
            .enumerate()
            .for_each(|(row, (fb_chunk, db_chunk))| {
                let row_y = row * ROW_HEIGHT;
                rasterize_row(
                    rasterizer,
                    primitive_cache,
                    row_offsets,
                    row_indices,
                    width,
                    height,
                    row,
                    use_binning,
                    |f| {
                        let local_y = f.y - row_y;
                        let local_idx = f.x + local_y * width;
                        if f.depth < db_chunk[local_idx] {
                            let Some(output) = fragment_shader.fs_main(&f.varying, uniform) else {
                                return;
                            };
                            fb_chunk[local_idx] = output.into();
                            db_chunk[local_idx] = f.depth;
                        }
                    },
                );
            });
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn draw_indexed_with_depth_blend<Var, C, O, U>(
        &mut self,
        vertices: &[V::Vertex],
        indexed: impl Iterator<Item = usize>,
        depth_buffer: &mut [f32],
        framebuffer: &mut [O],
        width: usize,
        height: usize,
        uniform: &U,
    ) where
        T: Primitive<Var>,
        <T as Primitive<Var>>::Rasterizer: Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
        <<T as Primitive<Var>>::Rasterizer as Rasterizer<Var>>::Primitive<Var>: Sync,
    {
        assert_eq!(framebuffer.len(), width * height);
        assert_eq!(depth_buffer.len(), width * height);
        if width == 0 || height == 0 {
            return;
        }

        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.par_extend(
            vertices
                .par_iter()
                .enumerate()
                .map(|(idx, vertex)| self.vertex_shader.vs_main(idx, vertex, uniform)),
        );

        self.primitive_cache.clear();
        self.primitive_cache
            .extend(T::assemble_indexed(&self.vertex_cache, &self.index_cache));
        let primitive_cache = &self.primitive_cache;
        if primitive_cache.is_empty() {
            return;
        }

        let use_binning = primitive_cache.len() >= MIN_BINNED_PRIMITIVES;
        if use_binning {
            build_row_bins(
                &self.rasterizer,
                primitive_cache,
                width,
                height,
                &mut self.row_counts,
                &mut self.row_offsets,
                &mut self.row_indices,
                &mut self.primitive_row_ranges,
            );
        }
        if use_binning && self.row_indices.is_empty() {
            return;
        }

        let row_offsets = &self.row_offsets;
        let row_indices = &self.row_indices;
        let rasterizer = &self.rasterizer;
        let fragment_shader = &self.fragment_shader;
        let chunk_size = width * ROW_HEIGHT;

        framebuffer
            .par_chunks_mut(chunk_size)
            .zip(depth_buffer.par_chunks_mut(chunk_size))
            .enumerate()
            .for_each(|(row, (fb_chunk, db_chunk))| {
                let row_y = row * ROW_HEIGHT;
                rasterize_row(
                    rasterizer,
                    primitive_cache,
                    row_offsets,
                    row_indices,
                    width,
                    height,
                    row,
                    use_binning,
                    |f| {
                        let local_y = f.y - row_y;
                        let local_idx = f.x + local_y * width;
                        if f.depth < db_chunk[local_idx] {
                            let Some(output) = fragment_shader.fs_main(&f.varying, uniform) else {
                                return;
                            };
                            fb_chunk[local_idx] =
                                F::blend(output, C::from(fb_chunk[local_idx])).into();
                            db_chunk[local_idx] = f.depth;
                        }
                    },
                );
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        graphics::{FrontFace, rasterizer::PointRasterizer},
        math::Vec4,
        pipeline::shader::VertexOutput,
    };

    #[test]
    fn row_bins_assign_points_to_matching_rows() {
        let rasterizer = <PointRasterizer as Rasterizer<()>>::new(FrontFace::Ccw, None);
        let primitives = [
            VertexOutput {
                position: Vec4::new(-0.75, 0.5, 0.0, 1.0),
                varying: (),
            },
            VertexOutput {
                position: Vec4::new(0.75, -0.5, 0.0, 1.0),
                varying: (),
            },
        ];
        let mut row_counts = Vec::new();
        let mut row_offsets = Vec::new();
        let mut row_indices = Vec::new();
        let mut primitive_row_ranges = Vec::new();

        build_row_bins(
            &rasterizer,
            &primitives,
            128,
            64,
            &mut row_counts,
            &mut row_offsets,
            &mut row_indices,
            &mut primitive_row_ranges,
        );

        assert_eq!(row_offsets, [0, 1, 2]);
        assert_eq!(row_indices, [0, 1]);
    }

    #[test]
    fn row_bins_preserve_primitive_order() {
        let rasterizer = <PointRasterizer as Rasterizer<()>>::new(FrontFace::Ccw, None);
        let primitives = [
            VertexOutput {
                position: Vec4::new(-0.9, 0.9, 0.0, 1.0),
                varying: (),
            },
            VertexOutput {
                position: Vec4::new(-0.8, 0.8, 0.0, 1.0),
                varying: (),
            },
            VertexOutput {
                position: Vec4::new(-0.7, 0.7, 0.0, 1.0),
                varying: (),
            },
        ];
        let mut row_counts = Vec::new();
        let mut row_offsets = Vec::new();
        let mut row_indices = Vec::new();
        let mut primitive_row_ranges = Vec::new();

        build_row_bins(
            &rasterizer,
            &primitives,
            128,
            64,
            &mut row_counts,
            &mut row_offsets,
            &mut row_indices,
            &mut primitive_row_ranges,
        );

        assert_eq!(&row_indices[row_offsets[0]..row_offsets[1]], [0, 1, 2]);
    }
}
