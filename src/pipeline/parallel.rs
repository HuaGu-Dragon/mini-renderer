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

const TILE_WIDTH: usize = 64;
const TILE_HEIGHT: usize = 32;
const MIN_BINNED_PRIMITIVES: usize = 512;

#[allow(clippy::too_many_arguments)]
fn build_tile_bins<Var, R>(
    rasterizer: &R,
    primitives: &[R::Primitive<Var>],
    width: usize,
    height: usize,
    tile_counts: &mut Vec<usize>,
    tile_offsets: &mut Vec<usize>,
    tile_indices: &mut Vec<usize>,
    primitive_tile_ranges: &mut Vec<Option<[usize; 4]>>,
) -> usize
where
    Var: Varying,
    R: Rasterizer<Var>,
{
    let tiles_x = width.div_ceil(TILE_WIDTH);
    let tiles_y = height.div_ceil(TILE_HEIGHT);
    let tile_count = tiles_x * tiles_y;

    tile_counts.clear();
    tile_counts.resize(tile_count, 0);
    primitive_tile_ranges.clear();
    primitive_tile_ranges.reserve(primitives.len());

    for primitive in primitives {
        let tile_range = rasterizer.primitive_bounds(primitive, width, height).map(
            |[min_x, min_y, max_x, max_y]| {
                [
                    min_x / TILE_WIDTH,
                    min_y / TILE_HEIGHT,
                    max_x.div_ceil(TILE_WIDTH),
                    max_y.div_ceil(TILE_HEIGHT),
                ]
            },
        );

        if let Some([min_x, min_y, max_x, max_y]) = tile_range {
            for tile_y in min_y..max_y {
                for tile_x in min_x..max_x {
                    tile_counts[tile_y * tiles_x + tile_x] += 1;
                }
            }
        }
        primitive_tile_ranges.push(tile_range);
    }

    tile_offsets.clear();
    tile_offsets.reserve(tile_count + 1);
    tile_offsets.push(0);
    for &count in tile_counts.iter() {
        tile_offsets.push(tile_offsets.last().copied().unwrap_or(0) + count);
    }

    tile_indices.clear();
    tile_indices.resize(tile_offsets.last().copied().unwrap_or(0), 0);
    tile_counts.fill(0);

    for (primitive_index, tile_range) in primitive_tile_ranges.iter().copied().enumerate() {
        let Some([min_x, min_y, max_x, max_y]) = tile_range else {
            continue;
        };

        for tile_y in min_y..max_y {
            for tile_x in min_x..max_x {
                let tile_index = tile_y * tiles_x + tile_x;
                let write_index = tile_offsets[tile_index] + tile_counts[tile_index];
                tile_indices[write_index] = primitive_index;
                tile_counts[tile_index] += 1;
            }
        }
    }

    tiles_x
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn rasterize_row<Var, R>(
    rasterizer: &R,
    primitives: &[R::Primitive<Var>],
    tile_offsets: &[usize],
    tile_indices: &[usize],
    tiles_x: usize,
    width: usize,
    height: usize,
    tile_row: usize,
    use_binning: bool,
    mut process_fragment: impl FnMut(Fragment<Var>),
) where
    Var: Varying,
    R: Rasterizer<Var>,
{
    let tile_y = tile_row * TILE_HEIGHT;
    let current_tile_height = (height - tile_y).min(TILE_HEIGHT);

    if !use_binning {
        rasterizer
            .rasterize_tile(
                primitives.iter().copied(),
                width,
                height,
                [0, tile_y, width, current_tile_height],
            )
            .for_each(process_fragment);
        return;
    }

    for tile_column in 0..tiles_x {
        let tile_index = tile_row * tiles_x + tile_column;
        let primitive_indices =
            &tile_indices[tile_offsets[tile_index]..tile_offsets[tile_index + 1]];
        let tile_x = tile_column * TILE_WIDTH;
        let current_tile_width = (width - tile_x).min(TILE_WIDTH);
        let fragments = rasterizer.rasterize_tile(
            primitive_indices
                .iter()
                .map(|&primitive_index| primitives[primitive_index]),
            width,
            height,
            [tile_x, tile_y, current_tile_width, current_tile_height],
        );
        fragments.for_each(&mut process_fragment);
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
            tile_counts: Vec::new(),
            tile_offsets: Vec::new(),
            tile_indices: Vec::new(),
            primitive_tile_ranges: Vec::new(),
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
            self.index_cache
                .par_iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        let primitive_cache: Vec<_> = T::assemble(&self.vertex_cache).collect();
        let use_binning = primitive_cache.len() >= MIN_BINNED_PRIMITIVES;
        let tiles_x = if use_binning {
            build_tile_bins(
                &self.rasterizer,
                &primitive_cache,
                width,
                height,
                &mut self.tile_counts,
                &mut self.tile_offsets,
                &mut self.tile_indices,
                &mut self.primitive_tile_ranges,
            )
        } else {
            0
        };
        let tile_offsets = &self.tile_offsets;
        let tile_indices = &self.tile_indices;
        let rasterizer = &self.rasterizer;
        let fragment_shader = &self.fragment_shader;
        let chunk_size = width * TILE_HEIGHT;

        framebuffer
            .par_chunks_mut(chunk_size)
            .enumerate()
            .for_each(|(tile_row, fb_chunk)| {
                let tile_y = tile_row * TILE_HEIGHT;
                rasterize_row(
                    rasterizer,
                    &primitive_cache,
                    tile_offsets,
                    tile_indices,
                    tiles_x,
                    width,
                    height,
                    tile_row,
                    use_binning,
                    |f| {
                        let local_y = f.y - tile_y;
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
            self.index_cache
                .par_iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        let primitive_cache: Vec<_> = T::assemble(&self.vertex_cache).collect();
        let use_binning = primitive_cache.len() >= MIN_BINNED_PRIMITIVES;
        let tiles_x = if use_binning {
            build_tile_bins(
                &self.rasterizer,
                &primitive_cache,
                width,
                height,
                &mut self.tile_counts,
                &mut self.tile_offsets,
                &mut self.tile_indices,
                &mut self.primitive_tile_ranges,
            )
        } else {
            0
        };
        let tile_offsets = &self.tile_offsets;
        let tile_indices = &self.tile_indices;
        let rasterizer = &self.rasterizer;
        let fragment_shader = &self.fragment_shader;
        let chunk_size = width * TILE_HEIGHT;

        framebuffer
            .par_chunks_mut(chunk_size)
            .enumerate()
            .for_each(|(tile_row, fb_chunk)| {
                let tile_y = tile_row * TILE_HEIGHT;
                rasterize_row(
                    rasterizer,
                    &primitive_cache,
                    tile_offsets,
                    tile_indices,
                    tiles_x,
                    width,
                    height,
                    tile_row,
                    use_binning,
                    |f| {
                        let local_y = f.y - tile_y;
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
            self.index_cache
                .par_iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        let primitive_cache: Vec<_> = T::assemble(&self.vertex_cache).collect();
        let use_binning = primitive_cache.len() >= MIN_BINNED_PRIMITIVES;
        let tiles_x = if use_binning {
            build_tile_bins(
                &self.rasterizer,
                &primitive_cache,
                width,
                height,
                &mut self.tile_counts,
                &mut self.tile_offsets,
                &mut self.tile_indices,
                &mut self.primitive_tile_ranges,
            )
        } else {
            0
        };
        let tile_offsets = &self.tile_offsets;
        let tile_indices = &self.tile_indices;
        let rasterizer = &self.rasterizer;
        let fragment_shader = &self.fragment_shader;
        let chunk_size = width * TILE_HEIGHT;

        framebuffer
            .par_chunks_mut(chunk_size)
            .zip(depth_buffer.par_chunks_mut(chunk_size))
            .enumerate()
            .for_each(|(tile_row, (fb_chunk, db_chunk))| {
                let tile_y = tile_row * TILE_HEIGHT;
                rasterize_row(
                    rasterizer,
                    &primitive_cache,
                    tile_offsets,
                    tile_indices,
                    tiles_x,
                    width,
                    height,
                    tile_row,
                    use_binning,
                    |f| {
                        let local_y = f.y - tile_y;
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
            self.index_cache
                .par_iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        let primitive_cache: Vec<_> = T::assemble(&self.vertex_cache).collect();
        let use_binning = primitive_cache.len() >= MIN_BINNED_PRIMITIVES;
        let tiles_x = if use_binning {
            build_tile_bins(
                &self.rasterizer,
                &primitive_cache,
                width,
                height,
                &mut self.tile_counts,
                &mut self.tile_offsets,
                &mut self.tile_indices,
                &mut self.primitive_tile_ranges,
            )
        } else {
            0
        };
        let tile_offsets = &self.tile_offsets;
        let tile_indices = &self.tile_indices;
        let rasterizer = &self.rasterizer;
        let fragment_shader = &self.fragment_shader;
        let chunk_size = width * TILE_HEIGHT;

        framebuffer
            .par_chunks_mut(chunk_size)
            .zip(depth_buffer.par_chunks_mut(chunk_size))
            .enumerate()
            .for_each(|(tile_row, (fb_chunk, db_chunk))| {
                let tile_y = tile_row * TILE_HEIGHT;
                rasterize_row(
                    rasterizer,
                    &primitive_cache,
                    tile_offsets,
                    tile_indices,
                    tiles_x,
                    width,
                    height,
                    tile_row,
                    use_binning,
                    |f| {
                        let local_y = f.y - tile_y;
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
    fn tile_bins_assign_points_to_matching_tiles() {
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
        let mut tile_counts = Vec::new();
        let mut tile_offsets = Vec::new();
        let mut tile_indices = Vec::new();
        let mut primitive_tile_ranges = Vec::new();

        let tiles_x = build_tile_bins(
            &rasterizer,
            &primitives,
            128,
            64,
            &mut tile_counts,
            &mut tile_offsets,
            &mut tile_indices,
            &mut primitive_tile_ranges,
        );

        assert_eq!(tiles_x, 2);
        assert_eq!(tile_offsets, [0, 1, 1, 1, 2]);
        assert_eq!(tile_indices, [0, 1]);
    }

    #[test]
    fn tile_bins_preserve_primitive_order() {
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
        let mut tile_counts = Vec::new();
        let mut tile_offsets = Vec::new();
        let mut tile_indices = Vec::new();
        let mut primitive_tile_ranges = Vec::new();

        build_tile_bins(
            &rasterizer,
            &primitives,
            128,
            64,
            &mut tile_counts,
            &mut tile_offsets,
            &mut tile_indices,
            &mut primitive_tile_ranges,
        );

        assert_eq!(&tile_indices[tile_offsets[0]..tile_offsets[1]], [0, 1, 2]);
    }
}
