pub trait VertexShader {
    type Vertex;
    type VaryingData;

    fn init() -> Self
    where
        Self: Sized;

    fn vs_main();
}

pub trait FragmentShader {
    type VaryingData;

    fn init() -> Self
    where
        Self: Sized;

    fn fs_main();
}

pub trait ShaderProgram {
    type Vertex;
    type VaryingData;
    type Uniform;

    fn vertex_shader(
        &self,
    ) -> impl VertexShader<Vertex = Self::Vertex, VaryingData = Self::VaryingData>;

    fn fragment_shader(&self) -> impl FragmentShader<VaryingData = Self::VaryingData>;
}
