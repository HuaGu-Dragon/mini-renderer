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
