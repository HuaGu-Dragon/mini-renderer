use std::{fmt::Debug, marker::PhantomData};

use rayon::prelude::*;

use crate::{
    color::ColorFormat,
    graphics::{rasterizer::Rasterizer, topology::Primitive},
    pipeline::{
        shader::{FragmentShader, VertexInput, VertexOutput, VertexShader},
        varying::Varying,
    },
};

pub mod shader;
pub mod varying;

pub struct Pipeline<T, R, V: VertexShader, F> {
    _marker: PhantomData<T>,
    rasterizer: R,
    vertex_shader: V,
    fragment_shader: F,
    vertex_cache: Vec<VertexOutput<V::Varying>>,
    index_cache: Vec<usize>,
}

impl<T, R, V: VertexShader, F> Pipeline<T, R, V, F> {
    pub fn new(rasterizer: R, vertex_shader: V, fragment_shader: F) -> Self {
        Self {
            _marker: PhantomData,
            rasterizer,
            vertex_shader,
            fragment_shader,
            vertex_cache: Vec::new(),
            index_cache: Vec::new(),
        }
    }

    #[inline]
    pub(crate) fn draw_indexed_without_depth<Var, U>(
        &mut self,
        vertices: &[VertexInput<V::Vertex, V::Varying>],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [F::Output],
        width: usize,
        height: usize,
        uniform: &U,
    ) where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Debug + Send + Sync,
        U: Sync,
        F::Output: Send,
        V::Vertex: Send + Sync,
    {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.par_extend(
            self.index_cache
                .par_iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        let num_threads = rayon::current_num_threads().max(1);
        let tile_height = height.div_ceil(num_threads);
        let chunk_size = width * tile_height;

        framebuffer
            .par_chunks_mut(chunk_size)
            .enumerate()
            .for_each(|(i, fb_chunk)| {
                let tile_y = i * tile_height;
                let current_tile_height = (height - tile_y).min(tile_height);

                let fragments = self.rasterizer.rasterize_tile(
                    T::assemble(&self.vertex_cache[..]),
                    width,
                    height,
                    [0, tile_y, width, current_tile_height],
                );

                fragments.for_each(|f| {
                    let local_y = f.y - tile_y;
                    let local_idx = f.x + local_y * width;
                    let Some(output) = self.fragment_shader.fs_main(&f.varying, uniform) else {
                        return;
                    };
                    fb_chunk[local_idx] = output;
                });
            });
    }

    #[inline]
    pub(crate) fn draw_indexed_without_depth_blend<Var, C, U>(
        &mut self,
        vertices: &[VertexInput<V::Vertex, V::Varying>],
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [C::Output],
        width: usize,
        height: usize,
        uniform: &U,
    ) where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Debug + Send + Sync,
        U: Sync,
        C: ColorFormat,
        C::Output: Send,
        V::Vertex: Send + Sync,
    {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.par_extend(
            self.index_cache
                .par_iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        let num_threads = rayon::current_num_threads().max(1);
        let tile_height = height.div_ceil(num_threads);
        let chunk_size = width * tile_height;

        framebuffer
            .par_chunks_mut(chunk_size)
            .enumerate()
            .for_each(|(i, fb_chunk)| {
                let tile_y = i * tile_height;
                let current_tile_height = (height - tile_y).min(tile_height);

                let fragments = self.rasterizer.rasterize_tile(
                    T::assemble(&self.vertex_cache[..]),
                    width,
                    height,
                    [0, tile_y, width, current_tile_height],
                );

                fragments.for_each(|f| {
                    let local_y = f.y - tile_y;
                    let local_idx = f.x + local_y * width;
                    let Some(output) = self.fragment_shader.fs_main(&f.varying, uniform) else {
                        return;
                    };
                    fb_chunk[local_idx] =
                        F::blend(output, C::from_output(fb_chunk[local_idx])).to_output();
                });
            });
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub(crate) fn draw_indexed<Var, C, U>(
        &mut self,
        vertices: &[VertexInput<V::Vertex, V::Varying>],
        indexed: impl Iterator<Item = usize>,
        depth_buffer: &mut [f32],
        framebuffer: &mut [C],
        width: usize,
        height: usize,
        uniform: &U,
    ) where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Debug + Send + Sync,
        U: Sync,
        C: Send,
        V::Vertex: Send + Sync,
    {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.par_extend(
            self.index_cache
                .par_iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        let num_threads = rayon::current_num_threads().max(1);
        let tile_height = height.div_ceil(num_threads);
        let chunk_size = width * tile_height;

        framebuffer
            .par_chunks_mut(chunk_size)
            .zip(depth_buffer.par_chunks_mut(chunk_size))
            .enumerate()
            .for_each(|(i, (fb_chunk, db_chunk))| {
                let tile_y = i * tile_height;
                let current_tile_height = (height - tile_y).min(tile_height);

                let fragments = self.rasterizer.rasterize_tile(
                    T::assemble(&self.vertex_cache[..]),
                    width,
                    height,
                    [0, tile_y, width, current_tile_height],
                );

                fragments.for_each(|f| {
                    let local_y = f.y - tile_y;
                    let local_idx = f.x + local_y * width;
                    if f.depth < db_chunk[local_idx] {
                        let Some(output) = self.fragment_shader.fs_main(&f.varying, uniform) else {
                            return;
                        };
                        fb_chunk[local_idx] = output;
                        db_chunk[local_idx] = f.depth;
                    }
                });
            });
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn draw_indexed_with_depth_blend<Var, C, U>(
        &mut self,
        vertices: &[VertexInput<V::Vertex, V::Varying>],
        indexed: impl Iterator<Item = usize>,
        depth_buffer: &mut [f32],
        framebuffer: &mut [C::Output],
        width: usize,
        height: usize,
        uniform: &U,
    ) where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Debug + Send + Sync,
        U: Sync,
        C: ColorFormat,
        C::Output: Send,
        V::Vertex: Send + Sync,
    {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.par_extend(
            self.index_cache
                .par_iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        let num_threads = rayon::current_num_threads().max(1);
        let tile_height = height.div_ceil(num_threads);
        let chunk_size = width * tile_height;

        framebuffer
            .par_chunks_mut(chunk_size)
            .zip(depth_buffer.par_chunks_mut(chunk_size))
            .enumerate()
            .for_each(|(i, (fb_chunk, db_chunk))| {
                let tile_y = i * tile_height;
                let current_tile_height = (height - tile_y).min(tile_height);

                let fragments = self.rasterizer.rasterize_tile(
                    T::assemble(&self.vertex_cache[..]),
                    width,
                    height,
                    [0, tile_y, width, current_tile_height],
                );

                fragments.for_each(|f| {
                    let local_y = f.y - tile_y;
                    let local_idx = f.x + local_y * width;
                    if f.depth < db_chunk[local_idx] {
                        let Some(output) = self.fragment_shader.fs_main(&f.varying, uniform) else {
                            return;
                        };
                        fb_chunk[local_idx] =
                            F::blend(output, C::from_output(fb_chunk[local_idx])).to_output();
                        db_chunk[local_idx] = f.depth;
                    }
                });
            });
    }
}
