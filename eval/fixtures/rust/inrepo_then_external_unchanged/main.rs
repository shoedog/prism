pub struct Foo;
impl Foo { pub fn ext(&self) -> String { String::new() } }
pub struct LocalA;
impl LocalA { pub fn m(&self) {} }
pub struct LocalB;
impl LocalB { pub fn m(&self) {} }
pub fn a() -> Foo { Foo }
pub fn drive() { a().ext().m(); }
