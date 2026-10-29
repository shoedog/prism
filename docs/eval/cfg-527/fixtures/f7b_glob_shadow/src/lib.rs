mod a {
    pub fn h() -> u8 { 1 }
}
mod user {
    use crate::a::*;
    #[cfg(feature = "y")]
    fn h() -> bool { true }
    pub fn caller() -> u8 { h() }
}
