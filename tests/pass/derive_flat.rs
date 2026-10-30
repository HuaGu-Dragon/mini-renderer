use mini_renderer::Varying;

#[derive(Clone, Copy, Varying)]
pub struct VaryingTest {
    tex_coord: (f32, f32),
    color: (f32, f32, f32),
    #[flat]
    texture_id: i32,
}

fn main() {}
