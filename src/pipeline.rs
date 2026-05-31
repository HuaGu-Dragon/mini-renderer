#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::{
    graphics::topology::Primitive,
    pipeline::shader::{VertexOutput, VertexShader},
};

#[cfg(feature = "rayon")]
mod parallel;
#[cfg(not(feature = "rayon"))]
mod serial;
pub mod shader;
pub mod varying;

pub struct Pipeline<T: Primitive<V::Varying>, V: VertexShader, F> {
    _marker: PhantomData<T>,
    rasterizer: T::Rasterizer,
    vertex_shader: V,
    fragment_shader: F,
    vertex_cache: Vec<VertexOutput<V::Varying>>,
    index_cache: Vec<usize>,
}
