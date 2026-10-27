pub struct T;
pub struct A;
pub struct B;
pub trait Conv<P> {
    fn conv(_p: P) {}
}
impl Conv<A> for T {
    fn conv(_p: A) {}
}
impl Conv<B> for T {}
impl T {
    pub fn use_b(b: B) { Self::conv(b); }
}
