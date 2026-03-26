#[cfg(feature = "glam")]
pub use glam::*;

#[cfg(not(feature = "glam"))]
pub use inner::*;

#[cfg(feature = "glam")]
mod glam {
    pub type Vec2 = glam::Vec2;
    pub type Vec3 = glam::Vec3;
    pub type Vec4 = glam::Vec4;
    pub type Mat3 = glam::Mat3;
    pub type Mat4 = glam::Mat4;
}

#[cfg(not(feature = "glam"))]
mod inner {
    #[derive(Debug, Default, Clone, Copy)]
    pub struct Vec2 {
        pub x: f32,
        pub y: f32,
    }

    impl Vec2 {
        pub fn new(x: f32, y: f32) -> Self {
            Self { x, y }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl Vec3 {
        pub fn new(x: f32, y: f32, z: f32) -> Self {
            Self { x, y, z }
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub struct Vec4 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
        pub w: f32,
    }

    impl Vec4 {
        pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
            Self { x, y, z, w }
        }
    }
}
