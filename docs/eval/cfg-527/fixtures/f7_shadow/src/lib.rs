pub struct T;
pub trait Tr {
    fn b(&self) -> u8;
    fn c(&self) -> u8 { self.b() }
}
impl Tr for T { fn b(&self) -> u8 { 1 } }
mod inner {
    impl super::T {
        #[cfg(feature = "y")]
        pub fn b(&self) -> bool { true }
    }
}
