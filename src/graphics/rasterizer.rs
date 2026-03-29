use crate::{
    graphics::{Face, FrontFace},
    math::{FloatExt, Vec2, Vec4},
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

    fn new(front_face: FrontFace, cull_mode: Option<Face>) -> Self;

    fn rasterize(
        &self,
        primitive: impl Iterator<Item = Self::Primitive<Var>>,
        width: usize,
        height: usize,
    ) -> impl Iterator<Item = Fragment<Var>>
    where
        Var: Varying,
    {
        self.rasterize_tile(primitive, width, height, [0, 0, width, height])
    }

    fn rasterize_tile(
        &self,
        primitive: impl Iterator<Item = Self::Primitive<Var>>,
        width: usize,
        height: usize,
        tile_bounds: [usize; 4],
    ) -> impl Iterator<Item = Fragment<Var>>
    where
        Var: Varying;
}

pub struct TriangleRasterizer {
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
}

impl TriangleRasterizer {
    pub fn new(front_face: FrontFace, cull_mode: Option<Face>) -> Self {
        Self {
            front_face,
            cull_mode,
        }
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

    fn rasterize_triangle<Var>(
        &self,
        positions: [Vec4; 3],
        varyings: [Var; 3],
        tile_bounds: [usize; 4],
    ) -> impl Iterator<Item = Fragment<Var>>
    where
        Var: Varying,
    {
        let [v0, v1, v2] = positions;
        let [v0_varying, v1_varying, v2_varying] = varyings;
        let [tile_x, tile_y, tile_width, tile_height] = tile_bounds;

        let min_x = v0.x.min(v1.x).min(v2.x).floor_custom() as i32;
        let max_x = v0.x.max(v1.x).max(v2.x).ceil_custom() as i32;
        let min_y = v0.y.min(v1.y).min(v2.y).floor_custom() as i32;
        let max_y = v0.y.max(v1.y).max(v2.y).ceil_custom() as i32;

        let min_x = min_x.max(tile_x as i32);
        let max_x = max_x.min((tile_x + tile_width) as i32);
        let min_y = min_y.max(tile_y as i32);
        let max_y = max_y.min((tile_y + tile_height) as i32);

        let area = Self::edge_function(
            Vec2::new(v0.x, v0.y),
            Vec2::new(v1.x, v1.y),
            Vec2::new(v2.x, v2.y),
        );

        let is_front_face = match self.front_face {
            FrontFace::Ccw => area > 0.0,
            FrontFace::Cw => area < 0.0,
        };

        let should_cull = area == 0.0
            || match self.cull_mode {
                Some(crate::graphics::Face::Front) => is_front_face,
                Some(crate::graphics::Face::Back) => !is_front_face,
                None => false,
            };

        let mut w0_row = 0.0;
        let mut w1_row = 0.0;
        let mut w2_row = 0.0;

        let mut step_x0 = 0.0;
        let mut step_x1 = 0.0;
        let mut step_x2 = 0.0;

        let mut step_y0 = 0.0;
        let mut step_y1 = 0.0;
        let mut step_y2 = 0.0;

        let mut inv_area = 0.0;
        let mut inv_w0 = 0.0;
        let mut inv_w1 = 0.0;
        let mut inv_w2 = 0.0;

        if !should_cull {
            step_x0 = v1.y - v2.y;
            step_x1 = v2.y - v0.y;
            step_x2 = v0.y - v1.y;

            step_y0 = v2.x - v1.x;
            step_y1 = v0.x - v2.x;
            step_y2 = v1.x - v0.x;

            let p_row = Vec2::new(min_x as f32 + 0.5, min_y as f32 + 0.5);

            w0_row = Self::edge_function(Vec2::new(v1.x, v1.y), Vec2::new(v2.x, v2.y), p_row);
            w1_row = Self::edge_function(Vec2::new(v2.x, v2.y), Vec2::new(v0.x, v0.y), p_row);
            w2_row = Self::edge_function(Vec2::new(v0.x, v0.y), Vec2::new(v1.x, v1.y), p_row);

            inv_area = 1.0 / area;
            inv_w0 = 1.0 / v0.w;
            inv_w1 = 1.0 / v1.w;
            inv_w2 = 1.0 / v2.w;
        }

        let x_range = min_x..max_x;
        let y_range = min_y..max_y;

        y_range.flat_map(move |y| {
            let mut w0 = w0_row;
            let mut w1 = w1_row;
            let mut w2 = w2_row;

            w0_row += step_y0;
            w1_row += step_y1;
            w2_row += step_y2;

            x_range.clone().filter_map(move |x| {
                let current_w0 = w0;
                let current_w1 = w1;
                let current_w2 = w2;

                w0 += step_x0;
                w1 += step_x1;
                w2 += step_x2;

                if should_cull {
                    return None;
                }

                let inside = (current_w0 * area >= 0.0)
                    && (current_w1 * area >= 0.0)
                    && (current_w2 * area >= 0.0);

                if inside {
                    let alpha = current_w0 * inv_area;
                    let beta = current_w1 * inv_area;
                    let gamma = current_w2 * inv_area;

                    let pc_w0 = alpha * inv_w0;
                    let pc_w1 = beta * inv_w1;
                    let pc_w2 = gamma * inv_w2;
                    let inv_w = pc_w0 + pc_w1 + pc_w2;
                    let inv_pc_sum = 1.0 / inv_w;

                    Some(Fragment {
                        x: x as usize,
                        y: y as usize,
                        depth: Varying::interpolate(v0.z, v1.z, v2.z, alpha, beta, gamma),
                        varying: Varying::interpolate(
                            v0_varying,
                            v1_varying,
                            v2_varying,
                            pc_w0 * inv_pc_sum,
                            pc_w1 * inv_pc_sum,
                            pc_w2 * inv_pc_sum,
                        ),
                    })
                } else {
                    None
                }
            })
        })
    }
}

impl<Var> Rasterizer<Var> for TriangleRasterizer {
    // type Primitive<'a, V>
    //     = &'a [VertexOutput<V>; 3]
    // where
    //     V: 'a;
    type Primitive<V> = [VertexOutput<V>; 3];

    fn new(front_face: FrontFace, cull_mode: Option<crate::graphics::Face>) -> Self {
        Self {
            front_face,
            cull_mode,
        }
    }

    fn rasterize_tile(
        &self,
        primitive: impl Iterator<Item = Self::Primitive<Var>>,
        width: usize,
        height: usize,
        tile_bounds: [usize; 4],
    ) -> impl Iterator<Item = Fragment<Var>>
    where
        Var: Varying,
    {
        primitive
            .filter_map(move |[vertex_output, vertex_output1, vertex_output2]| {
                if Self::should_cull_triangle(
                    vertex_output.position,
                    vertex_output1.position,
                    vertex_output2.position,
                ) {
                    None
                } else {
                    let v0 = self.clip_to_screen(vertex_output.position, width, height);
                    let v1 = self.clip_to_screen(vertex_output1.position, width, height);
                    let v2 = self.clip_to_screen(vertex_output2.position, width, height);

                    Some(self.rasterize_triangle(
                        [v0, v1, v2],
                        [
                            vertex_output.varying,
                            vertex_output1.varying,
                            vertex_output2.varying,
                        ],
                        tile_bounds,
                    ))
                }
            })
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_edge_function_positive() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(1.0, 0.0);
        let c = Vec2::new(0.0, 1.0);
        let result = TriangleRasterizer::edge_function(a, b, c);
        assert!(result > 0.0);
    }

    #[test]
    fn test_edge_function_negative() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        let c = Vec2::new(1.0, 0.0);
        let result = TriangleRasterizer::edge_function(a, b, c);
        assert!(result < 0.0);
    }

    #[test]
    fn test_edge_function_collinear() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(1.0, 1.0);
        let c = Vec2::new(2.0, 2.0);
        let result = TriangleRasterizer::edge_function(a, b, c);
        assert!(approx_eq(result, 0.0));
    }

    #[test]
    fn test_edge_function_unit_triangle() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(1.0, 0.0);
        let c = Vec2::new(0.0, 1.0);
        let result = TriangleRasterizer::edge_function(a, b, c);
        assert!(approx_eq(result, 1.0));
    }

    #[test]
    fn test_clip_to_screen_ndc_center() {
        let rasterizer = TriangleRasterizer::new(FrontFace::Ccw, None);
        let clip_pos = Vec4::new(0.0, 0.0, 0.0, 1.0); // NDC center
        let screen_pos = rasterizer.clip_to_screen(clip_pos, 100, 100);

        // NDC center (0,0) should map to screen center (50, 50)
        assert!(approx_eq(screen_pos.x, 50.0));
        assert!(approx_eq(screen_pos.y, 50.0));
        assert!(approx_eq(screen_pos.z, 0.5));
    }

    #[test]
    fn test_clip_to_screen_ndc_left_bottom() {
        let rasterizer = TriangleRasterizer::new(FrontFace::Ccw, None);
        let clip_pos = Vec4::new(-1.0, -1.0, -1.0, 1.0); // NDC left-bottom
        let screen_pos = rasterizer.clip_to_screen(clip_pos, 100, 100);

        // NDC (-1,-1) should map to screen (0, 100)
        assert!(approx_eq(screen_pos.x, 0.0));
        assert!(approx_eq(screen_pos.y, 100.0));
    }

    #[test]
    fn test_clip_to_screen_ndc_right_top() {
        let rasterizer = TriangleRasterizer::new(FrontFace::Ccw, None);
        let clip_pos = Vec4::new(1.0, 1.0, 1.0, 1.0); // NDC right-top
        let screen_pos = rasterizer.clip_to_screen(clip_pos, 100, 100);

        // NDC (1,1) should map to screen (100, 0)
        assert!(approx_eq(screen_pos.x, 100.0));
        assert!(approx_eq(screen_pos.y, 0.0));
    }

    #[test]
    fn test_clip_to_screen_perspective() {
        let rasterizer = TriangleRasterizer::new(FrontFace::Ccw, None);
        let clip_pos = Vec4::new(1.0, 1.0, 1.0, 2.0); // w != 1.0
        let screen_pos = rasterizer.clip_to_screen(clip_pos, 100, 100);

        // Perspective division: 1.0 / 2.0 = 0.5
        // screen_x = (0.5 + 1.0) * 0.5 * 100 = 75
        // screen_y = (1.0 - 0.5) * 0.5 * 100 = 25
        assert!(approx_eq(screen_pos.x, 75.0));
        assert!(approx_eq(screen_pos.y, 25.0));
    }

    #[test]
    fn test_should_cull_triangle_completely_behind() {
        // All vertices behind near plane
        let v0 = Vec4::new(0.0, 0.0, -2.0, 1.0);
        let v1 = Vec4::new(1.0, 0.0, -2.0, 1.0);
        let v2 = Vec4::new(0.0, 1.0, -2.0, 1.0);
        assert!(TriangleRasterizer::should_cull_triangle(v0, v1, v2));
    }

    #[test]
    fn test_should_cull_triangle_completely_in_front() {
        // All vertices in front of far plane
        let v0 = Vec4::new(0.0, 0.0, 2.0, 1.0);
        let v1 = Vec4::new(1.0, 0.0, 2.0, 1.0);
        let v2 = Vec4::new(0.0, 1.0, 2.0, 1.0);
        assert!(TriangleRasterizer::should_cull_triangle(v0, v1, v2));
    }

    #[test]
    fn test_should_cull_triangle_left_of_viewport() {
        // All vertices left of viewport
        let v0 = Vec4::new(-2.0, 0.0, 0.0, 1.0);
        let v1 = Vec4::new(-1.5, 0.5, 0.0, 1.0);
        let v2 = Vec4::new(-1.5, -0.5, 0.0, 1.0);
        assert!(TriangleRasterizer::should_cull_triangle(v0, v1, v2));
    }

    #[test]
    fn test_should_cull_triangle_right_of_viewport() {
        // All vertices right of viewport
        let v0 = Vec4::new(2.0, 0.0, 0.0, 1.0);
        let v1 = Vec4::new(1.5, 0.5, 0.0, 1.0);
        let v2 = Vec4::new(1.5, -0.5, 0.0, 1.0);
        assert!(TriangleRasterizer::should_cull_triangle(v0, v1, v2));
    }

    #[test]
    fn test_should_not_cull_visible_triangle() {
        // Visible triangle in the center
        let v0 = Vec4::new(-0.5, -0.5, 0.0, 1.0);
        let v1 = Vec4::new(0.5, -0.5, 0.0, 1.0);
        let v2 = Vec4::new(0.0, 0.5, 0.0, 1.0);
        assert!(!TriangleRasterizer::should_cull_triangle(v0, v1, v2));
    }
}
