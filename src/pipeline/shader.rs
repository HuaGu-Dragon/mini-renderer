pub trait VertexShader {
    type Vertex;
    type VaryingData;
    type Uniform;

    fn vs_main();
}

pub trait FragmentShader {
    type VaryingData;
    type Uniform;

    fn fs_main();
}

pub trait ShaderProgram {
    type Vertex;
    type VaryingData;
    type Uniform;

    fn vertex_shader(
        &self,
    ) -> impl VertexShader<Vertex = Self::Vertex, VaryingData = Self::VaryingData, Uniform = Self::Uniform>;

    fn fragment_shader(
        &self,
    ) -> impl FragmentShader<VaryingData = Self::VaryingData, Uniform = Self::Uniform>;
}
