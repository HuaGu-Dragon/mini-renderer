use mini_renderer::graphics::primitive::PrimitiveState;
use mini_renderer::graphics::topology::PrimitiveTopology;
use mini_renderer::math::Vec4;
use mini_renderer::pipeline::shader::{FragmentShader, VertexOutput, VertexShader};
use mini_renderer::renderer::{Renderer, create_render_pipeline};

// Simple test vertex shader
struct TestVertexShader;

impl VertexShader for TestVertexShader {
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

// Simple test fragment shader that outputs red
struct TestFragmentShader;

impl FragmentShader for TestFragmentShader {
    type Varying = f32;
    type Output = u32;
    type Uniform = ();

    fn fs_main(&self, _varying: &Self::Varying, _uniform: &Self::Uniform) -> Option<u32> {
        Some(0xFF0000FF) // Red color (ABGR format)
    }
}

// Fragment shader that discards some fragments
struct DiscardFragmentShader;

impl FragmentShader for DiscardFragmentShader {
    type Varying = f32;
    type Output = u32;
    type Uniform = ();

    fn fs_main(&self, _varying: &Self::Varying, _uniform: &Self::Uniform) -> Option<u32> {
        None // Always discard
    }
}

#[test]
fn test_renderer_creation() {
    let renderer = Renderer::new(800, 600);
    assert_eq!(renderer.width(), 800);
    assert_eq!(renderer.height(), 600);
}

#[test]
fn test_simple_triangle_renders() {
    let mut pipeline = create_render_pipeline(
        TestVertexShader,
        TestFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::trangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    // Create a simple triangle in the center of the viewport
    let vertices = [(-0.5, -0.5, 0.0), (0.5, -0.5, 0.0), (0.0, 0.5, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &());

    // Check that at least some pixels were written
    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(
        non_zero_pixels > 0,
        "Triangle should render at least one pixel, got {}",
        non_zero_pixels
    );
}

#[test]
fn test_triangle_without_depth() {
    let mut pipeline = create_render_pipeline(
        TestVertexShader,
        TestFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::trangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let vertices = [(-0.5, -0.5, 0.0), (0.5, -0.5, 0.0), (0.0, 0.5, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .draw(&vertices, &mut framebuffer, &());

    // Check that at least some pixels were written
    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(
        non_zero_pixels > 0,
        "Triangle should render at least one pixel"
    );
}

#[test]
fn test_discard_fragments() {
    let mut pipeline = create_render_pipeline(
        TestVertexShader,
        DiscardFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::trangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let vertices = [(-0.5, -0.5, 0.0), (0.5, -0.5, 0.0), (0.0, 0.5, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &());

    // With discard shader, no pixels should be written
    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert_eq!(
        non_zero_pixels, 0,
        "Discard shader should result in no pixels written"
    );
}

#[test]
fn test_multiple_triangles() {
    let mut pipeline = create_render_pipeline(
        TestVertexShader,
        TestFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::trangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    // Two triangles
    let vertices = [
        (-0.7, -0.5, 0.0),
        (-0.1, -0.5, 0.0),
        (-0.4, 0.2, 0.0),
        (0.1, -0.5, 0.0),
        (0.7, -0.5, 0.0),
        (0.4, 0.2, 0.0),
    ];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&vertices, &mut framebuffer, &());

    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(
        non_zero_pixels > 0,
        "Multiple triangles should render pixels"
    );
}

#[test]
fn test_back_face_culling() {
    let mut pipeline = create_render_pipeline(
        TestVertexShader,
        TestFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::trangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: Some(mini_renderer::graphics::Face::Back),
        },
    );

    // Triangle vertices arranged clockwise in NDC space (which becomes CCW in screen space due to Y flip)
    // This makes it a front-facing triangle after screen-space transform
    let front_facing = [(-0.5, -0.5, 0.0), (0.0, 0.5, 0.0), (0.5, -0.5, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&front_facing, &mut framebuffer, &());

    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(
        non_zero_pixels > 0,
        "Front-facing triangle should render with back-face culling"
    );
}

#[test]
fn test_front_face_culling() {
    let mut pipeline = create_render_pipeline(
        TestVertexShader,
        TestFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::trangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: Some(mini_renderer::graphics::Face::Front),
        },
    );

    // Triangle vertices arranged clockwise in NDC space (becomes CCW in screen space due to Y flip)
    // This makes it a front-facing triangle that should be culled
    let front_facing = [(-0.5, -0.5, 0.0), (0.0, 0.5, 0.0), (0.5, -0.5, 0.0)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&front_facing, &mut framebuffer, &());

    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert_eq!(
        non_zero_pixels, 0,
        "Front-facing triangle should be culled with front-face culling"
    );
}

#[test]
fn test_depth_testing_closer_wins() {
    let mut pipeline = create_render_pipeline(
        TestVertexShader,
        TestFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::trangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    // First triangle at z=0.5 (closer)
    let triangle1 = [(-0.5, -0.5, 0.5), (0.5, -0.5, 0.5), (0.0, 0.5, 0.5)];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&triangle1, &mut framebuffer, &());

    let pixels_after_first = framebuffer.iter().filter(|&&p| p != 0).count();

    // Second triangle at z=0.8 (farther, should not overwrite)
    let triangle2 = [(-0.3, -0.3, 0.8), (0.3, -0.3, 0.8), (0.0, 0.3, 0.8)];

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw(&triangle2, &mut framebuffer, &());

    let pixels_after_second = framebuffer.iter().filter(|&&p| p != 0).count();

    // Pixels should not increase significantly (only edges might be filled)
    assert_eq!(
        pixels_after_first, pixels_after_second,
        "Farther triangle should not overwrite closer pixels"
    );
}

#[test]
fn test_indexed_draw() {
    let mut pipeline = create_render_pipeline(
        TestVertexShader,
        TestFragmentShader,
        PrimitiveState {
            topology: PrimitiveTopology::trangle_list(),
            front_face: mini_renderer::graphics::FrontFace::Ccw,
            cull_mode: None,
        },
    );

    let vertices = [(-0.5, -0.5, 0.0), (0.5, -0.5, 0.0), (0.0, 0.5, 0.0)];

    let indices = vec![0, 1, 2];

    let mut framebuffer = vec![0u32; 100 * 100];
    let mut depth_buffer = vec![1.0; 100 * 100];

    let renderer = Renderer::new(100, 100);

    renderer
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_depth(&mut depth_buffer)
        .draw_indexed(&vertices, indices.into_iter(), &mut framebuffer, &());

    let non_zero_pixels = framebuffer.iter().filter(|&&p| p != 0).count();
    assert!(non_zero_pixels > 0, "Indexed draw should render pixels");
}
