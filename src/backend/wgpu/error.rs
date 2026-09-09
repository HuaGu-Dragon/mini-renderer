use core::fmt;

#[derive(Debug)]
pub enum WgpuRendererError {
    ZeroSizedSurface,
    SurfaceTimeout,
    SurfaceOccluded,
    SurfaceOutdated,
    SurfaceLost,
    SurfaceValidation,
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapter(wgpu::RequestAdapterError),
    RequestDevice(wgpu::RequestDeviceError),
}

impl fmt::Display for WgpuRendererError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSizedSurface => formatter.write_str("surface has zero width or height"),
            Self::SurfaceTimeout => {
                formatter.write_str("timed out while acquiring a surface texture")
            }
            Self::SurfaceOccluded => formatter.write_str("surface is currently occluded"),
            Self::SurfaceOutdated => formatter.write_str("surface configuration is outdated"),
            Self::SurfaceLost => formatter.write_str("surface was lost"),
            Self::SurfaceValidation => formatter.write_str("surface texture validation failed"),
            Self::CreateSurface(error) => write!(formatter, "failed to create surface: {error}"),
            Self::RequestAdapter(error) => write!(formatter, "failed to request adapter: {error}"),
            Self::RequestDevice(error) => write!(formatter, "failed to request device: {error}"),
        }
    }
}

impl core::error::Error for WgpuRendererError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::CreateSurface(error) => Some(error),
            Self::RequestAdapter(error) => Some(error),
            Self::RequestDevice(error) => Some(error),
            _ => None,
        }
    }
}
