use std::f32;
use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::path::Path;
use std::rc::Rc;

use image::DynamicImage;
use mini_renderer::graphics::color::IntoColor;
use mini_renderer::graphics::primitive::PrimitiveAssembler;
use mini_renderer::graphics::rasterizer::TriangleRasterizer;
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
        let AppState::Suspended { window } = &mut self.state else {
            unreachable!("got resumed event while not suspended");
        };
        let mut surface =
            Surface::new(&self.context, window.clone()).expect("failed creating surface");

        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
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
                    surface.resize(width, height).unwrap();
                    renderer.resize(width.get() as usize, height.get() as usize);
                }
            }
            WindowEvent::RedrawRequested => {
                let mut buffer = surface.next_buffer().unwrap();

                controller.update_camera(&mut renderer.camera);

                renderer.render(&mut buffer);

                buffer.present().unwrap();

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
    pipeline: Pipeline<Vertex, Fragment, TriangleRasterizer>,
    camera: Camera,
    model_vertices: Vec<VertexInput<(f32, f32, f32), ColorOutput>>,
    model_indices: Vec<usize>,
}

impl Renderer {
    fn new(width: usize, height: usize) -> Self {
        let mut buffer = Vec::with_capacity(width * height);

        let (model_vertices, model_indices, textures) = load_model("assets/wuwa/aemeath.obj");

        let camera = Camera {
            eye: (0.0, 1.0, 5.0).into(),
            target: (0.0, 1.0, 0.0).into(),
            up: Vec3::Y,
            aspect: width as f32 / height as f32,
            fovy: 45.0,
            znear: 1.0,
            zfar: 100.0,
        };

        unsafe {
            buffer.set_len(width * height);
        }

        let depth_buffer = vec![1.0; width * height];

        let pipeline = Pipeline::new(
            Vertex { camera },
            Fragment { textures },
            TriangleRasterizer::new(width, height),
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
            camera,
            model_vertices,
            model_indices,
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

        self.pipeline.vertex_shader.camera.aspect = width as f32 / height as f32;
        self.camera.aspect = width as f32 / height as f32;
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

        self.pipeline.vertex_shader.camera = self.camera;

        let pixels = unsafe {
            std::mem::transmute::<&mut [MaybeUninit<Pixel>], &mut [Pixel]>(&mut self.buffer[..])
        };

        self.pipeline.draw_indexed(
            &self.model_vertices,
            self.model_indices.iter().copied(),
            &mut self.depth_buffer,
            pixels,
            self.width,
        );

        buffer.pixels().copy_from_slice(pixels);
    }
}

struct Vertex {
    camera: Camera,
}

struct Fragment {
    textures: Vec<Texture>,
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
        let camera = &self.camera;
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
    type Output = Color;

    fn fs_main(&self, varying: &Self::Varying) -> Option<Color> {
        if varying.texture_id < self.textures.len() {
            let texture = &self.textures[varying.texture_id];
            if texture.has_texture {
                let sampled_color = texture.sample(varying.tex_coord.0, varying.tex_coord.1);

                let alpha_threshold = 0.1;
                if (sampled_color.3 as f32 / 255.0) < alpha_threshold {
                    return None;
                }

                Some(Color {
                    r: sampled_color.0,
                    g: sampled_color.1,
                    b: sampled_color.2,
                })
            } else {
                Some(Color {
                    r: (varying.color.0 * 255.0) as u8,
                    g: (varying.color.1 * 255.0) as u8,
                    b: (varying.color.2 * 255.0) as u8,
                })
            }
        } else {
            Some(Color {
                r: (varying.color.0 * 255.0) as u8,
                g: (varying.color.1 * 255.0) as u8,
                b: (varying.color.2 * 255.0) as u8,
            })
        }
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
    normal: (f32, f32, f32),
    color: (f32, f32, f32),
    texture_id: usize,
}

impl Varying for ColorOutput {
    fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self {
        Self {
            tex_coord: Varying::interpolate(v0.tex_coord, v1.tex_coord, v2.tex_coord, w0, w1, w2),
            normal: Varying::interpolate(v0.normal, v1.normal, v2.normal, w0, w1, w2),
            color: Varying::interpolate(v0.color, v1.color, v2.color, w0, w1, w2),
            texture_id: v0.texture_id,
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
    is_q_pressed: bool,
    is_e_pressed: bool,
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
            is_q_pressed: false,
            is_e_pressed: false,
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
            PhysicalKey::Code(KeyCode::KeyQ) => {
                self.is_q_pressed = is_pressed;
            }
            PhysicalKey::Code(KeyCode::KeyE) => {
                self.is_e_pressed = is_pressed;
            }
            _ => {}
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward = forward.normalize();
        let right = forward.cross(camera.up).normalize();

        if self.is_forward_pressed {
            camera.target += forward * self.speed;
            camera.eye += forward * self.speed;
        }
        if self.is_backward_pressed {
            camera.target -= forward * self.speed;
            camera.eye -= forward * self.speed;
        }
        if self.is_right_pressed {
            camera.target += right * self.speed;
            camera.eye += right * self.speed;
        }
        if self.is_left_pressed {
            camera.target -= right * self.speed;
            camera.eye -= right * self.speed;
        }
        if self.is_up_pressed {
            camera.target += camera.up * self.speed;
            camera.eye += camera.up * self.speed;
        }
        if self.is_down_pressed {
            camera.target -= camera.up * self.speed;
            camera.eye -= camera.up * self.speed;
        }
        if self.is_q_pressed {
            camera.eye -= right * self.speed;
        }
        if self.is_e_pressed {
            camera.eye += right * self.speed;
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

#[derive(Clone)]
struct Texture {
    width: u32,
    height: u32,
    data: Vec<u8>,
    has_texture: bool,
}

impl Texture {
    fn new(image: DynamicImage) -> Self {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        Self {
            width,
            height,
            data: rgba.into_raw(),
            has_texture: true,
        }
    }

    fn sample(&self, u: f32, v: f32) -> (u8, u8, u8, u8) {
        let u = if u.is_finite() {
            let u_wrapped = u - u.floor();
            u_wrapped.clamp(0.0, 1.0)
        } else {
            0.5
        };

        let v = if v.is_finite() {
            let v_wrapped = v - v.floor();
            v_wrapped.clamp(0.0, 1.0)
        } else {
            0.5
        };

        let x = ((u * self.width as f32).floor() as u32).min(self.width - 1);
        let y = ((v * self.height as f32).floor() as u32).min(self.height - 1);

        let index = ((y * self.width + x) * 4) as usize;

        if index + 3 < self.data.len() {
            (
                self.data[index],
                self.data[index + 1],
                self.data[index + 2],
                self.data[index + 3],
            )
        } else {
            (255, 0, 255, 255)
        }
    }
}

type Vertexs = Vec<VertexInput<(f32, f32, f32), ColorOutput>>;

fn load_model(path: &str) -> (Vertexs, Vec<usize>, Vec<Texture>) {
    let obj = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS);

    let (models, materials) = obj.expect("failed to load model");
    let materials = materials.expect("Failed to load MTL file");

    let total_vertices: usize = models.iter().map(|m| m.mesh.positions.len() / 3).sum();
    let total_indices: usize = models.iter().map(|m| m.mesh.indices.len()).sum();

    let mut vertices = Vec::with_capacity(total_vertices);
    let mut indices = Vec::with_capacity(total_indices);
    let mut textures = Vec::with_capacity(materials.len().max(1));

    let obj_dir = Path::new(path).parent().unwrap_or(Path::new(""));

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut min_z = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut max_z = f32::MIN;

    for model in &models {
        let mesh = &model.mesh;
        assert!(mesh.positions.len() % 3 == 0);
        for i in (0..mesh.positions.len()).step_by(3) {
            let x = mesh.positions[i];
            let y = mesh.positions[i + 1];
            let z = mesh.positions[i + 2];

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            min_z = min_z.min(z);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            max_z = max_z.max(z);
        }
    }

    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;
    let center_z = (min_z + max_z) / 2.0;

    let scale_x = max_x - min_x;
    let scale_y = max_y - min_y;
    let scale_z = max_z - min_z;
    let max_scale = scale_x.max(scale_y).max(scale_z);

    let scale_factor = if max_scale > 0.0 {
        2.0 / max_scale
    } else {
        1.0
    };

    for (mat_idx, material) in materials.iter().enumerate() {
        println!("Material {}: {:?}", mat_idx, material.name);
        if let Some(diffuse_texture) = &material.diffuse_texture {
            let texture_path = obj_dir.join(diffuse_texture);
            println!("  Diffuse texture: {:?}", diffuse_texture);

            match image::open(&texture_path) {
                Ok(img) => {
                    println!("  ✓ Loaded successfully: {}x{}", img.width(), img.height());
                    textures.push(Texture::new(img));
                }
                Err(e) => {
                    println!("  ✗ Failed to load: {:?}", e);
                    textures.push(Texture {
                        width: 1,
                        height: 1,
                        data: vec![255, 255, 255, 255],
                        has_texture: false,
                    });
                }
            }
        } else {
            println!("  No diffuse texture, using white");
            textures.push(Texture {
                width: 1,
                height: 1,
                data: vec![255, 255, 255, 255],
                has_texture: false,
            });
        }
    }

    if textures.is_empty() {
        textures.push(Texture {
            width: 1,
            height: 1,
            data: vec![255, 255, 255, 255],
            has_texture: false,
        });
    }

    for model in models {
        let mesh = &model.mesh;
        let vertex_offset = vertices.len();

        assert!(mesh.positions.len() % 3 == 0);
        for vertex_idx in 0..mesh.positions.len() / 3 {
            let i = vertex_idx * 3;
            let position = (
                (mesh.positions[i] - center_x) * scale_factor,
                (mesh.positions[i + 1] - center_y) * scale_factor,
                (mesh.positions[i + 2] - center_z) * scale_factor,
            );

            let material_id = mesh.material_id.unwrap_or(0);

            let tex_coord = if !mesh.texcoords.is_empty() {
                let tex_idx = vertex_idx * 2;
                (mesh.texcoords[tex_idx], 1.0 - mesh.texcoords[tex_idx + 1])
            } else {
                (0.0, 0.0)
            };

            let normal = if !mesh.normals.is_empty() {
                let norm_idx = vertex_idx * 3;
                (
                    mesh.normals[norm_idx],
                    mesh.normals[norm_idx + 1],
                    mesh.normals[norm_idx + 2],
                )
            } else {
                (0.0, 1.0, 0.0)
            };

            let texture_id = if material_id < textures.len() {
                material_id
            } else {
                0
            };

            let color = if material_id < materials.len() {
                let mat = &materials[material_id];
                if let Some(diffuse) = mat.diffuse {
                    (diffuse[0], diffuse[1], diffuse[2])
                } else {
                    (1.0, 1.0, 1.0)
                }
            } else {
                (1.0, 1.0, 1.0)
            };

            vertices.push(VertexInput {
                vertex: position,
                varying: Some(ColorOutput {
                    tex_coord,
                    normal,
                    color,
                    texture_id,
                }),
            });
        }

        assert!(mesh.indices.len() % 3 == 0);

        for tri_idx in (0..mesh.indices.len()).step_by(3) {
            if tri_idx + 2 >= mesh.indices.len() {
                break;
            }
            indices.push(vertex_offset + mesh.indices[tri_idx] as usize);
            indices.push(vertex_offset + mesh.indices[tri_idx + 2] as usize);
            indices.push(vertex_offset + mesh.indices[tri_idx + 1] as usize);
            indices.push(vertex_offset + mesh.indices[tri_idx + 2] as usize);
        }
    }

    (vertices, indices, textures)
}
