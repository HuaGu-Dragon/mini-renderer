use crate::{
    graphics::{Face, FrontFace, topology::PrimitiveTopology},
    pipeline::shader::VertexOutput,
};

pub struct PrimitiveState {
    pub topology: PrimitiveTopology,
    // strip_index_format: None,
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
    // polygon_mode: wgpu::PolygonMode::Fill,
    // unclipped_depth: false,
    // conservative: false,
}

pub enum Primitive<'a, Var> {
    // Point,
    // Line,
    Triangle(
        &'a VertexOutput<Var>,
        &'a VertexOutput<Var>,
        &'a VertexOutput<Var>,
    ),
}

pub struct PrimitiveAssembler {
    pub topology: PrimitiveTopology,
}

impl PrimitiveAssembler {
    pub fn new(topology: PrimitiveTopology) -> Self {
        Self { topology }
    }

    pub fn assemble<'a, Var>(
        &self,
        vertexs: &'a [VertexOutput<Var>],
    ) -> impl Iterator<Item = Primitive<'a, Var>> {
        match self.topology {
            PrimitiveTopology::TriangleList => vertexs
                .chunks_exact(3)
                .map(|chunk| Primitive::Triangle(&chunk[0], &chunk[1], &chunk[2])),
        }
    }
}

pub trait IndexStorage {
    fn as_slice(&self) -> &[usize];
}

impl<const N: usize> IndexStorage for [usize; N] {
    fn as_slice(&self) -> &[usize] {
        self.as_slice()
    }
}

impl IndexStorage for Vec<usize> {
    fn as_slice(&self) -> &[usize] {
        self.as_slice()
    }
}
