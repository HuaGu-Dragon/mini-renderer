use wgpu::util::DeviceExt;

use crate::{
    backend::wgpu::pipeline::{WgpuRenderPipeline, WgpuRenderPipelineDescriptor},
    graphics::topology::{LineList, LineStrip, PointList, TriangleList, TriangleStrip},
};

pub mod error;
pub mod pipeline;

pub type Result<T> = std::result::Result<T, error::WgpuRendererError>;

pub struct WgpuRenderer {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl WgpuRenderer {
    pub async fn new(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let surface = target.into();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        let surface = instance
            .create_surface(surface)
            .map_err(error::WgpuRendererError::CreateSurface)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
            .await
            .map_err(error::WgpuRendererError::RequestAdapterError)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(error::WgpuRendererError::RequestDeviceError)?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::all(),
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        Ok(Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn begin_frame(&mut self) -> Result<WgpuFrame> {
        let wgpu::CurrentSurfaceTexture::Success(output) = self.surface.get_current_texture()
        else {
            return Err(error::WgpuRendererError::SurfaceError);
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mini-renderer encoder"),
            });

        Ok(WgpuFrame {
            output,
            view,
            encoder,
        })
    }

    pub fn submit(&self, frame: WgpuFrame) {
        self.queue.submit(core::iter::once(frame.encoder.finish()));
        self.queue.present(frame.output);
    }

    pub fn create_render_pipeline<T, V>(
        &self,
        descriptor: WgpuRenderPipelineDescriptor<'_, T, V>,
    ) -> WgpuRenderPipeline
    where
        T: WgpuTopology,
        V: for<'a> WgpuVertex<'a>,
    {
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: descriptor.label,
                bind_group_layouts: descriptor.bind_group_layouts,
                immediate_size: 0,
            });

        let vertex_buffer = [Some(V::layout())];

        let raw = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: descriptor.label,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: descriptor.shader,
                    entry_point: descriptor.vertex_entry,
                    compilation_options: Default::default(),
                    buffers: &vertex_buffer,
                },
                primitive: wgpu::PrimitiveState {
                    topology: T::TOPOLOGY,
                    strip_index_format: descriptor.strip_index_format,
                    // TODO: make it configurable
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: descriptor.depth_stencil,
                multisample: descriptor.multisample,
                fragment: Some(wgpu::FragmentState {
                    module: descriptor.shader,
                    entry_point: descriptor.fragment_entry,
                    compilation_options: Default::default(),
                    targets: descriptor.color_targets,
                }),
                multiview_mask: None,
                cache: None,
            });

        WgpuRenderPipeline { raw }
    }

    pub fn create_vertex_buffer<V>(&self, vertices: &[V]) -> wgpu::Buffer
    where
        V: for<'a> WgpuVertex<'a>,
    {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
    }

    pub fn create_index_buffer<I>(&self, indices: &[I]) -> wgpu::Buffer
    where
        I: bytemuck::NoUninit,
    {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            })
    }
}

pub struct WgpuFrame {
    output: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}

pub trait WgpuTopology {
    const TOPOLOGY: wgpu::PrimitiveTopology;
}

impl WgpuTopology for PointList {
    const TOPOLOGY: wgpu::PrimitiveTopology = wgpu::PrimitiveTopology::PointList;
}

impl WgpuTopology for LineList {
    const TOPOLOGY: wgpu::PrimitiveTopology = wgpu::PrimitiveTopology::LineList;
}
impl WgpuTopology for LineStrip {
    const TOPOLOGY: wgpu::PrimitiveTopology = wgpu::PrimitiveTopology::LineStrip;
}

impl WgpuTopology for TriangleList {
    const TOPOLOGY: wgpu::PrimitiveTopology = wgpu::PrimitiveTopology::TriangleList;
}
impl WgpuTopology for TriangleStrip {
    const TOPOLOGY: wgpu::PrimitiveTopology = wgpu::PrimitiveTopology::TriangleStrip;
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

pub trait WgpuVertexFormat: bytemuck::Pod {
    const FORMAT: wgpu::VertexFormat;
}

pub trait WgpuVertex<'a>
where
    Self: 'a + bytemuck::NoUninit,
{
    fn layout() -> wgpu::VertexBufferLayout<'a>;
}
