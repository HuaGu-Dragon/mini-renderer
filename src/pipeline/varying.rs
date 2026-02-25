pub trait Varying: Sized {
    /// Interpolate between three vertices using barycentric coordinates
    fn interpolate(v0: &Self, v1: &Self, v2: &Self, w0: f32, w1: f32, w2: f32) -> Self;
}

impl Varying for () {
    fn interpolate(_v0: &Self, _v1: &Self, _v2: &Self, _w0: f32, _w1: f32, _w2: f32) -> Self {}
}

impl Varying for f32 {
    fn interpolate(v0: &Self, v1: &Self, v2: &Self, w0: f32, w1: f32, w2: f32) -> Self {
        w0 * v0 + w1 * v1 + w2 * v2
    }
}

impl Varying for (f32, f32) {
    fn interpolate(v0: &Self, v1: &Self, v2: &Self, w0: f32, w1: f32, w2: f32) -> Self {
        (
            Varying::interpolate(&v0.0, &v1.0, &v2.0, w0, w1, w2),
            Varying::interpolate(&v0.1, &v1.1, &v2.1, w0, w1, w2),
        )
    }
}

impl Varying for (f32, f32, f32) {
    fn interpolate(v0: &Self, v1: &Self, v2: &Self, w0: f32, w1: f32, w2: f32) -> Self {
        (
            Varying::interpolate(&v0.0, &v1.0, &v2.0, w0, w1, w2),
            Varying::interpolate(&v0.1, &v1.1, &v2.1, w0, w1, w2),
            Varying::interpolate(&v0.2, &v1.2, &v2.2, w0, w1, w2),
        )
    }
}
