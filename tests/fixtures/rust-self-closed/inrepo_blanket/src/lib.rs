pub struct Foo;
pub trait Convert<T> { fn convert(x: T) -> Self; }
impl<T> Convert<T> for T {
 fn convert(x: T) -> Self { x }
}
impl Convert<u8> for Foo {
 fn convert(x: u8) -> Self { if x == 0 { Self::convert(Foo) } else { Foo } }
}
