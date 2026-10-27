pub struct T;
impl T { pub fn b() -> bool { true } }
pub trait Tr { fn b() -> u8 { 1 } }
pub fn host() {
    struct T;
    impl Tr for T {}
    impl T { fn caller() -> u8 { Self::b() } }
}
