pub struct Foo;
impl Foo { pub(crate) fn b() -> bool { true } }
pub fn use_b() -> bool { Foo::b() }
