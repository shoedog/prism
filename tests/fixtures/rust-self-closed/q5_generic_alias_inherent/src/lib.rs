pub struct A;
pub struct Foo<X>(pub X);
pub type FA = Foo<A>;
impl FA { pub fn b() -> bool { true } }
pub trait Tr { fn a() -> bool; fn b() -> u8; }
impl Tr for Foo<A> {
    fn a() -> bool { Self::b() }
    fn b() -> u8 { 1 }
}
