pub struct Foo;
pub type FA = Foo;
impl FA { pub fn b() -> bool { true } }
pub trait Tr { fn a() -> bool; fn b() -> u8; }
impl Tr for Foo {
    fn a() -> bool { Self::b() }
    fn b() -> u8 { 1 }
}
