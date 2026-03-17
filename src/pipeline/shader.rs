use crate::math::Vec4;

pub struct VertexInput<Var, Varying> {
    pub vertex: Var,
    pub varying: Option<Varying>,
}

#[derive(Debug, Clone, Copy)]
pub struct VertexOutput<Var> {
    pub position: Vec4,
    pub varying: Var,
}

pub trait VertexShader {
    type Vertex;
    type Varying;
    type Uniform;

    fn vs_main(
        &self,
        index: usize,
        vertex: &VertexInput<Self::Vertex, Self::Varying>,
        uniform: &Self::Uniform,
    ) -> VertexOutput<Self::Varying>;
}

pub trait FragmentShader {
    type Varying;
    type Output;
    type Uniform;

    fn fs_main(&self, varying: &Self::Varying, uniform: &Self::Uniform) -> Option<Self::Output>;
}
