use mini_renderer::Varying;

#[derive(Clone, Copy, Varying)]
pub enum VaryingTest {
    TexCoord(f32, f32),
    Color(f32, f32, f32),
}

fn main() {}
