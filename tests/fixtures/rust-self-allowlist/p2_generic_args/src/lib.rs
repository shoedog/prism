pub struct Foo<X>(pub X);
pub struct A;
pub struct B;
impl Foo<A> {
    pub fn f() {}
}
pub trait Tf {
    fn f() {}
    fn g();
}
impl Tf for Foo<B> {
    fn g() { Self::f(); }
}
