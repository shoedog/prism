pub struct Foo;
pub type Alias = Foo;
pub trait Tr { fn b() -> bool; fn caller() -> u8; }
impl Tr for Foo {
 fn b() -> bool { true }
 fn caller() -> u8 { Self::b() }
}
impl Alias { pub fn b() -> u8 { 1 } }
