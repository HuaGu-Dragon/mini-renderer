pub mod primitive;
pub mod rasterizer;
pub mod topology;

#[derive(Debug, PartialEq)]
pub enum FrontFace {
    Ccw,
    Cw,
}

#[derive(Debug, PartialEq)]
pub enum Face {
    Front,
    Back,
}
