#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::marker::PhantomData;

#[cfg(feature = "rayon")]
use crate::graphics::rasterizer::Rasterizer;
use crate::{
    graphics::topology::Primitive,
    pipeline::shader::{VertexOutput, VertexShader},
};

#[cfg(feature = "rayon")]
type AssembledPrimitive<T, V> =
    <<T as Primitive<<V as VertexShader>::Varying>>::Rasterizer as Rasterizer<
        <V as VertexShader>::Varying,
    >>::Primitive<<V as VertexShader>::Varying>;

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
    #[cfg(feature = "rayon")]
    primitive_cache: Vec<AssembledPrimitive<T, V>>,
    #[cfg(feature = "rayon")]
    row_counts: Vec<usize>,
    #[cfg(feature = "rayon")]
    row_offsets: Vec<usize>,
    #[cfg(feature = "rayon")]
    row_indices: Vec<usize>,
    #[cfg(feature = "rayon")]
    primitive_row_ranges: Vec<Option<[usize; 2]>>,
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
