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

impl<T: Primitive<V::Varying>, V: VertexShader, F> Pipeline<T, V, F> {
    pub(crate) fn checked_target_len(width: usize, height: usize) -> usize {
        width
            .checked_mul(height)
            .expect("renderer dimensions overflow the addressable buffer length")
    }

    pub(crate) fn cache_indices(
        &mut self,
        indexed: impl Iterator<Item = usize>,
        vertex_count: usize,
    ) {
        self.index_cache.clear();
        self.index_cache.extend(indexed);

        if let Some(index) = self
            .index_cache
            .iter()
            .copied()
            .find(|&index| index >= vertex_count)
        {
            panic!("vertex index {index} out of bounds for {vertex_count} vertices");
        }
    }
}
