use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use mini_renderer::graphics::primitive::PrimitiveState;
use mini_renderer::graphics::topology::PrimitiveTopology;
use mini_renderer::math::Vec4;
use mini_renderer::pipeline::shader::{FragmentShader, VertexOutput, VertexShader};
use mini_renderer::renderer::{Renderer, create_render_pipeline};

const WIDTH: usize = 512;
const HEIGHT: usize = 512;
const PIXEL_COUNT: usize = WIDTH * HEIGHT;

#[derive(Clone, Copy)]
struct Vertex {
    position: Vec4,
}

struct BenchVertexShader;

impl VertexShader for BenchVertexShader {
    type Vertex = Vertex;
    type Varying = ();
    type Uniform = ();

    fn vs_main(
        &self,
        _index: usize,
        vertex: &Self::Vertex,
        _uniform: &Self::Uniform,
    ) -> VertexOutput<Self::Varying> {
        VertexOutput {
            position: vertex.position,
            varying: (),
        }
    }
}

struct BenchFragmentShader;

impl FragmentShader for BenchFragmentShader {
    type Varying = ();
    type Output = u32;
    type Uniform = ();

    fn fs_main(&self, _varying: &Self::Varying, _uniform: &Self::Uniform) -> Option<Self::Output> {
        Some(0xff_80_40_20)
    }
}

fn triangle_grid(cells_per_axis: usize) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(cells_per_axis * cells_per_axis * 6);
    let step = 2.0 / cells_per_axis as f32;

    for y in 0..cells_per_axis {
        let y0 = -1.0 + y as f32 * step;
        let y1 = y0 + step;

        for x in 0..cells_per_axis {
            let x0 = -1.0 + x as f32 * step;
            let x1 = x0 + step;
            let position = |x, y| Vertex {
                position: Vec4::new(x, y, 0.0, 1.0),
            };

            vertices.extend([
                position(x0, y0),
                position(x1, y0),
                position(x1, y1),
                position(x0, y0),
                position(x1, y1),
                position(x0, y1),
            ]);
        }
    }

    vertices
}

fn indexed_triangle_grid(cells_per_axis: usize) -> (Vec<Vertex>, Vec<usize>) {
    let vertices_per_axis = cells_per_axis + 1;
    let mut vertices = Vec::with_capacity(vertices_per_axis * vertices_per_axis);
    let mut indices = Vec::with_capacity(cells_per_axis * cells_per_axis * 6);
    let step = 2.0 / cells_per_axis as f32;

    for y in 0..vertices_per_axis {
        for x in 0..vertices_per_axis {
            vertices.push(Vertex {
                position: Vec4::new(-1.0 + x as f32 * step, -1.0 + y as f32 * step, 0.0, 1.0),
            });
        }
    }

    for y in 0..cells_per_axis {
        for x in 0..cells_per_axis {
            let top_left = y * vertices_per_axis + x;
            let top_right = top_left + 1;
            let bottom_left = top_left + vertices_per_axis;
            let bottom_right = bottom_left + 1;
            indices.extend([
                top_left,
                top_right,
                bottom_right,
                top_left,
                bottom_right,
                bottom_left,
            ]);
        }
    }

    (vertices, indices)
}

fn rasterizer_benchmarks(criterion: &mut Criterion) {
    let renderer = Renderer::new(WIDTH, HEIGHT);
    let mut group = criterion.benchmark_group("triangle_grid");
    group.sample_size(30);

    for cells_per_axis in [8, 32] {
        let vertices = triangle_grid(cells_per_axis);
        let triangle_count = vertices.len() / 3;
        let mut pipeline = create_render_pipeline(
            BenchVertexShader,
            BenchFragmentShader,
            PrimitiveState::new(PrimitiveTopology::triangle_list()),
        );

        group.bench_with_input(
            BenchmarkId::new("without_depth", triangle_count),
            &vertices,
            |bencher, vertices| {
                bencher.iter_batched(
                    || vec![0_u32; PIXEL_COUNT],
                    |mut framebuffer| {
                        renderer
                            .begin_render_pass()
                            .set_pipeline(&mut pipeline)
                            .draw(vertices, &mut framebuffer, &());
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("with_depth", triangle_count),
            &vertices,
            |bencher, vertices| {
                bencher.iter_batched(
                    || (vec![0_u32; PIXEL_COUNT], vec![f32::INFINITY; PIXEL_COUNT]),
                    |(mut framebuffer, mut depth_buffer)| {
                        renderer
                            .begin_render_pass()
                            .set_pipeline(&mut pipeline)
                            .with_depth(&mut depth_buffer)
                            .draw(vertices, &mut framebuffer, &());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();

    let (vertices, indices) = indexed_triangle_grid(32);
    let triangle_count = indices.len() / 3;
    let mut pipeline = create_render_pipeline(
        BenchVertexShader,
        BenchFragmentShader,
        PrimitiveState::new(PrimitiveTopology::triangle_list()),
    );
    let mut indexed_group = criterion.benchmark_group("indexed_triangle_grid");
    indexed_group.sample_size(30);

    indexed_group.bench_function(
        BenchmarkId::new("without_depth", triangle_count),
        |bencher| {
            bencher.iter_batched(
                || vec![0_u32; PIXEL_COUNT],
                |mut framebuffer| {
                    renderer
                        .begin_render_pass()
                        .set_pipeline(&mut pipeline)
                        .draw_indexed(&vertices, indices.iter().copied(), &mut framebuffer, &());
                },
                BatchSize::LargeInput,
            );
        },
    );

    indexed_group.bench_function(BenchmarkId::new("with_depth", triangle_count), |bencher| {
        bencher.iter_batched(
            || (vec![0_u32; PIXEL_COUNT], vec![f32::INFINITY; PIXEL_COUNT]),
            |(mut framebuffer, mut depth_buffer)| {
                renderer
                    .begin_render_pass()
                    .set_pipeline(&mut pipeline)
                    .with_depth(&mut depth_buffer)
                    .draw_indexed(&vertices, indices.iter().copied(), &mut framebuffer, &());
            },
            BatchSize::LargeInput,
        );
    });

    indexed_group.finish();
}

criterion_group!(benches, rasterizer_benchmarks);
criterion_main!(benches);
