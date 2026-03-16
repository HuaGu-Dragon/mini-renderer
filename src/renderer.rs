use crate::{
    graphics::{primitive::PrimitiveState, topology::Primitive},
    pipeline::{
        Pipeline,
        shader::{FragmentShader, VertexShader},
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
    width: usize,
    height: usize,
}

pub struct RenderPass<'pass> {
    render: &'pass Renderer,
}

impl Renderer {
    pub fn begin_render_pass(&self) -> RenderPass<'_> {
        RenderPass { render: self }
    }
}

impl RenderPass<'_> {
    pub fn set_pipeline(&mut self) {}
    pub fn draw(&self) {}
}
