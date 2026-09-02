#[derive(Debug)]
pub enum WgpuRendererError {
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapterError(wgpu::RequestAdapterError),
    RequestDeviceError(wgpu::RequestDeviceError),
}
