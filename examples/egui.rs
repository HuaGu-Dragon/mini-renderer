use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::rc::Rc;

use egui::TextureId;
use egui_demo_lib::DemoWindows;
use egui_winit::State;
use mini_renderer::color::ColorFormat;
use mini_renderer::graphics::primitive::PrimitiveState;
use mini_renderer::graphics::rasterizer::TriangleRasterizer;
use mini_renderer::graphics::topology::{PrimitiveTopology, TrangleList};
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
        renderer: Box<Renderer>,
    },
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::Init = cause {
            let window_attrs = WindowAttributes::default().with_title("egui integration");
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

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        let renderer = Box::new(Renderer::new(
            size.width as usize,
            size.height as usize,
            egui_ctx,
            egui_state,
        ));

        self.state = AppState::Running { surface, renderer };
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
                    surface.resize(width, height).unwrap();
                }
            }
            WindowEvent::RedrawRequested => {
                let window = surface.window().clone();
                let mut buffer = surface.next_buffer().unwrap();
                renderer.render(&mut buffer, &window);
                buffer.present().unwrap();
                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {
                // Pass other events to egui
                renderer.handle_window_event(surface.window(), &event);
            }
        }
    }
}

struct Renderer {
    render: mini_renderer::renderer::Renderer,
    buffer: Vec<MaybeUninit<Pixel>>,
    pipeline: Pipeline<TrangleList, TriangleRasterizer, Vertex, Fragment>,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    demo: DemoWindows,

    uniform: EguiUniform,
    cached_vertices: Vec<VertexInput<egui::epaint::Vertex, EguiVarying>>,
    cached_indices: Vec<usize>,
}

pub struct EguiTexture {
    width: usize,
    height: usize,
    pixels: Vec<egui::Color32>,
}

impl Renderer {
    fn new(width: usize, height: usize, context: egui::Context, egui_state: State) -> Self {
        let mut buffer = Vec::with_capacity(width * height);
        unsafe {
            buffer.set_len(width * height);
        }

        let pipeline = mini_renderer::renderer::create_render_pipeline(
            Vertex,
            Fragment,
            PrimitiveState {
                topology: PrimitiveTopology::trangle_list(),
                front_face: mini_renderer::graphics::FrontFace::Cw,
                cull_mode: None,
            },
        );

        let renderer = mini_renderer::renderer::Renderer::new(width, height);

        Self {
            render: renderer,
            buffer,
            pipeline,
            egui_ctx: context,
            egui_state,
            demo: DemoWindows::default(),
            uniform: EguiUniform {
                screen_size: (width as f32, height as f32),
                pixels_per_point: 1.0,
                textures: HashMap::new(),
                current_texture_id: TextureId::default(),
                current_clip_rect: egui::Rect::EVERYTHING,
            },
            cached_vertices: Vec::new(),
            cached_indices: Vec::new(),
        }
    }

    fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.egui_state.on_window_event(window, event);
    }

    fn update_textures(&mut self, textures_delta: egui::TexturesDelta) {
        for (texture_id, image_delta) in &textures_delta.set {
            let (pixels, width, height) = match &image_delta.image {
                egui::ImageData::Color(image) => (&image.pixels, image.width(), image.height()),
            };

            if let Some(pos) = image_delta.pos {
                if let Some(texture) = self.uniform.textures.get_mut(texture_id) {
                    let x_offset = pos[0];
                    let y_offset = pos[1];

                    for y in 0..height {
                        let src_start = y * width;
                        let src_end = src_start + width;
                        let dst_start = (y_offset + y) * texture.width + x_offset;
                        let dst_end = dst_start + width;

                        texture.pixels[dst_start..dst_end]
                            .copy_from_slice(&pixels[src_start..src_end]);
                    }
                }
            } else {
                let texture = EguiTexture {
                    width,
                    height,
                    pixels: pixels.clone(),
                };
                self.uniform.textures.insert(*texture_id, texture);
            }
        }

        for texture_id in &textures_delta.free {
            self.uniform.textures.remove(texture_id);
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        if width == self.render.width && height == self.render.height {
            return;
        }

        self.render.width = width;
        self.render.height = height;
        let mut buffer = Vec::with_capacity(width * height);
        unsafe {
            buffer.set_len(width * height);
        }
        self.buffer = buffer;
        self.uniform.screen_size = (width as f32, height as f32);
    }

    fn render(&mut self, buffer: &mut Buffer, window: &Window) {
        self.resize(
            buffer.width().get() as usize,
            buffer.height().get() as usize,
        );

        let pixels = unsafe {
            std::mem::transmute::<&mut [MaybeUninit<Pixel>], &mut [Pixel]>(&mut self.buffer[..])
        };

        pixels.fill(Pixel::new_rgb(20, 20, 20));

        let raw_input = self.egui_state.take_egui_input(window);
        self.egui_ctx.begin_pass(raw_input);
        self.demo.ui(&self.egui_ctx);
        let output = self.egui_ctx.end_pass();

        self.uniform.pixels_per_point = output.pixels_per_point;

        let clipped_primitives = self
            .egui_ctx
            .tessellate(output.shapes, output.pixels_per_point);

        self.update_textures(output.textures_delta);

        let render_pass = self.render.begin_render_pass();
        let mut bound_pipeline = render_pass.set_pipeline(&mut self.pipeline).with_blend();

        let start = std::time::Instant::now();
        for clipped_primitive in clipped_primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = clipped_primitive.primitive {
                self.cached_vertices.clear();
                self.cached_indices.clear();

                for &vertex in &mesh.vertices {
                    self.cached_vertices.push(VertexInput {
                        vertex,
                        varying: None,
                    });
                }

                for &index in &mesh.indices {
                    self.cached_indices.push(index as usize);
                }

                self.uniform.current_clip_rect = clipped_primitive.clip_rect;
                self.uniform.current_texture_id = mesh.texture_id;

                bound_pipeline.draw_indexed(
                    &self.cached_vertices,
                    self.cached_indices.iter().copied(),
                    pixels,
                    &self.uniform,
                );
            }
        }
        println!("fps: {}", 1.0 / start.elapsed().as_secs_f32());

        buffer.pixels().copy_from_slice(pixels);
    }
}

struct Vertex;
struct Fragment;

pub struct EguiUniform {
    pub screen_size: (f32, f32),
    pub pixels_per_point: f32,
    pub textures: HashMap<TextureId, EguiTexture>,
    pub current_texture_id: TextureId,
    pub current_clip_rect: egui::Rect,
}

impl VertexShader for Vertex {
    type Vertex = egui::epaint::Vertex;
    type Varying = EguiVarying;
    type Uniform = EguiUniform;

    fn vs_main(
        &self,
        _index: usize,
        vertex: &VertexInput<Self::Vertex, Self::Varying>,
        uniform: &Self::Uniform,
    ) -> VertexOutput<Self::Varying> {
        let v = &vertex.vertex;

        let logical_width = uniform.screen_size.0 / uniform.pixels_per_point;
        let logical_height = uniform.screen_size.1 / uniform.pixels_per_point;

        let ndc_x = (v.pos.x / logical_width) * 2.0 - 1.0;
        let ndc_y = 1.0 - (v.pos.y / logical_height) * 2.0;

        VertexOutput {
            position: Vec4::new(ndc_x, ndc_y, 1.0, 1.0),
            varying: EguiVarying {
                uv: (v.uv.x, v.uv.y),
                color: (
                    v.color.r() as f32 / 255.0,
                    v.color.g() as f32 / 255.0,
                    v.color.b() as f32 / 255.0,
                    v.color.a() as f32 / 255.0,
                ),
                screen_pos: (v.pos.x, v.pos.y),
            },
        }
    }
}

impl FragmentShader for Fragment {
    type Varying = EguiVarying;
    type Uniform = EguiUniform;
    type Output = EguiColor;

    fn fs_main(&self, varying: &Self::Varying, uniform: &Self::Uniform) -> Option<EguiColor> {
        let clip_rect = uniform.current_clip_rect;
        let screen_pos = varying.screen_pos;

        if screen_pos.0 < clip_rect.min.x
            || screen_pos.0 >= clip_rect.max.x
            || screen_pos.1 < clip_rect.min.y
            || screen_pos.1 >= clip_rect.max.y
        {
            return None;
        }

        let color = varying.color;
        let mut r = color.0;
        let mut g = color.1;
        let mut b = color.2;
        let mut a = color.3;

        let texture = uniform.textures.get(&uniform.current_texture_id)?;

        let uv = varying.uv;
        let tex_x = ((uv.0 * texture.width as f32).clamp(0.0, (texture.width - 1) as f32)) as usize;
        let tex_y =
            ((uv.1 * texture.height as f32).clamp(0.0, (texture.height - 1) as f32)) as usize;
        let pixel = texture.pixels.get(tex_x + tex_y * texture.width)?;

        r *= pixel.r() as f32 / 255.0;
        g *= pixel.g() as f32 / 255.0;
        b *= pixel.b() as f32 / 255.0;
        a *= pixel.a() as f32 / 255.0;

        Some(EguiColor { r, g, b, a })
    }

    fn blend(output: Self::Output, background: Self::Output) -> Self::Output {
        let alpha = output.a;
        EguiColor {
            r: output.r * alpha + background.r * (1.0 - alpha),
            g: output.g * alpha + background.g * (1.0 - alpha),
            b: output.b * alpha + background.b * (1.0 - alpha),
            a: output.a + background.a * (1.0 - alpha),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EguiColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorFormat for EguiColor {
    type Output = Pixel;

    fn to_output(self) -> Self::Output {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;
        Pixel::new_rgb(r, g, b)
    }

    fn from_output(output: Self::Output) -> Self {
        let r = output.r as f32 / 255.0;
        let g = output.g as f32 / 255.0;
        let b = output.b as f32 / 255.0;
        EguiColor { r, g, b, a: 1.0 }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EguiVarying {
    pub uv: (f32, f32),
    pub color: (f32, f32, f32, f32),
    pub screen_pos: (f32, f32),
}

impl Varying for EguiVarying {
    fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self {
        Self {
            uv: Varying::interpolate(v0.uv, v1.uv, v2.uv, w0, w1, w2),
            color: Varying::interpolate(v0.color, v1.color, v2.color, w0, w1, w2),
            screen_pos: Varying::interpolate(
                v0.screen_pos,
                v1.screen_pos,
                v2.screen_pos,
                w0,
                w1,
                w2,
            ),
        }
    }
}
