pub struct Foo;
mod m {
    use super::Foo;
    impl Foo { fn b() -> bool { true } }
}
pub trait Tr2 { fn b() -> u8; }
impl<X> Tr2 for X { fn b() -> u8 { 1 } }
pub trait Tr { fn a() -> u8; }
impl Tr for Foo {
    fn a() -> u8 { Self::b() }
}
