pub struct T;
pub trait Tr { fn b() -> u8; }
impl<X> Tr for X { fn b() -> u8 { 1 } }
impl T {
    #[cfg(any())]
    fn b() -> bool { true }
    pub fn c() -> u8 { Self::b() }
}
