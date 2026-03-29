use std::marker::PhantomData;

use crate::pipeline::shader::{VertexOutput, VertexShader};

#[cfg(feature = "rayon")]
mod parallel;
#[cfg(not(feature = "rayon"))]
mod serial;
pub mod shader;
pub mod varying;

pub struct Pipeline<T, R, V: VertexShader, F> {
    _marker: PhantomData<T>,
    rasterizer: R,
    vertex_shader: V,
    fragment_shader: F,
    vertex_cache: Vec<VertexOutput<V::Varying>>,
    index_cache: Vec<usize>,
}
