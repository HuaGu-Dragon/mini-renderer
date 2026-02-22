pub trait IntoColor: Sized {
    type Output;

    fn into_color(self) -> Self::Output;
}
