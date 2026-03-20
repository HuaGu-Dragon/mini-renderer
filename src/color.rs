pub trait ColorFormat {
    type Output: Copy;

    fn to_output(self) -> Self::Output;

    fn from_output(color: Self::Output) -> Self;
}
