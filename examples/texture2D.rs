use std::f32;
use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::rc::Rc;

use image::{ImageBuffer, Rgb};
use mini_renderer::graphics::color::IntoColor;
use mini_renderer::graphics::primitive::PrimitiveAssembler;
use mini_renderer::graphics::rasterizer::TriangleRasterizer;
use mini_renderer::math::Vec4;
use mini_renderer::pipeline::Pipeline;
use mini_renderer::pipeline::shader::{FragmentShader, VertexInput, VertexOutput, VertexShader};
use mini_renderer::pipeline::varying::Varying;
use softbuffer::{Buffer, Context, Pixel, Surface};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowAttributes, WindowId};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let context = Context::new(event_loop.owned_display_handle()).unwrap();

    let mut app = App {
        context,
        state: AppState::Initial,
    };
    event_loop.run_app(&mut app).unwrap();
}

struct App {
    context: Context<OwnedDisplayHandle>,
    state: AppState,
}

enum AppState {
    Initial,
    Suspended {
        window: Rc<Window>,
    },
    Running {
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
        renderer: Renderer,
    },
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::Init = cause {
            // Create window on startup.
            let window_attrs = WindowAttributes::default().with_title("sandbox");
            let window = event_loop
                .create_window(window_attrs)
                .expect("failed creating window");
            self.state = AppState::Suspended {
                window: Rc::new(window),
            };
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Create or re-create the surface.
        let AppState::Suspended { window } = &mut self.state else {
            unreachable!("got resumed event while not suspended");
        };
        let mut surface =
            Surface::new(&self.context, window.clone()).expect("failed creating surface");

        // TODO: https://github.com/rust-windowing/softbuffer/issues/106
        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            // Resize surface
            surface.resize(width, height).unwrap();
        }

        let renderer = Renderer::new(size.width as usize, size.height as usize);
        self.state = AppState::Running { surface, renderer };
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Drop the surface.
        let AppState::Running { surface, .. } = &mut self.state else {
            unreachable!("got resumed event while not running");
        };
        let window = surface.window().clone();
        self.state = AppState::Suspended { window };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let AppState::Running { surface, renderer } = &mut self.state else {
            unreachable!("got window event while suspended");
        };

        if surface.window().id() != window_id {
            return;
        }

        match event {
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    // Resize surface
                    surface.resize(width, height).unwrap();
                    renderer.resize(width.get() as usize, height.get() as usize);
                }
            }
            WindowEvent::RedrawRequested => {
                let start = std::time::Instant::now();

                // Get the next buffer.
                let mut buffer = surface.next_buffer().unwrap();

                // Render into the buffer.
                renderer.render(&mut buffer);

                // Send the buffer to the compositor.
                buffer.present().unwrap();
                println!("fps: {:.2}", 1.0 / start.elapsed().as_secs_f32());

                surface.window().request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }
}

struct Renderer {
    width: usize,
    height: usize,
    buffer: Vec<MaybeUninit<Pixel>>,
    depth_buffer: Vec<f32>,
    pipeline: Pipeline<Vertex, Fragment, TriangleRasterizer>,
}

impl Renderer {
    fn new(width: usize, height: usize) -> Self {
        let mut buffer = Vec::with_capacity(width * height);
        unsafe {
            buffer.set_len(width * height);
        }

        let depth_buffer = vec![1.0; width * height];

        let diffuse_bytes = include_bytes!("../assets/HuaGuDragon.jpg");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgb32f();

        let pipeline = Pipeline::new(
            Vertex,
            Fragment {
                buffer: diffuse_rgba,
            },
            TriangleRasterizer::new(width, height, mini_renderer::graphics::FrontFace::Ccw),
            PrimitiveAssembler::new(
                mini_renderer::graphics::topology::PrimitiveTopology::TriangleList,
            ),
        );

        Self {
            width,
            height,
            buffer,
            depth_buffer,
            pipeline,
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        if width == self.width && height == self.height {
            return;
        }

        self.width = width;
        self.height = height;
        let mut buffer = Vec::with_capacity(width * height);
        unsafe {
            buffer.set_len(width * height);
        }

        self.buffer = buffer;
        self.depth_buffer.resize(width * height, 1.0);
        self.pipeline.rasterizer.resize(width, height);
    }

    fn render(&mut self, buffer: &mut Buffer) {
        self.resize(
            buffer.width().get() as usize,
            buffer.height().get() as usize,
        );

        let pixels = unsafe {
            std::mem::transmute::<&mut [MaybeUninit<Pixel>], &mut [Pixel]>(&mut self.buffer[..])
        };

        pixels.fill(Pixel::new_rgb(0, 0, 0));
        self.depth_buffer.fill(1.0);

        let vertexs = [
            VertexInput {
                vertex: (-0.5, 0.5, 0.0),
                varying: Some(ColorOutput {
                    tex_coord: (0.0, 0.0),
                    color: (1.0, 0.0, 0.0),
                }),
            },
            VertexInput {
                vertex: (0.5, 0.5, 0.0),
                varying: Some(ColorOutput {
                    tex_coord: (1.0, 0.0),
                    color: (0.0, 1.0, 0.0),
                }),
            },
            VertexInput {
                vertex: (-0.5, -0.5, 0.0),
                varying: Some(ColorOutput {
                    tex_coord: (0.0, 1.0),
                    color: (0.0, 0.0, 1.0),
                }),
            },
            VertexInput {
                vertex: (0.5, 0.5, 0.0),
                varying: Some(ColorOutput {
                    tex_coord: (1.0, 0.0),
                    color: (0.0, 1.0, 0.0),
                }),
            },
            VertexInput {
                vertex: (0.5, -0.5, 0.0),
                varying: Some(ColorOutput {
                    tex_coord: (1.0, 1.0),
                    color: (1.0, 1.0, 0.0),
                }),
            },
            VertexInput {
                vertex: (-0.5, -0.5, 0.0),
                varying: Some(ColorOutput {
                    tex_coord: (0.0, 1.0),
                    color: (0.0, 0.0, 1.0),
                }),
            },
        ];

        let pixels = unsafe {
            std::mem::transmute::<&mut [MaybeUninit<Pixel>], &mut [Pixel]>(&mut self.buffer[..])
        };

        self.pipeline
            .draw(&vertexs, &mut self.depth_buffer, pixels, self.width);

        buffer.pixels().copy_from_slice(pixels);
    }
}

struct Vertex;
struct Fragment {
    buffer: ImageBuffer<Rgb<f32>, Vec<f32>>,
}

impl VertexShader for Vertex {
    type Vertex = (f32, f32, f32);

    type Varying = ColorOutput;

    fn vs_main(
        &self,
        _index: usize,
        vertex: &mini_renderer::pipeline::shader::VertexInput<Self::Vertex, Self::Varying>,
    ) -> mini_renderer::pipeline::shader::VertexOutput<Self::Varying> {
        let VertexInput { vertex, varying } = vertex;
        VertexOutput {
            position: Vec4::new(vertex.0, vertex.1, vertex.2, 1.0),
            varying: varying.unwrap(),
        }
    }
}

impl FragmentShader for Fragment {
    type Varying = ColorOutput;
    type Output = Color;

    fn fs_main(&self, varying: &Self::Varying) -> Option<Color> {
        let (u, v) = varying.tex_coord;

        let x = (u * (self.buffer.width() - 1) as f32) as u32;
        let y = (v * (self.buffer.height() - 1) as f32) as u32;

        let x = x.min(self.buffer.width() - 1);
        let y = y.min(self.buffer.height() - 1);

        let pixel = self.buffer.get_pixel(x, y);

        Some(Color {
            r: ((pixel[0] * 0.7 + varying.color.0 * 0.3) * 255.0) as u8,
            g: ((pixel[1] * 0.7 + varying.color.1 * 0.3) * 255.0) as u8,
            b: ((pixel[2] * 0.7 + varying.color.2 * 0.3) * 255.0) as u8,
        })
    }
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl IntoColor for Color {
    type Output = Pixel;

    fn into_color(self) -> Self::Output {
        Pixel::new_rgb(self.r, self.g, self.b)
    }
}

#[derive(Debug, Clone, Copy)]
struct ColorOutput {
    tex_coord: (f32, f32),
    color: (f32, f32, f32),
}

impl Varying for ColorOutput {
    fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self {
        Self {
            tex_coord: Varying::interpolate(v0.tex_coord, v1.tex_coord, v2.tex_coord, w0, w1, w2),
            color: Varying::interpolate(v0.color, v1.color, v2.color, w0, w1, w2),
        }
    }
}
