use crate::{graphics::color::IntoColor, math::Vec4};

pub struct VertexInput<Var, Varying> {
    pub vertex: Var,
    pub varying: Option<Varying>,
}

#[derive(Debug)]
pub struct VertexOutput<Var> {
    pub position: Vec4,
    pub varying: Var,
}

pub trait VertexShader {
    type Vertex;
    type Varying;

    fn vs_main(
        &self,
        index: usize,
        vertex: &VertexInput<Self::Vertex, Self::Varying>,
    ) -> VertexOutput<Self::Varying>;
}

pub trait FragmentShader {
    type Varying;
    type Output: IntoColor;

    fn fs_main(&self, varying: &Self::Varying) -> Option<Self::Output>;
}

pub trait ShaderProgram {
    type Vertex;
    type Varying;
    type Output: IntoColor;

    fn vertex_shader(&self) -> impl VertexShader<Vertex = Self::Vertex, Varying = Self::Varying>;

    fn fragment_shader(
        &self,
    ) -> impl FragmentShader<Varying = Self::Varying, Output = Option<Self::Output>>;
}
