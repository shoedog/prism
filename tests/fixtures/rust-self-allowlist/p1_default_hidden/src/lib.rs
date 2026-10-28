pub mod a;
pub struct T;
pub trait Tr1 {
    fn a1();
    fn b() {}
}
impl Tr1 for T {
    fn a1() { Self::b(); }
}
impl T {
    pub fn inherent_caller() { Self::b(); }
}
