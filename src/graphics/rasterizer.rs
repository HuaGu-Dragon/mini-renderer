use crate::{
    graphics::primitive::Primitive,
    math::{Vec2, Vec4},
    pipeline::varying::Varying,
};

pub struct Fragment<V> {
    pub x: usize,
    pub y: usize,
    pub varying: V,
}

pub trait Rasterizer<Var> {
    fn rasterize<'a>(
        &self,
        primitive: impl Iterator<Item = Primitive<'a, Var>>,
    ) -> impl Iterator<Item = Fragment<Var>>
    where
        Var: Varying + 'a;
}

pub struct TriangleRasterizer {
    pub width: usize,
    pub height: usize,
}

impl TriangleRasterizer {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    fn clip_to_screen(&self, clip_pos: Vec4) -> Vec2 {
        let ndc_x = clip_pos.x / clip_pos.w;
        let ndc_y = clip_pos.y / clip_pos.w;

        let screen_x = (ndc_x + 1.) * 0.5 * self.width as f32;
        let screen_y = (1. - ndc_y) * 0.5 * self.height as f32;

        Vec2::new(screen_x, screen_y)
    }

    fn edge_function(a: Vec2, b: Vec2, c: Vec2) -> f32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }

    fn rasterize_triangle<Var>(
        &self,
        v0: Vec2,
        v1: Vec2,
        v2: Vec2,
        v0_varying: &Var,
        v1_varying: &Var,
        v2_varying: &Var,
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
        let max_x = max_x.min(self.width as i32);
        let min_y = min_y.max(0);
        let max_y = max_y.min(self.height as i32);

        let area = Self::edge_function(v0, v1, v2);

        if area <= 0.0 {
            return fragments;
        }

        for y in min_y..max_y {
            for x in min_x..max_x {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);

                let w0 = Self::edge_function(v1, v2, p);
                let w1 = Self::edge_function(v2, v0, p);
                let w2 = Self::edge_function(v0, v1, p);

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    fragments.push(Fragment {
                        x: x as usize,
                        y: y as usize,
                        varying: Varying::interpolate(
                            v0_varying,
                            v1_varying,
                            v2_varying,
                            w0 / area,
                            w1 / area,
                            w2 / area,
                        ),
                    });
                }
            }
        }

        fragments
    }
}

impl<Var> Rasterizer<Var> for TriangleRasterizer {
    fn rasterize<'a>(
        &self,
        primitive: impl Iterator<Item = Primitive<'a, Var>>,
    ) -> impl Iterator<Item = Fragment<Var>>
    where
        Var: Varying + 'a,
    {
        primitive.flat_map(|p| match p {
            Primitive::Triangle(vertex_output, vertex_output1, vertex_output2) => {
                let v0 = self.clip_to_screen(vertex_output.position);
                let v1 = self.clip_to_screen(vertex_output1.position);
                let v2 = self.clip_to_screen(vertex_output2.position);

                self.rasterize_triangle(
                    v0,
                    v1,
                    v2,
                    &vertex_output.varying,
                    &vertex_output1.varying,
                    &vertex_output2.varying,
                )
            }
        })
    }
}
