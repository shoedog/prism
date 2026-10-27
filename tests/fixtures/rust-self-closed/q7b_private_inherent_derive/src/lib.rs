#[derive(Default)]
pub struct Foo;
mod m {
    use super::Foo;
    impl Foo { fn default() -> u8 { 1 } }
}
impl Foo {
    pub fn caller() -> Foo { Self::default() }
}
