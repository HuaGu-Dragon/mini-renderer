use std::sync::Arc;

use mini_renderer::{
    backend::wgpu::{
        WgpuRenderPassDescriptor, WgpuRenderer, WgpuRendererError, WgpuVertex,
        pipeline::WgpuRenderPipelineDescriptor,
    },
    graphics::topology::TriangleList,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

impl WgpuVertex for Vertex {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x3,
    ];
}

const VERTICES: [Vertex; 3] = [
    Vertex {
        position: [0.0, 0.6],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.6, -0.6],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.6, -0.6],
        color: [0.0, 0.0, 1.0],
    },
];

const INDICES: [u16; 3] = [0, 1, 2];

struct State {
    window: Arc<Window>,
    renderer: WgpuRenderer,
    pipeline: mini_renderer::backend::wgpu::pipeline::WgpuRenderPipeline,
    vertex_buffer: mini_renderer::backend::wgpu::buffer::VertexBuffer<Vertex>,
    index_buffer: mini_renderer::backend::wgpu::buffer::IndexBuffer<u16>,
}

impl State {
    async fn new(window: Arc<Window>) -> mini_renderer::backend::wgpu::Result<Self> {
        let size = window.inner_size();
        let renderer = WgpuRenderer::new(window.clone(), size.width, size.height).await?;
        let shader = renderer
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("wgpu triangle shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgpu_triangle.wgsl").into()),
            });
        let color_targets = [Some(wgpu::ColorTargetState {
            format: renderer.surface_format(),
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = renderer.create_render_pipeline(
            WgpuRenderPipelineDescriptor::<TriangleList, Vertex>::new(&shader, &color_targets)
                .with_label("wgpu triangle pipeline")
                .with_cull_mode(Some(wgpu::Face::Back)),
        );
        let vertex_buffer =
            renderer.create_vertex_buffer_with_label(Some("triangle vertices"), &VERTICES);
        let index_buffer =
            renderer.create_index_buffer_with_label(Some("triangle indices"), &INDICES);

        Ok(Self {
            window,
            renderer,
            pipeline,
            vertex_buffer,
            index_buffer,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    fn render(&mut self) -> mini_renderer::backend::wgpu::Result<()> {
        let mut frame = self.renderer.begin_frame()?;
        let pass = frame.with_render_pass(
            WgpuRenderPassDescriptor::clear(wgpu::Color {
                r: 0.02,
                g: 0.02,
                b: 0.03,
                a: 1.0,
            })
            .with_label("main render pass"),
        );

        pass.pipeline(&self.pipeline)
            .vertex_buffer(0, &self.vertex_buffer)
            .index_buffer(&self.index_buffer)
            .draw_indexed(0..INDICES.len() as u32, 0, 0..1);

        self.renderer.submit(frame);
        Ok(())
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default().with_title("mini-renderer wgpu"))
                .expect("failed to create window"),
        );
        let state =
            pollster::block_on(State::new(window)).expect("failed to initialize the wgpu renderer");
        state.window.request_redraw();
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                match state.render() {
                    Ok(())
                    | Err(WgpuRendererError::ZeroSizedSurface)
                    | Err(WgpuRendererError::SurfaceTimeout)
                    | Err(WgpuRendererError::SurfaceOccluded)
                    | Err(WgpuRendererError::SurfaceOutdated) => {}
                    Err(error) => {
                        eprintln!("rendering failed: {error}");
                        event_loop.exit();
                    }
                }
                state.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop
        .run_app(&mut App::default())
        .expect("event loop failed");
}
