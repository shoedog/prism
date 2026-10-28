pub struct Foo<const N: usize>;
const fn len(s: &str) -> usize { s.len() }
impl Foo<{ len("a b") }> { pub fn f() {} }
impl Foo<{ len("ab") }> { pub fn caller() { Self::f(); } }
