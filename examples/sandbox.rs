use std::f32;
use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::rc::Rc;

use mini_renderer::graphics::primitive::PrimitiveState;
use mini_renderer::graphics::topology::PrimitiveTopology;
use mini_renderer::math::Vec4;
use mini_renderer::pipeline::shader::{FragmentShader, VertexInput, VertexOutput, VertexShader};
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

#[derive(Debug)]
struct App {
    context: Context<OwnedDisplayHandle>,
    state: AppState,
}

#[derive(Debug)]
enum AppState {
    Initial,
    Suspended {
        window: Rc<Window>,
    },
    Running {
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
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

        self.state = AppState::Running { surface };
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Drop the surface.
        let AppState::Running { surface } = &mut self.state else {
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
        let AppState::Running { surface } = &mut self.state else {
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
                }
            }
            WindowEvent::RedrawRequested => {
                // Get the next buffer.
                let mut buffer = surface.next_buffer().unwrap();

                // Render into the buffer.
                let mut renderer = Renderer::new(
                    buffer.width().get() as usize,
                    buffer.height().get() as usize,
                );

                renderer.render(&mut buffer, |renderer| {
                    renderer.draw_line(0, 0, 100, 100, [255, 0, 0, 255]);
                });

                // Send the buffer to the compositor.
                buffer.present().unwrap();
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
}

impl Renderer {
    fn new(width: usize, height: usize) -> Self {
        let mut buffer = Vec::with_capacity(width * height);
        unsafe {
            buffer.set_len(width * height);
        }
        Self {
            width,
            height,
            buffer,
        }
    }

    fn draw_point(&mut self, x: i32, y: i32, color: [u8; 4]) {
        assert!(x >= 0 && x < self.width as i32);
        assert!(y >= 0 && y < self.height as i32);

        self.buffer[x as usize + y as usize * self.width]
            .write(Pixel::new_rgb(color[0], color[1], color[2]));
    }

    fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
        let dx = x1 - x0;
        let dy = y1 - y0;

        let mut e = -dx;
        let step = 2 * dy;
        let desc = -2 * dx;

        let mut x = x0;
        let mut y = y0;

        while x != x1 {
            self.draw_point(x, y, color);

            e += step;
            if e >= 0 {
                y += 1;
                e += desc;
            }
            x += 1;
        }
    }

    fn render<F>(&mut self, buffer: &mut Buffer, _draw: F)
    where
        F: Fn(&mut Self),
    {
        // draw(self);

        // let pixels = unsafe {
        //     std::mem::transmute::<&mut [MaybeUninit<Pixel>], &mut [softbuffer::Pixel]>(
        //         &mut self.buffer[..],
        //     )
        // };
        println!("====start====");

        let mut pipeline = mini_renderer::renderer::create_render_pipeline(
            Vertex,
            Fragment,
            PrimitiveState {
                topology: PrimitiveTopology::trangle_list(),
                front_face: mini_renderer::graphics::FrontFace::Ccw,
                cull_mode: None,
            },
        );

        let vertexs = [
            VertexInput {
                vertex: (0.0, 0.5, 0.0),
                varying: Some((1.0, 0.0, 0.0)),
            },
            VertexInput {
                vertex: (0.5, -0.5, 0.0),
                varying: Some((0.0, 1.0, 0.0)),
            },
            VertexInput {
                vertex: (-0.5, -0.5, 0.0),
                varying: Some((0.0, 0.0, 1.0)),
            },
            VertexInput {
                vertex: (0.5, 0.75, -1.0),
                varying: Some((1.0, 1.0, 1.0)),
            },
            VertexInput {
                vertex: (0.3, -0.4, -0.25),
                varying: Some((1.0, 1.0, 1.0)),
            },
            VertexInput {
                vertex: (-0.75, -0.75, 1.0),
                varying: Some((1.0, 1.0, 1.0)),
            },
        ];

        let pixels = unsafe {
            std::mem::transmute::<&mut [MaybeUninit<Pixel>], &mut [Pixel]>(&mut self.buffer[..])
        };

        let mut depth_buffer = vec![1.0; self.width * self.height];

        pipeline.draw(
            &vertexs,
            &mut depth_buffer,
            pixels,
            self.width,
            self.height,
            &(),
        );

        buffer.pixels().swap_with_slice(pixels);

        println!("====end====");
    }
}

struct Vertex;
struct Fragment;

impl VertexShader for Vertex {
    type Vertex = (f32, f32, f32);
    type Varying = (f32, f32, f32);
    type Uniform = ();

    fn vs_main(
        &self,
        _index: usize,
        vertex: &mini_renderer::pipeline::shader::VertexInput<Self::Vertex, Self::Varying>,
        _uniform: &Self::Uniform,
    ) -> mini_renderer::pipeline::shader::VertexOutput<Self::Varying> {
        let VertexInput { vertex, varying } = vertex;
        VertexOutput {
            position: Vec4::new(vertex.0, vertex.1, vertex.2, 1.0),
            varying: varying.unwrap(),
        }
    }
}

impl FragmentShader for Fragment {
    type Varying = (f32, f32, f32);
    type Output = Pixel;
    type Uniform = ();

    fn fs_main(&self, varying: &Self::Varying, _uniform: &Self::Uniform) -> Option<Pixel> {
        Some(Pixel::new_rgb(
            (varying.0 * 255.0) as u8,
            (varying.1 * 255.0) as u8,
            (varying.2 * 255.0) as u8,
        ))
    }
}
