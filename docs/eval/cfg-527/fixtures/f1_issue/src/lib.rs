pub struct T;
pub trait Tr { fn b() -> u8; }
impl<X> Tr for X { fn b() -> u8 { 1 } }
impl T {
    #[cfg(any())]
    pub fn b() -> bool { true }
}
pub fn free_caller() -> u8 { T::b() }
