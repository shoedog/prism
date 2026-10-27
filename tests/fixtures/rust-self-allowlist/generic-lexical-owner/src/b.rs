use crate::Foo;
pub struct A;
impl Foo<A> { pub fn caller() { Self::f(); } }
