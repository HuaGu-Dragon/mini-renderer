use mini_renderer::Varying;

#[derive(Clone, Copy, Varying)]
pub union VaryingTest {
    tex_coord: (f32, f32),
    color: (f32, f32, f32),
}

fn main() {}
