use mini_renderer::graphics::primitive::PrimitiveState;
use mini_renderer::graphics::topology::PrimitiveTopology;
use mini_renderer::math::Vec4;
use mini_renderer::pipeline::shader::{FragmentShader, VertexOutput, VertexShader};
use mini_renderer::renderer::{Renderer, create_render_pipeline};

// Test: Vertex shader with transformation
struct TransformVertexShader;

impl VertexShader for TransformVertexShader {
    type Vertex = (f32, f32, f32);
    type Varying = (f32, f32, f32);
    type Uniform = f32;

    fn vs_main(
        &self,
        _index: usize,
        vertex: &Self::Vertex,
        scale: &Self::Uniform,
    ) -> VertexOutput<Self::Varying> {
        let (x, y, z) = vertex;
        VertexOutput {
            position: Vec4::new(x * scale, y * scale, *z, 1.0),
            varying: (*x, *y, *z),
        }
    }
}

// Fragment shader that uses varying
struct ColorFragmentShader;

impl FragmentShader for ColorFragmentShader {
    type Varying = (f32, f32, f32);
    type Output = u32;
    type Uniform = f32;

    fn fs_main(&self, varying: &Self::Varying, _uniform: &Self::Uniform) -> Option<u32> {
        let (r, g, b) = varying;
        let r_byte = ((r.clamp(0.0, 1.0) * 255.0) as u32) << 24;
        let g_byte = ((g.clamp(0.0, 1.0) * 255.0) as u32) << 16;
        let b_byte = ((b.clamp(0.0, 1.0) * 255.0) as u32) << 8;
        let a_byte = 255u32;
        Some(r_byte | g_byte | b_byte | a_byte)
    }
}

// Fragment shader that outputs constant color
struct ConstantColorFragmentShader {
    color: u32,
}

impl FragmentShader for ConstantColorFragmentShader {
    type Varying = f32;
    type Output = u32;
    type Uniform = ();

    fn fs_main(&self, _varying: &Self::Varying, _uniform: &Self::Uniform) -> Option<u32> {
        Some(self.color)
    }
}

// Vertex shader that doesn't modify position
struct PassthroughVertexShader;

impl VertexShader for PassthroughVertexShader {
    type Vertex = (f32, f32, f32);
    type Varying = f32;
    type Uniform = ();

    fn vs_main(
        &self,
        _index: usize,
        vertex: &Self::Vertex,
        _uniform: &Self::Uniform,
    ) -> VertexOutput<Self::Varying> {
        VertexOutput {
            position: Vec4::new(vertex.0, vertex.1, vertex.2, 1.0),
            varying: 1.0,
        }
    }
}

// Vertex shader with color interpolation
struct ColorVertexShader;

impl VertexShader for ColorVertexShader {
    type Vertex = ((f32, f32, f32), (f32, f32, f32));
    type Varying = (f32, f32, f32);
    type Uniform = f32;

    fn vs_main(
        &self,
        _index: usize,
        vertex: &Self::Vertex,
        _uniform: &Self::Uniform,
    ) -> VertexOutput<Self::Varying> {
        let (pos, color) = vertex;
        VertexOutput {
            position: Vec4::new(pos.0, pos.1, pos.2, 1.0),
            varying: *color,
        }
    }
}

#[test]
fn test_vertex_shader_with_scale_uniform() {
    let mut pipeline = create_render_pipeline(
        TransformVertexShader,
        ColorFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let vertices = [(-0.5, -0.5, 0.0), (0.5, -0.5, 0.0), (0.0, 0.5, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    // Scale uniform of 0.5
    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &0.5);

    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(
        non_zero_pixels > 0,
        "Scaled triangle should render at least one pixel"
    );
}

#[test]
fn test_vertex_shader_different_scales() {
    let mut pipeline1 = create_render_pipeline(
        TransformVertexShader,
        ColorFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let vertices = [(-0.5, -0.5, 0.0), (0.5, -0.5, 0.0), (0.0, 0.5, 0.0)];

    let mut framebuffer1 = vec![0u32; 100 * 100];
    let mut depth_buffer1 = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    // Render with scale 0.5
    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline1)
        .with_depth(&mut depth_buffer1)
        .draw(&vertices, &mut framebuffer1, &0.5);

    let pixels_scale_05 = framebuffer1.iter().filter(|&&p| p != 0).count();

    // Render with scale 1.0
    let mut pipeline2 = create_render_pipeline(
        TransformVertexShader,
        ColorFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let mut framebuffer2 = vec![0u32; 100 * 100];
    let mut depth_buffer2 = vec![1.0; 100 * 100];

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline2)
        .with_depth(&mut depth_buffer2)
        .draw(&vertices, &mut framebuffer2, &1.0);

    let pixels_scale_10 = framebuffer2.iter().filter(|&&p| p != 0).count();

    // Larger scale should produce more pixels
    assert!(
        pixels_scale_10 > pixels_scale_05,
        "Scale 1.0 should produce more pixels ({}) than scale 0.5 ({})",
        pixels_scale_10,
        pixels_scale_05
    );
}

#[test]
fn test_varying_interpolation_single_color() {
    let mut pipeline = create_render_pipeline(
        ColorVertexShader,
        ColorFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    // Triangle with red color at all vertices
    let vertices = [
        ((-0.5, -0.5, 0.0), (1.0, 0.0, 0.0)),
        ((0.5, -0.5, 0.0), (1.0, 0.0, 0.0)),
        ((0.0, 0.5, 0.0), (1.0, 0.0, 0.0)),
    ];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &0.0f32);

    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(
        non_zero_pixels > 0,
        "Triangle with uniform color should render"
    );

    // All pixels should be red (or close to it)
    for &pixel in framebuffer.iter() {
        if pixel != 0 {
            let r = ((pixel >> 24) & 0xFF) as f32 / 255.0;
            let g = ((pixel >> 16) & 0xFF) as f32 / 255.0;
            let b = ((pixel >> 8) & 0xFF) as f32 / 255.0;

            // Check that red is dominant
            assert!(r > g, "Red should be greater than green");
            assert!(r > b, "Red should be greater than blue");
        }
    }
}

#[test]
fn test_varying_interpolation_gradient() {
    let mut pipeline = create_render_pipeline(
        ColorVertexShader,
        ColorFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    // Triangle with gradient: red at top, green at bottom-left, blue at bottom-right
    let vertices = [
        ((-0.5, -0.5, 0.0), (0.0, 1.0, 0.0)), // green
        ((0.5, -0.5, 0.0), (0.0, 0.0, 1.0)),  // blue
        ((0.0, 0.5, 0.0), (1.0, 0.0, 0.0)),   // red
    ];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &0.0f32);

    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(
        non_zero_pixels > 0,
        "Triangle with color gradient should render"
    );
}

#[test]
fn test_passthrough_vertex_shader() {
    let mut pipeline = create_render_pipeline(
        PassthroughVertexShader,
        ConstantColorFragmentShader { color: 0xFFFFFFFF },
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let vertices = [(-0.25, -0.25, 0.0), (0.25, -0.25, 0.0), (0.0, 0.25, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &());

    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(non_zero_pixels > 0, "Passthrough shader should render");
}

#[test]
fn test_multiple_shader_combinations() {
    // Test 1: TransformVertexShader with ColorFragmentShader
    let mut pipeline1 = create_render_pipeline(
        TransformVertexShader,
        ColorFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let vertices = [(-0.5, -0.5, 0.0), (0.5, -0.5, 0.0), (0.0, 0.5, 0.0)];

    let mut framebuffer1 = vec![0u32; 100 * 100];
    let mut depth_buffer1 = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline1)
        .with_depth(&mut depth_buffer1)
        .draw(&vertices, &mut framebuffer1, &0.8);

    let pixels1 = framebuffer1.iter().filter(|&&p| p != 0).count();

    // Test 2: PassthroughVertexShader with ConstantColorFragmentShader
    let mut pipeline2 = create_render_pipeline(
        PassthroughVertexShader,
        ConstantColorFragmentShader { color: 0xFF0000FF },
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let mut framebuffer2 = vec![0u32; 100 * 100];
    let mut depth_buffer2 = vec![1.0; 100 * 100];

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline2)
        .with_depth(&mut depth_buffer2)
        .draw(&vertices, &mut framebuffer2, &());

    let pixels2 = framebuffer2.iter().filter(|&&p| p != 0).count();

    // Both should render some pixels
    assert!(pixels1 > 0, "First shader combination should render");
    assert!(pixels2 > 0, "Second shader combination should render");
}

#[test]
fn test_zero_scale_transform() {
    let mut pipeline = create_render_pipeline(
        TransformVertexShader,
        ColorFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let vertices = [(-0.5, -0.5, 0.0), (0.5, -0.5, 0.0), (0.0, 0.5, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    // Scale of 0 should collapse the triangle to the origin
    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &0.0);

    // May or may not render, but shouldn't crash
    let _non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
}

#[test]
fn test_very_small_triangle() {
    let mut pipeline = create_render_pipeline(
        PassthroughVertexShader,
        ConstantColorFragmentShader { color: 0xFF0000FF },
        PrimitiveState {
            topology: PrimitiveTopology::triangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    // Very small triangle
    let vertices = [(0.0, 0.0, 0.0), (0.01, 0.0, 0.0), (0.0, 0.01, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &());

    // Should not crash, may render some pixels
    let _non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
}
