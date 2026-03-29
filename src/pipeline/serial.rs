use std::marker::PhantomData;

use crate::{
    graphics::{rasterizer::Rasterizer, topology::Primitive},
    pipeline::{
        Pipeline,
        shader::{FragmentShader, VertexShader},
        varying::Varying,
    },
};

impl<T, R, V: VertexShader, F> Pipeline<T, R, V, F> {
    pub(crate) fn new(rasterizer: R, vertex_shader: V, fragment_shader: F) -> Self {
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
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
    {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.extend(
            self.index_cache
                .iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        self.rasterizer
            .rasterize(T::assemble(&self.vertex_cache[..]), width, height)
            .for_each(|f| {
                let idx = f.x + f.y * width;
                let Some(output) = self.fragment_shader.fs_main(&f.varying, uniform) else {
                    return;
                };
                framebuffer[idx] = output.into();
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
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
    {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.extend(
            self.index_cache
                .iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        self.rasterizer
            .rasterize(T::assemble(&self.vertex_cache[..]), width, height)
            .for_each(|f| {
                let idx = f.x + f.y * width;
                let Some(output) = self.fragment_shader.fs_main(&f.varying, uniform) else {
                    return;
                };
                framebuffer[idx] = F::blend(output, C::from(framebuffer[idx])).into();
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
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<F::Output> + Send,
        V::Vertex: Send + Sync,
    {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.extend(
            self.index_cache
                .iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        self.rasterizer
            .rasterize(T::assemble(&self.vertex_cache[..]), width, height)
            .for_each(|f| {
                let idx = f.x + f.y * width;
                if f.depth < depth_buffer[idx] {
                    let Some(output) = self.fragment_shader.fs_main(&f.varying, uniform) else {
                        return;
                    };
                    framebuffer[idx] = output.into();
                    depth_buffer[idx] = f.depth;
                }
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
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var> + Sync,
        V: VertexShader<Varying = Var, Uniform = U> + Sync,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C> + Sync,
        Var: Varying + Send + Sync,
        U: Sync,
        C: From<O> + Into<O> + Send,
        O: Send + Copy,
        V::Vertex: Send + Sync,
    {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        self.vertex_cache.clear();
        self.vertex_cache.extend(
            self.index_cache
                .iter()
                .map(|&idx| self.vertex_shader.vs_main(idx, &vertices[idx], uniform)),
        );

        self.rasterizer
            .rasterize(T::assemble(&self.vertex_cache[..]), width, height)
            .for_each(|f| {
                let idx = f.x + f.y * width;
                if f.depth < depth_buffer[idx] {
                    let Some(output) = self.fragment_shader.fs_main(&f.varying, uniform) else {
                        return;
                    };
                    framebuffer[idx] = F::blend(output, C::from(framebuffer[idx])).into();
                    depth_buffer[idx] = f.depth;
                }
            });
    }
}
