use std::f32;
use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::rc::Rc;

use image::{ImageBuffer, Rgb};
use mini_renderer::graphics::primitive::PrimitiveState;
use mini_renderer::graphics::rasterizer::TriangleRasterizer;
use mini_renderer::graphics::topology::{PrimitiveTopology, TrangleList};
use mini_renderer::math::{Vec3, Vec4};
use mini_renderer::pipeline::Pipeline;
use mini_renderer::pipeline::shader::{FragmentShader, VertexInput, VertexOutput, VertexShader};
use mini_renderer::pipeline::varying::Varying;
use softbuffer::{Buffer, Context, Pixel, Surface};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{KeyCode, PhysicalKey};
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
        renderer: Box<Renderer>,
        controller: CameraController,
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
        let renderer = Box::new(Renderer::new(size.width as usize, size.height as usize));
        let controller = CameraController::new(0.01);
        self.state = AppState::Running {
            surface,
            renderer,
            controller,
        };
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
        let AppState::Running {
            surface,
            renderer,
            controller,
        } = &mut self.state
        else {
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

                controller.update_camera(&mut renderer.camera);

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
            WindowEvent::KeyboardInput { event, .. } => controller.process_events(&event),
            _ => {}
        }
    }
}

struct Renderer {
    width: usize,
    height: usize,
    buffer: Vec<MaybeUninit<Pixel>>,
    depth_buffer: Vec<f32>,
    pipeline: Pipeline<TrangleList, TriangleRasterizer, Vertex, Fragment>,
    camera: Camera,
}

impl Renderer {
    fn new(width: usize, height: usize) -> Self {
        let mut buffer = Vec::with_capacity(width * height);
        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: Vec3::Y,
            aspect: width as f32 / height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        unsafe {
            buffer.set_len(width * height);
        }

        let depth_buffer = vec![1.0; width * height];

        let diffuse_bytes = include_bytes!("../assets/HuaGuDragon.jpg");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgb32f();

        let pipeline = mini_renderer::renderer::create_render_pipeline(
            Vertex,
            Fragment {
                buffer: diffuse_rgba,
            },
            PrimitiveState {
                topology: PrimitiveTopology::trangle_list(),
                front_face: mini_renderer::graphics::FrontFace::Ccw,
                cull_mode: None,
            },
        );

        Self {
            width,
            height,
            buffer,
            depth_buffer,
            pipeline,
            camera,
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

        self.camera.aspect = width as f32 / height as f32;
        self.buffer = buffer;
        self.depth_buffer.resize(width * height, 1.0);
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

        self.pipeline.draw(
            &vertexs,
            &mut self.depth_buffer,
            pixels,
            self.width,
            self.height,
            &self.camera,
        );

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
    type Uniform = Camera;

    fn vs_main(
        &self,
        _index: usize,
        vertex: &mini_renderer::pipeline::shader::VertexInput<Self::Vertex, Self::Varying>,
        uniform: &Camera,
    ) -> mini_renderer::pipeline::shader::VertexOutput<Self::Varying> {
        let VertexInput { vertex, varying } = vertex;
        let camera = uniform;
        let position =
            camera.build_view_projection_matrix() * Vec4::new(vertex.0, vertex.1, vertex.2, 1.0);
        VertexOutput {
            position,
            varying: varying.unwrap(),
        }
    }
}

impl FragmentShader for Fragment {
    type Varying = ColorOutput;
    type Output = Pixel;
    type Uniform = Camera;

    fn fs_main(&self, varying: &Self::Varying, _uniform: &Camera) -> Option<Pixel> {
        let (u, v) = varying.tex_coord;

        let x = (u * (self.buffer.width() - 1) as f32) as u32;
        let y = (v * (self.buffer.height() - 1) as f32) as u32;

        let x = x.min(self.buffer.width() - 1);
        let y = y.min(self.buffer.height() - 1);

        let pixel = self.buffer.get_pixel(x, y);

        Some(Pixel::new_rgb(
            ((pixel[0] * 0.7 + varying.color.0 * 0.3) * 255.0) as u8,
            ((pixel[1] * 0.7 + varying.color.1 * 0.3) * 255.0) as u8,
            ((pixel[2] * 0.7 + varying.color.2 * 0.3) * 255.0) as u8,
        ))
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

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
        }
    }

    fn process_events(&mut self, event: &KeyEvent) {
        let is_pressed = event.state == ElementState::Pressed;

        if event.physical_key == PhysicalKey::Code(KeyCode::Space) {
            self.is_up_pressed = is_pressed;
        }

        match event.physical_key {
            PhysicalKey::Code(KeyCode::ShiftLeft) => {
                self.is_down_pressed = is_pressed;
            }
            PhysicalKey::Code(KeyCode::KeyW) | PhysicalKey::Code(KeyCode::ArrowUp) => {
                self.is_forward_pressed = is_pressed;
            }
            PhysicalKey::Code(KeyCode::KeyA) | PhysicalKey::Code(KeyCode::ArrowLeft) => {
                self.is_left_pressed = is_pressed;
            }
            PhysicalKey::Code(KeyCode::KeyS) | PhysicalKey::Code(KeyCode::ArrowDown) => {
                self.is_backward_pressed = is_pressed;
            }
            PhysicalKey::Code(KeyCode::KeyD) | PhysicalKey::Code(KeyCode::ArrowRight) => {
                self.is_right_pressed = is_pressed;
            }
            _ => {}
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward = forward.normalize();
        let right = forward.cross(camera.up).normalize();

        if self.is_forward_pressed {
            camera.eye += forward * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward * self.speed;
        }
        if self.is_right_pressed {
            camera.eye += right * self.speed;
        }
        if self.is_left_pressed {
            camera.eye -= right * self.speed;
        }
        if self.is_up_pressed {
            camera.eye += camera.up * self.speed;
        }
        if self.is_down_pressed {
            camera.eye -= camera.up * self.speed;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Camera {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> glam::Mat4 {
        let view = glam::Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj =
            glam::Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);

        proj * view
    }
}
