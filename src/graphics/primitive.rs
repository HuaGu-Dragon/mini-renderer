use crate::graphics::{Face, FrontFace, topology::PrimitiveTopology};

pub struct PrimitiveState<T> {
    pub topology: PrimitiveTopology<T>,
    // strip_index_format: None,
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
    // polygon_mode: wgpu::PolygonMode::Fill,
    // unclipped_depth: false,
    // conservative: false,
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
