use std::marker::PhantomData;

pub trait Varying: Sized + Copy {
    /// Interpolate between three vertices using barycentric coordinates
    fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self;
}

impl Varying for () {
    fn interpolate(_v0: Self, _v1: Self, _v2: Self, _w0: f32, _w1: f32, _w2: f32) -> Self {}
}

impl<T> Varying for PhantomData<T> {
    fn interpolate(_v0: Self, _v1: Self, _v2: Self, _w0: f32, _w1: f32, _w2: f32) -> Self {
        PhantomData
    }
}

macro_rules! impl_varying {
    ([$($T:ident $idx:tt),*]) => {
        impl<$($T: Varying,)*> Varying for ($($T,)*) {
            fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self {
                (
                    $(
                        Varying::interpolate(v0.$idx, v1.$idx, v2.$idx, w0, w1, w2),
                    )*
                )
            }
        }
    };
    ($($ty:ty),*) => {
        $(
            impl Varying for $ty {
                fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self {
                    (w0 * v0 as f32 + w1 * v1 as f32 + w2 * v2 as f32).into()
                }
            }
        )*
    };
}

impl_varying!(f32, f64);
impl_varying!([T0 0]);
impl_varying!([T0 0, T1 1]);
impl_varying!([T0 0, T1 1, T2 2]);
impl_varying!([T0 0, T1 1, T2 2, T3 3]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4, T5 5]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10, T11 11]);
impl_varying!([T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10, T11 11, T12 12]);
