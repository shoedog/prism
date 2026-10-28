#![allow(uncommon_codepoints,non_snake_case)]
pub enum E { K(u8) }
impl E {
 pub fn K(_:u8) -> bool {true}
 pub fn caller() -> E {Self::K(1)}
}
