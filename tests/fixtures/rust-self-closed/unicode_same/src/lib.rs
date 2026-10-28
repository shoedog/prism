#![allow(uncommon_codepoints,non_snake_case)]
pub enum E { É(u8) }
impl E {
 pub fn É(_:u8) -> bool {true}
 pub fn caller() -> E {Self::É(1)}
}
