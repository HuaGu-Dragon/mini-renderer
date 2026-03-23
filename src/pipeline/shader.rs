use crate::math::Vec4;

#[derive(Debug, Clone, Copy)]
pub struct VertexOutput<Varying> {
    pub position: Vec4,
    pub varying: Varying,
}

pub trait VertexShader {
    type Vertex;
    type Uniform;

    fn vs_main(
        &self,
        index: usize,
        vertex: &Self::Vertex,
        uniform: &Self::Uniform,
    ) -> VertexOutput<Self::Varying>;
}

pub trait FragmentShader {
    type Varying;
    type Output: Copy;
    type Uniform;

    fn fs_main(&self, varying: &Self::Varying, uniform: &Self::Uniform) -> Option<Self::Output>;

    #[allow(unused_variables)]
    fn blend(output: Self::Output, background: Self::Output) -> Self::Output {
        output
    }
}
