pub struct Foo;
pub type Alias = Foo;
pub trait Tr { fn b() -> bool; fn caller(); }
pub trait Other { fn b() -> u8; }
impl Tr for Foo {
 fn b() -> bool { true }
 fn caller() { Self::b(); }
}
impl Other for Alias { fn b() -> u8 { 1 } }
