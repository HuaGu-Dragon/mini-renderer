use mini_renderer::{
    graphics::{primitive::PrimitiveState, topology::PrimitiveTopology},
    math::Vec4,
    pipeline::shader::{FragmentShader, VertexOutput, VertexShader},
    renderer::{Renderer, create_render_pipeline},
};

struct Passthrough;

impl VertexShader for Passthrough {
    type Vertex = Vec4;
    type Varying = ();
    type Uniform = ();

    fn vs_main(&self, _index: usize, vertex: &Vec4, _uniform: &()) -> VertexOutput<()> {
        VertexOutput {
            position: *vertex,
            varying: (),
        }
    }
}

struct White;

impl FragmentShader for White {
    type Varying = ();
    type Output = f32;
    type Uniform = ();

    fn fs_main(&self, _varying: &(), _uniform: &()) -> Option<f32> {
        Some(1.0)
    }
}

fn render(vertices: &[Vec4], width: usize, height: usize) -> Vec<f32> {
    let mut pipeline = create_render_pipeline(
        Passthrough,
        White,
        PrimitiveState::new(PrimitiveTopology::triangle_list()),
    );
    let mut framebuffer = vec![0.0; width * height];
    Renderer::new(width, height)
        .begin_render_pass()
        .set_pipeline(&mut pipeline)
        .with_blend(|source: f32, destination: f32| source * 0.5 + destination * 0.5)
        .draw(vertices, &mut framebuffer, &());
    framebuffer
}

fn screen_vertex(x: f32, y: f32, width: usize, height: usize) -> Vec4 {
    Vec4::new(
        2.0 * x / width as f32 - 1.0,
        1.0 - 2.0 * y / height as f32,
        0.0,
        1.0,
    )
}

#[test]
fn clipped_triangle_blends_generated_diagonal_once() {
    let original = [
        Vec4::new(-2.0, -1.0, 0.0, 1.0),
        Vec4::new(-0.75, -1.0, 0.0, 1.0),
        Vec4::new(0.0, -0.5, 0.0, 1.0),
    ];

    for reversed in [false, true] {
        for start in 0..3 {
            let mut vertices = core::array::from_fn::<_, 3, _>(|i| original[(start + i) % 3]);
            if reversed {
                vertices.swap(1, 2);
            }
            let framebuffer = render(&vertices, 8, 8);
            assert_eq!(
                framebuffer[56], 0.5,
                "generated diagonal, start={start}, reversed={reversed}"
            );
            assert!(
                framebuffer
                    .iter()
                    .all(|&pixel| pixel == 0.0 || pixel == 0.5)
            );
        }
    }
}

#[test]
fn adjacent_triangles_cover_half_open_quad_once() {
    let corners = [
        screen_vertex(0.5, 0.5, 8, 8),
        screen_vertex(6.5, 0.5, 8, 8),
        screen_vertex(6.5, 6.5, 8, 8),
        screen_vertex(0.5, 6.5, 8, 8),
    ];

    for indices in [[0, 1, 2, 0, 2, 3], [0, 1, 3, 1, 2, 3]] {
        for reversed in [false, true] {
            let mut vertices = indices.map(|index| corners[index]);
            if reversed {
                for triangle in vertices.as_chunks_mut::<3>().0 {
                    triangle.swap(1, 2);
                }
            }
            let framebuffer = render(&vertices, 8, 8);
            for y in 0..8 {
                for x in 0..8 {
                    let expected = if x < 6 && y < 6 { 0.5 } else { 0.0 };
                    assert_eq!(
                        framebuffer[y * 8 + x],
                        expected,
                        "pixel ({x},{y}), reversed={reversed}"
                    );
                }
            }
        }
    }
}

#[test]
fn horizontal_and_vertical_shared_edges_blend_once() {
    let center = screen_vertex(3.5, 3.5, 8, 8);
    let boundary = [
        (0.5, 0.5),
        (3.5, 0.5),
        (6.5, 0.5),
        (6.5, 3.5),
        (6.5, 6.5),
        (3.5, 6.5),
        (0.5, 6.5),
        (0.5, 3.5),
    ];

    for reversed in [false, true] {
        let mut vertices = Vec::new();
        for i in 0..boundary.len() {
            let (x0, y0) = boundary[i];
            let (x1, y1) = boundary[(i + 1) % boundary.len()];
            let mut triangle = [
                center,
                screen_vertex(x0, y0, 8, 8),
                screen_vertex(x1, y1, 8, 8),
            ];
            if reversed {
                triangle.swap(1, 2);
            }
            vertices.extend(triangle);
        }
        let framebuffer = render(&vertices, 8, 8);
        for y in 0..8 {
            for x in 0..8 {
                let expected = if x < 6 && y < 6 { 0.5 } else { 0.0 };
                assert_eq!(
                    framebuffer[y * 8 + x],
                    expected,
                    "pixel ({x},{y}), reversed={reversed}"
                );
            }
        }
    }
}

#[cfg(feature = "rayon")]
#[test]
fn binned_triangles_blend_shared_edges_once() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        let mut vertices = Vec::new();
        for row in 0..16 {
            for column in 0..16 {
                let x = column as f32 * 8.0 + 1.5;
                let y = row as f32 * 8.0 + 1.5;
                let corners = [
                    screen_vertex(x, y, 128, 256),
                    screen_vertex(x + 6.0, y, 128, 256),
                    screen_vertex(x + 6.0, y + 6.0, 128, 256),
                    screen_vertex(x, y + 6.0, 128, 256),
                ];
                vertices.extend([
                    corners[0], corners[1], corners[2], corners[0], corners[2], corners[3],
                ]);
            }
        }
        let framebuffer = render(&vertices, 128, 256);
        for y in 0..256 {
            for x in 0..128 {
                let covered = y < 128 && (1..7).contains(&(x % 8)) && (1..7).contains(&(y % 8));
                let expected = if covered { 0.5 } else { 0.0 };
                assert_eq!(framebuffer[y * 128 + x], expected, "binned pixel ({x},{y})");
            }
        }
    });
}
