use crate::{
    graphics::{primitive::PrimitiveState, topology::Primitive},
    pipeline::{
        Pipeline,
        shader::{FragmentShader, ShaderProgram, VertexShader},
    },
};

#[allow(clippy::type_complexity)]
pub fn create_render_pipeline<T, S>(
    shader: &S,
    primitive: PrimitiveState<T>,
) -> Pipeline<
    T,
    T::Rasterizer,
    impl VertexShader<Vertex = S::Vertex, Varying = S::Varying>,
    impl FragmentShader<Varying = S::Varying, Output = S::Output>,
>
where
    T: Primitive<S::Varying>,
    S: ShaderProgram,
{
    let rasterizer = T::rasterizer(primitive.front_face);

    let vertex_shader = shader.vertex_shader();
    let fragment_shader = shader.fragment_shader();

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
