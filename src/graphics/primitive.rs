use core::marker::PhantomData;

use crate::graphics::{Face, FrontFace, topology::PrimitiveTopology};

pub struct PrimitiveState<T> {
    _topology: PhantomData<T>,
    // strip_index_format: None,
    pub(crate) front_face: FrontFace,
    pub(crate) cull_mode: Option<Face>,
    // polygon_mode: wgpu::PolygonMode::Fill,
    // unclipped_depth: false,
    // conservative: false,
}

impl<T> PrimitiveState<T> {
    #[allow(unused_variables)]
    pub fn new(topology: PrimitiveTopology<T>) -> Self {
        Self {
            _topology: PhantomData,
            front_face: FrontFace::Ccw,
            cull_mode: None,
        }
    }

    pub fn with_cull_mode(mut self, cull_mode: Face) -> Self {
        self.cull_mode = Some(cull_mode);
        self
    }

    pub fn with_front_face(mut self, front_face: FrontFace) -> Self {
        self.front_face = front_face;
        self
    }
}
