use core::marker::PhantomData;
use rayon::prelude::*;

use crate::{
    graphics::{rasterizer::Rasterizer, topology::Primitive},
    pipeline::{
        Pipeline,
        shader::{FragmentShader, VertexShader},
        varying::Varying,
    },
};

impl<T: Primitive<V::Varying>, V: VertexShader, F> Pipeline<T, V, F> {
    pub(crate) fn new(rasterizer: T::Rasterizer, vertex_shader: V, fragment_shader: F) -> Self {
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
    {
        let target_len = Self::checked_target_len(width, height);
        assert_eq!(framebuffer.len(), target_len);
        if target_len == 0 {
            return;
        }

        self.cache_indices(indexed, vertices.len());

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
                    fb_chunk[local_idx] = output.into();
                });
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
    {
        let target_len = Self::checked_target_len(width, height);
        assert_eq!(framebuffer.len(), target_len);
        if target_len == 0 {
            return;
        }

        self.cache_indices(indexed, vertices.len());

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
                    fb_chunk[local_idx] = F::blend(output, C::from(fb_chunk[local_idx])).into();
                });
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
    {
        let target_len = Self::checked_target_len(width, height);
        assert_eq!(framebuffer.len(), target_len);
        assert_eq!(depth_buffer.len(), target_len);
        if target_len == 0 {
            return;
        }

        self.cache_indices(indexed, vertices.len());

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
                        fb_chunk[local_idx] = output.into();
                        db_chunk[local_idx] = f.depth;
                    }
                });
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
    {
        let target_len = Self::checked_target_len(width, height);
        assert_eq!(framebuffer.len(), target_len);
        assert_eq!(depth_buffer.len(), target_len);
        if target_len == 0 {
            return;
        }

        self.cache_indices(indexed, vertices.len());

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
                        fb_chunk[local_idx] = F::blend(output, C::from(fb_chunk[local_idx])).into();
                        db_chunk[local_idx] = f.depth;
                    }
                });
            });
    }
}
