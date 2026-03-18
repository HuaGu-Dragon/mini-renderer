use std::fmt::Debug;

use crate::{
    graphics::{primitive::PrimitiveState, rasterizer::Rasterizer, topology::Primitive},
    pipeline::{
        Pipeline,
        shader::{FragmentShader, VertexInput, VertexShader},
        varying::Varying,
    },
};

pub fn create_render_pipeline<T, VS, FS>(
    vertex_shader: VS,
    fragment_shader: FS,
    primitive: PrimitiveState<T>,
) -> Pipeline<T, T::Rasterizer, VS, FS>
where
    T: Primitive<VS::Varying>,
    VS: VertexShader,
    FS: FragmentShader<Varying = VS::Varying>,
{
    let rasterizer = T::rasterizer(primitive.front_face);

    Pipeline::new(rasterizer, vertex_shader, fragment_shader)
}

pub struct Renderer {
    pub width: usize,
    pub height: usize,
}

pub struct RenderPass<'pass> {
    render: &'pass Renderer,
}

pub struct BoundPipeline<'a, T, R, V, F> {
    render: &'a Renderer,
    pipeline: &'a mut Pipeline<T, R, V, F>,
}

impl Renderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    // pub fn resize(&mut self, width: usize, height: usize) {
    //     if width == self.width && height == self.height {
    //         return;
    //     }

    //     self.width = width;
    //     self.height = height;
    // }

    pub fn begin_render_pass(&self) -> RenderPass<'_> {
        RenderPass { render: self }
    }
}

impl<'pass> RenderPass<'pass> {
    pub fn set_pipeline<'a, T, R, V, F>(
        &'a self,
        pipeline: &'a mut Pipeline<T, R, V, F>,
    ) -> BoundPipeline<'a, T, R, V, F> {
        BoundPipeline {
            render: self.render,
            pipeline,
        }
    }
}

impl<'a, T, R, V, F> BoundPipeline<'a, T, R, V, F> {
    pub fn draw<Var, U, C>(
        &mut self,
        vertices: &[VertexInput<V::Vertex, V::Varying>],
        // instances: Range<u32>,
        framebuffer: &mut [C],
        depth_buffer: &mut [f32],
        uniform: &U,
    ) where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var>,
        V: VertexShader<Varying = Var, Uniform = U>,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C>,
        Var: Varying + Debug,
    {
        self.pipeline.draw(
            vertices,
            depth_buffer,
            framebuffer,
            self.render.width,
            self.render.height,
            uniform,
        );
    }

    pub fn draw_indexed<Var, U, C>(
        &mut self,
        vertices: &[VertexInput<V::Vertex, V::Varying>],
        // instances: Range<u32>,
        indexed: impl Iterator<Item = usize>,
        framebuffer: &mut [C],
        depth_buffer: &mut [f32],
        uniform: &U,
    ) where
        T: Primitive<Var, Rasterizer = R, Primitive<Var> = R::Primitive<Var>>,
        R: Rasterizer<Var>,
        V: VertexShader<Varying = Var, Uniform = U>,
        F: FragmentShader<Varying = Var, Uniform = U, Output = C>,
        Var: Varying + Debug,
    {
        self.pipeline.draw_indexed(
            vertices,
            indexed,
            depth_buffer,
            framebuffer,
            self.render.width,
            self.render.height,
            uniform,
        );
    }
}
