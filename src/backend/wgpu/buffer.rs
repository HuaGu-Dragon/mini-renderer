use std::ops::Deref;

#[repr(transparent)]
pub struct VertexBuffer<T> {
    buffer: wgpu::Buffer,
    _marker: core::marker::PhantomData<T>,
}

#[repr(transparent)]
pub struct IndexBuffer<T: WgpuIndex> {
    buffer: wgpu::Buffer,
    _marker: core::marker::PhantomData<T>,
}

pub trait WgpuIndex: bytemuck::Pod {
    const FORMAT: wgpu::IndexFormat;
}

impl WgpuIndex for u16 {
    const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}

impl WgpuIndex for u32 {
    const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

impl<T> From<wgpu::Buffer> for VertexBuffer<T> {
    fn from(value: wgpu::Buffer) -> Self {
        Self {
            buffer: value,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<T: WgpuIndex> From<wgpu::Buffer> for IndexBuffer<T> {
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

impl<T: WgpuIndex> Deref for IndexBuffer<T> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
