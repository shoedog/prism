pub struct A; pub struct B;
pub struct Foo<T>(T);
impl Foo<B> { pub fn b() -> u8 { 2 } }
impl Foo<A> { pub fn caller() -> u8 { Self::b() } }
