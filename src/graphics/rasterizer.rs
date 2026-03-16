use crate::{
    graphics::FrontFace,
    math::{Vec2, Vec4},
    pipeline::{shader::VertexOutput, varying::Varying},
};

pub struct Fragment<V> {
    pub x: usize,
    pub y: usize,
    pub depth: f32,
    pub varying: V,
}

pub trait Rasterizer<Var> {
    // FIXME: due to a current limitation of the type system, this implies a 'static lifetime
    // type Primitive<'a, V>
    // where
    //     V: 'a;
    type Primitive<V>;

    fn new(front_face: FrontFace) -> Self;

    fn rasterize(
        &self,
        primitive: impl Iterator<Item = Self::Primitive<Var>>,
        width: usize,
        height: usize,
    ) -> impl Iterator<Item = Fragment<Var>>
    where
        Var: Varying;
}

pub struct TriangleRasterizer {
    pub front_face: FrontFace,
}

impl TriangleRasterizer {
    pub fn new(front_face: FrontFace) -> Self {
        Self { front_face }
    }

    fn clip_to_screen(&self, clip_pos: Vec4, width: usize, height: usize) -> Vec4 {
        let ndc_x = clip_pos.x / clip_pos.w;
        let ndc_y = clip_pos.y / clip_pos.w;
        let ndc_z = clip_pos.z / clip_pos.w;

        let screen_x = (ndc_x + 1.) * 0.5 * width as f32;
        let screen_y = (1. - ndc_y) * 0.5 * height as f32;
        let screen_z = (ndc_z + 1.) * 0.5;

        Vec4::new(screen_x, screen_y, screen_z, clip_pos.w)
    }

    fn edge_function(a: Vec2, b: Vec2, c: Vec2) -> f32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }

    fn should_cull_triangle(v0: Vec4, v1: Vec4, v2: Vec4) -> bool {
        if v0.z < -v0.w && v1.z < -v1.w && v2.z < -v2.w {
            return true;
        }
        if v0.z > v0.w && v1.z > v1.w && v2.z > v2.w {
            return true;
        }
        if v0.x < -v0.w && v1.x < -v1.w && v2.x < -v2.w {
            return true;
        }
        if v0.x > v0.w && v1.x > v1.w && v2.x > v2.w {
            return true;
        }
        if v0.y < -v0.w && v1.y < -v1.w && v2.y < -v2.w {
            return true;
        }
        if v0.y > v0.w && v1.y > v1.w && v2.y > v2.w {
            return true;
        }
        false
    }

    // TODO: simplify inputs
    #[allow(clippy::too_many_arguments)]
    fn rasterize_triangle<Var>(
        &self,
        v0: Vec4,
        v1: Vec4,
        v2: Vec4,
        v0_varying: &Var,
        v1_varying: &Var,
        v2_varying: &Var,
        width: usize,
        height: usize,
    ) -> Vec<Fragment<Var>>
    where
        Var: Varying,
    {
        let mut fragments = Vec::new();

        let min_x = v0.x.min(v1.x).min(v2.x).floor() as i32;
        let max_x = v0.x.max(v1.x).max(v2.x).ceil() as i32;
        let min_y = v0.y.min(v1.y).min(v2.y).floor() as i32;
        let max_y = v0.y.max(v1.y).max(v2.y).ceil() as i32;

        let min_x = min_x.max(0);
        let max_x = max_x.min(width as i32);
        let min_y = min_y.max(0);
        let max_y = max_y.min(height as i32);

        let area = Self::edge_function(
            Vec2::new(v0.x, v0.y),
            Vec2::new(v1.x, v1.y),
            Vec2::new(v2.x, v2.y),
        );

        let should_cull = match self.front_face {
            FrontFace::Ccw => area <= 0.0,
            FrontFace::Cw => area >= 0.0,
        };

        if should_cull {
            return fragments;
        }

        for y in min_y..max_y {
            for x in min_x..max_x {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);

                let w0 = Self::edge_function(Vec2::new(v1.x, v1.y), Vec2::new(v2.x, v2.y), p);
                let w1 = Self::edge_function(Vec2::new(v2.x, v2.y), Vec2::new(v0.x, v0.y), p);
                let w2 = Self::edge_function(Vec2::new(v0.x, v0.y), Vec2::new(v1.x, v1.y), p);

                let weight0 = (w0 / area) / v0.w;
                let weight1 = (w1 / area) / v1.w;
                let weight2 = (w2 / area) / v2.w;

                let sum = weight0 + weight1 + weight2;

                let inside = if area > 0.0 {
                    w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
                } else {
                    w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
                };

                if inside {
                    fragments.push(Fragment {
                        x: x as usize,
                        y: y as usize,
                        depth: Varying::interpolate(
                            v0.z,
                            v1.z,
                            v2.z,
                            weight0 / sum,
                            weight1 / sum,
                            weight2 / sum,
                        ),
                        varying: Varying::interpolate(
                            *v0_varying,
                            *v1_varying,
                            *v2_varying,
                            weight0 / sum,
                            weight1 / sum,
                            weight2 / sum,
                        ),
                    });
                }
            }
        }

        fragments
    }
}

impl<Var> Rasterizer<Var> for TriangleRasterizer {
    // type Primitive<'a, V>
    //     = &'a [VertexOutput<V>; 3]
    // where
    //     V: 'a;
    type Primitive<V> = [VertexOutput<V>; 3];

    fn new(front_face: FrontFace) -> Self {
        Self { front_face }
    }

    fn rasterize(
        &self,
        primitive: impl Iterator<Item = Self::Primitive<Var>>,
        width: usize,
        height: usize,
    ) -> impl Iterator<Item = Fragment<Var>>
    where
        Var: Varying,
    {
        primitive.flat_map(move |p| match p {
            [vertex_output, vertex_output1, vertex_output2] => {
                if Self::should_cull_triangle(
                    vertex_output.position,
                    vertex_output1.position,
                    vertex_output2.position,
                ) {
                    return Vec::new();
                }

                let v0 = self.clip_to_screen(vertex_output.position, width, height);
                let v1 = self.clip_to_screen(vertex_output1.position, width, height);
                let v2 = self.clip_to_screen(vertex_output2.position, width, height);

                self.rasterize_triangle(
                    v0,
                    v1,
                    v2,
                    &vertex_output.varying,
                    &vertex_output1.varying,
                    &vertex_output2.varying,
                    width,
                    height,
                )
            }
        })
    }
}
