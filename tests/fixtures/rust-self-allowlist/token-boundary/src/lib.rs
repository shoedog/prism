pub trait Tr {}
pub struct dynTr;
pub struct Foo<T: ?Sized>(Box<T>);
impl Foo<dynTr> { pub fn f() {} }
impl Foo<dyn Tr> { pub fn caller() { Self::f(); } }
