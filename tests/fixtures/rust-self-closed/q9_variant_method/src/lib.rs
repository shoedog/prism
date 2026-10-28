#![allow(non_snake_case)]
pub enum E { V(u8) }
impl E {
    pub fn V(_x: u8) -> bool { true }
    pub fn caller() -> E { Self::V(1) }
}
