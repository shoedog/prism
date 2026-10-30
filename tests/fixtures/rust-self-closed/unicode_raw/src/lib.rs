#![allow(uncommon_codepoints,non_snake_case)]
pub enum E { r#É(u8) }
impl E {
 pub fn r#É(_:u8) -> bool {true}
 pub fn caller() -> E {Self::r#É(1)}
}
