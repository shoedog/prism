pub struct A; pub struct B;
pub struct Foo<T>(T);
pub trait Tr { fn b() -> u8 { 1 } }
impl Tr for Foo<A> {}
impl Foo<B> { pub fn b() -> bool { true } }
impl Foo<A> { pub fn caller() -> u8 { Self::b() } }
