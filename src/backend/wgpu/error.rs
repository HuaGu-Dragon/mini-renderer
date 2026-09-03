#[derive(Debug)]
pub enum WgpuRendererError {
    SurfaceError,
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapterError(wgpu::RequestAdapterError),
    RequestDeviceError(wgpu::RequestDeviceError),
}
