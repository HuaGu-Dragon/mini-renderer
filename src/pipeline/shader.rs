use crate::math::Vec4;

pub struct VertexInput<V, Varying> {
    pub vertex: V,
    pub varying: Varying,
}

#[derive(Debug)]
pub struct VertexOutput<V> {
    pub position: Vec4,
    pub varying: V,
}

pub trait VertexShader {
    type Vertex;
    type Varying;

    fn vs_main(
        &self,
        vertex: &VertexInput<Self::Vertex, Self::Varying>,
    ) -> VertexOutput<Self::Varying>;
}

pub trait FragmentShader {
    type Varying;

    fn fs_main(&self, varying: &Self::Varying) -> [u8; 4];
}

pub trait ShaderProgram {
    type Vertex;
    type Varying;

    fn vertex_shader(&self) -> impl VertexShader<Vertex = Self::Vertex, Varying = Self::Varying>;

    fn fragment_shader(&self) -> impl FragmentShader<Varying = Self::Varying>;
}
