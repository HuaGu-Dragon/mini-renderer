use core::marker::PhantomData;

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

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_f32_interpolate_zero_weights() {
        let result = f32::interpolate(1.0, 2.0, 3.0, 1.0, 0.0, 0.0);
        assert!(approx_eq(result, 1.0));
    }

    #[test]
    fn test_f32_interpolate_equal_weights() {
        let result = f32::interpolate(1.0, 2.0, 3.0, 1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0);
        assert!(approx_eq(result, 2.0));
    }

    #[test]
    fn test_f32_interpolate_one_vertex() {
        let result = f32::interpolate(1.0, 2.0, 3.0, 0.0, 1.0, 0.0);
        assert!(approx_eq(result, 2.0));
    }

    #[test]
    fn test_f32_interpolate_mixed_weights() {
        let result = f32::interpolate(0.0, 1.0, 2.0, 0.5, 0.3, 0.2);
        let expected = 0.5 * 0.0 + 0.3 * 1.0 + 0.2 * 2.0;
        assert!(approx_eq(result, expected));
    }

    #[test]
    fn test_phantom_data_interpolate() {
        let result =
            PhantomData::<i32>::interpolate(PhantomData, PhantomData, PhantomData, 0.2, 0.5, 0.3);
        assert_eq!(result, PhantomData);
    }

    #[test]
    fn test_tuple_f32_interpolate() {
        let result = <(f32,)>::interpolate((1.0,), (2.0,), (3.0,), 0.5, 0.3, 0.2);
        assert!(approx_eq(result.0, 0.5 * 1.0 + 0.3 * 2.0 + 0.2 * 3.0));
    }

    #[test]
    fn test_tuple_f32_f32_interpolate() {
        let v0 = (1.0, 10.0);
        let v1 = (2.0, 20.0);
        let v2 = (3.0, 30.0);
        let result = <(f32, f32)>::interpolate(v0, v1, v2, 0.5, 0.3, 0.2);
        assert!(approx_eq(result.0, 0.5 * 1.0 + 0.3 * 2.0 + 0.2 * 3.0));
        assert!(approx_eq(result.1, 0.5 * 10.0 + 0.3 * 20.0 + 0.2 * 30.0));
    }

    #[test]
    fn test_tuple_f32_f32_f32_interpolate() {
        let v0 = (1.0, 10.0, 100.0);
        let v1 = (2.0, 20.0, 200.0);
        let v2 = (3.0, 30.0, 300.0);
        let weights = (0.5, 0.3, 0.2);
        let result = <(f32, f32, f32)>::interpolate(v0, v1, v2, weights.0, weights.1, weights.2);

        assert!(approx_eq(result.0, 0.5 * 1.0 + 0.3 * 2.0 + 0.2 * 3.0));
        assert!(approx_eq(result.1, 0.5 * 10.0 + 0.3 * 20.0 + 0.2 * 30.0));
        assert!(approx_eq(result.2, 0.5 * 100.0 + 0.3 * 200.0 + 0.2 * 300.0));
    }

    #[test]
    fn test_interpolate_perspective_correction() {
        let w0 = 0.5;
        let w1 = 0.3;
        let w2 = 0.2;

        assert!(approx_eq(w0 + w1 + w2, 1.0));

        let result = f32::interpolate(100.0, 200.0, 300.0, w0, w1, w2);
        let expected = w0 * 100.0 + w1 * 200.0 + w2 * 300.0;
        assert!(approx_eq(result, expected));
    }

    #[test]
    fn test_interpolate_at_edge() {
        let result = f32::interpolate(1.0, 2.0, 3.0, 0.5, 0.5, 0.0);
        let expected = 0.5 * 1.0 + 0.5 * 2.0;
        assert!(approx_eq(result, expected));
    }

    #[test]
    fn test_interpolate_negative_values() {
        let result = f32::interpolate(-1.0, -2.0, -3.0, 0.5, 0.3, 0.2);
        let expected = -0.5 + 0.3 * (-2.0) + 0.2 * (-3.0);
        assert!(approx_eq(result, expected));
    }
}
