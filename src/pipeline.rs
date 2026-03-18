use std::{fmt::Debug, marker::PhantomData};

use rayon::prelude::*;

use crate::{
    graphics::{rasterizer::Rasterizer, topology::Primitive},
    pipeline::{
        shader::{FragmentShader, VertexInput, VertexShader},
        varying::Varying,
    },
};

pub mod shader;
pub mod varying;

pub struct Pipeline<T, R, V, F> {
    _marker: PhantomData<T>,
    rasterizer: R,
    vertex_shader: V,
    fragment_shader: F,
}

impl<T, R, V, F> Pipeline<T, R, V, F> {
    pub fn new(rasterizer: R, vertex_shader: V, fragment_shader: F) -> Self {
        Self {
            _marker: PhantomData,
            rasterizer,
            vertex_shader,
            fragment_shader,
        }
    }

    pub fn draw<Var, C, U>(
        &mut self,
        vertives: &[VertexInput<V::Vertex, V::Varying>],
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
    {
        let output = vertives
            .iter()
            .enumerate()
            .map(|v| self.vertex_shader.vs_main(v.0, v.1, uniform))
            .collect::<Vec<_>>();

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
                    T::assemble(&output[..]),
                    width,
                    height,
                    0,
                    tile_y,
                    width,
                    current_tile_height,
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
    pub fn draw_indexed<Var, C, U>(
        &mut self,
        vertives: &[VertexInput<V::Vertex, V::Varying>],
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
    {
        let output = indexed
            .map(|idx| self.vertex_shader.vs_main(idx, &vertives[idx], uniform))
            .collect::<Vec<_>>();

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
                    T::assemble(&output[..]),
                    width,
                    height,
                    0,
                    tile_y,
                    width,
                    current_tile_height,
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
}
