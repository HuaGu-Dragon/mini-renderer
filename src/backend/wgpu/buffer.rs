use std::ops::Deref;

#[repr(transparent)]
pub struct VertexBuffer<T> {
    buffer: wgpu::Buffer,
    _marker: core::marker::PhantomData<T>,
}

#[repr(transparent)]
pub struct IndexBuffer<T> {
    buffer: wgpu::Buffer,
    _marker: core::marker::PhantomData<T>,
}

impl<T> From<wgpu::Buffer> for VertexBuffer<T> {
    fn from(value: wgpu::Buffer) -> Self {
        Self {
            buffer: value,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<T> From<wgpu::Buffer> for IndexBuffer<T> {
    fn from(value: wgpu::Buffer) -> Self {
        Self {
            buffer: value,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<T> Deref for VertexBuffer<T> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T> Deref for IndexBuffer<T> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
