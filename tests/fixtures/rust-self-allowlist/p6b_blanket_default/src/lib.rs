pub trait Tb {
    fn a(&self);
    fn b(&self) {}
}
impl<X: Clone> Tb for X {
    fn a(&self) { Self::b(self); }
}
pub struct X;
impl X { pub fn b(&self) {} }
