use crate::{
    graphics::{
        primitive::{PrimitiveAssembler, PrimitiveState},
        rasterizer::{Rasterizer, TriangleRasterizer},
        topology::PrimitiveTopology,
    },
    pipeline::{
        Pipeline,
        shader::{FragmentShader, ShaderProgram, VertexShader},
    },
};

#[allow(clippy::type_complexity)]
pub fn create_render_pipeline<S>(
    shader: &S,
    primitive: PrimitiveState,
) -> Pipeline<
    impl VertexShader<Vertex = S::Vertex, Varying = S::Varying>,
    impl FragmentShader<Varying = S::Varying, Output = S::Output>,
    // TODO: change it to an trait or something else? to support different rasterizers and not need heap allocations
    impl Rasterizer<S::Varying>,
>
where
    S: ShaderProgram,
{
    let assembler = PrimitiveAssembler::new(primitive.topology);
    let rasterizer = match primitive.topology {
        PrimitiveTopology::TriangleList => TriangleRasterizer::new(primitive.front_face),
    };

    let vertex_shader = shader.vertex_shader();
    let fragment_shader = shader.fragment_shader();

    Pipeline::new(vertex_shader, fragment_shader, rasterizer, assembler)
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
