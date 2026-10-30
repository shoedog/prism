pub struct Foo<T>(pub T);
pub mod a {
    use crate::Foo; pub struct A;
    impl Foo<A> { pub fn f() {} }
}
pub mod b {
    use crate::Foo; pub struct A;
    impl Foo<A> { pub fn caller() { Self::f(); } }
}
