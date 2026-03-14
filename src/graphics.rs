pub mod color;
pub mod primitive;
pub mod rasterizer;
pub mod topology;

pub enum FrontFace {
    Ccw,
    Cw,
}

pub enum Face {
    Front,
    Back,
}
