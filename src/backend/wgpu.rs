use core::ops::Range;

use wgpu::util::DeviceExt;

use crate::{
    backend::wgpu::buffer::{IndexBuffer, VertexBuffer, WgpuIndex},
    graphics::topology::{LineList, LineStrip, PointList, TriangleList, TriangleStrip},
};

pub mod buffer;
pub mod error;
pub mod pipeline;

pub use error::WgpuRendererError;
pub use pipeline::{WgpuPrimitiveState, WgpuRenderPipeline, WgpuRenderPipelineDescriptor};

pub type Result<T> = std::result::Result<T, WgpuRendererError>;

pub struct WgpuRenderer {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    configured: bool,
}

impl WgpuRenderer {
    pub async fn new(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(target.into())
            .map_err(error::WgpuRendererError::CreateSurface)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(error::WgpuRendererError::RequestAdapter)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(error::WgpuRendererError::RequestDevice)?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);
        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::AutoVsync)
            .unwrap_or(capabilities.present_modes[0]);
        let alpha_mode = capabilities
            .alpha_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::CompositeAlphaMode::Auto)
            .unwrap_or(capabilities.alpha_modes[0]);
        let color_space = wgpu::SurfaceColorSpace::Auto;

        let configured = width > 0 && height > 0;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };

        if configured {
            surface.configure(&device, &config);
        }

        Ok(Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
            configured,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.configured = width > 0 && height > 0;
        self.config.width = width.max(1);
        self.config.height = height.max(1);

        if self.configured {
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn begin_frame(&mut self) -> Result<WgpuFrame> {
        if !self.configured {
            return Err(error::WgpuRendererError::ZeroSizedSurface);
        }

        let (output, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output) => (output, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(output) => (output, true),
            wgpu::CurrentSurfaceTexture::Timeout => {
                return Err(error::WgpuRendererError::SurfaceTimeout);
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                return Err(error::WgpuRendererError::SurfaceOccluded);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Err(error::WgpuRendererError::SurfaceOutdated);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                return Err(error::WgpuRendererError::SurfaceLost);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(error::WgpuRendererError::SurfaceValidation);
            }
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
            suboptimal,
        })
    }

    pub fn submit(&mut self, frame: WgpuFrame) {
        let WgpuFrame {
            output,
            encoder,
            suboptimal,
            ..
        } = frame;

        self.queue.submit(core::iter::once(encoder.finish()));
        self.queue.present(output);

        if suboptimal && self.configured {
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn create_render_pipeline<T, V>(
        &self,
        descriptor: WgpuRenderPipelineDescriptor<'_, T, V>,
    ) -> WgpuRenderPipeline
    where
        T: WgpuTopology,
        V: WgpuVertex,
    {
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: descriptor.label,
                bind_group_layouts: descriptor.bind_group_layouts,
                immediate_size: 0,
            });
        let vertex_buffers = [Some(V::layout())];
        let primitive = descriptor.primitive;

        let raw = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: descriptor.label,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: descriptor.shader,
                    entry_point: descriptor.vertex_entry,
                    compilation_options: Default::default(),
                    buffers: &vertex_buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: T::TOPOLOGY,
                    strip_index_format: primitive.strip_index_format,
                    front_face: primitive.front_face,
                    cull_mode: primitive.cull_mode,
                    unclipped_depth: primitive.unclipped_depth,
                    polygon_mode: primitive.polygon_mode,
                    conservative: primitive.conservative,
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

    pub fn create_vertex_buffer<V: WgpuVertex>(&self, vertices: &[V]) -> buffer::VertexBuffer<V> {
        self.create_vertex_buffer_with_label(None, vertices)
    }

    pub fn create_vertex_buffer_with_label<V: WgpuVertex>(
        &self,
        label: Option<&str>,
        vertices: &[V],
    ) -> buffer::VertexBuffer<V> {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
            .into()
    }

    pub fn create_index_buffer<I: WgpuIndex>(&self, indices: &[I]) -> buffer::IndexBuffer<I> {
        self.create_index_buffer_with_label(None, indices)
    }

    pub fn create_index_buffer_with_label<I: WgpuIndex>(
        &self,
        label: Option<&str>,
        indices: &[I],
    ) -> buffer::IndexBuffer<I> {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            })
            .into()
    }

    pub fn create_depth_texture(
        &self,
        label: Option<&str>,
        format: wgpu::TextureFormat,
    ) -> WgpuDepthTexture {
        WgpuDepthTexture::new(
            &self.device,
            label,
            self.config.width,
            self.config.height,
            format,
        )
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}

pub struct Pass<'a> {
    // render_pass: &'a mut WgpuFrame,
    render_pass: wgpu::RenderPass<'a>,
}

impl<'a> Pass<'a> {
    pub fn pipeline(mut self, pipeline: &'a WgpuRenderPipeline) -> Self {
        self.render_pass.set_pipeline(&pipeline.raw);

        self
    }

    pub fn vertex_buffer<T>(mut self, slot: u32, buffer_slice: &VertexBuffer<T>) -> Self {
        self.render_pass
            .set_vertex_buffer(slot, buffer_slice.slice(..));

        self
    }

    pub fn index_buffer<T: WgpuIndex>(mut self, buffer_slice: &IndexBuffer<T>) -> Self {
        self.render_pass
            .set_index_buffer(buffer_slice.slice(..), T::FORMAT);

        self
    }

    pub fn draw_indexed(mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }
}

pub struct WgpuFrame {
    output: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
    suboptimal: bool,
}

impl WgpuFrame {
    pub fn with_render_pass(
        &mut self,
        descriptor: WgpuRenderPassDescriptor<'_>,
        // render: impl FnOnce(&mut wgpu::RenderPass<'_>) -> R,
    ) -> Pass<'_> {
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &self.view,
            depth_slice: None,
            resolve_target: descriptor.resolve_target,
            ops: descriptor.color_ops,
        })];
        let pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: descriptor.label,
            color_attachments: &color_attachments,
            depth_stencil_attachment: descriptor.depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        Pass { render_pass: pass }

        // render(&mut pass)
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        &mut self.encoder
    }
}

pub struct WgpuRenderPassDescriptor<'a> {
    pub label: Option<&'a str>,
    pub color_ops: wgpu::Operations<wgpu::Color>,
    pub resolve_target: Option<&'a wgpu::TextureView>,
    pub depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
}

impl<'a> WgpuRenderPassDescriptor<'a> {
    pub fn clear(color: wgpu::Color) -> Self {
        Self {
            label: None,
            color_ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
            resolve_target: None,
            depth_stencil_attachment: None,
        }
    }

    pub fn load() -> Self {
        Self {
            label: None,
            color_ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
            resolve_target: None,
            depth_stencil_attachment: None,
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_depth_stencil(
        mut self,
        attachment: wgpu::RenderPassDepthStencilAttachment<'a>,
    ) -> Self {
        self.depth_stencil_attachment = Some(attachment);
        self
    }
}

pub struct WgpuDepthTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    format: wgpu::TextureFormat,
}

impl WgpuDepthTexture {
    fn new(
        device: &wgpu::Device,
        label: Option<&str>,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            format,
        }
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
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

pub trait WgpuVertexFormat: bytemuck::Pod {
    const FORMAT: wgpu::VertexFormat;
}

pub trait WgpuVertex: bytemuck::Pod {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute];
    const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Vertex;

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: Self::STEP_MODE,
            attributes: Self::ATTRIBUTES,
        }
    }
}

macro_rules! impl_vertex_format {
    ($($type:ty => $format:ident),* $(,)?) => {
        $(
            impl WgpuVertexFormat for $type {
                const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::$format;
            }
        )*
    };
}

impl_vertex_format! {
    f32 => Float32,
    [f32; 2] => Float32x2,
    [f32; 3] => Float32x3,
    [f32; 4] => Float32x4,
    i32 => Sint32,
    [i32; 2] => Sint32x2,
    [i32; 3] => Sint32x3,
    [i32; 4] => Sint32x4,
    u32 => Uint32,
    [u32; 2] => Uint32x2,
    [u32; 3] => Uint32x3,
    [u32; 4] => Uint32x4,
    [i16; 2] => Sint16x2,
    [i16; 4] => Sint16x4,
    [u16; 2] => Uint16x2,
    [u16; 4] => Uint16x4,
    [i8; 2] => Sint8x2,
    [i8; 4] => Sint8x4,
    [u8; 2] => Uint8x2,
    [u8; 4] => Uint8x4,
}
