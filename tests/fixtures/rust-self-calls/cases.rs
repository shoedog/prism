pub struct T;
impl T {
    pub fn assoc() {}
    pub fn method(&self) {}
    pub fn assoc_call() { Self::assoc(); }
    pub fn method_call(&self) { Self::method(self); }
    pub fn closure_call() { let c = || Self::assoc(); c(); }
    pub fn nested_call() { fn nested() { Self::assoc(); } nested(); }
    pub fn missing_call() { Self::missing(); }
    pub const CONST: usize = 1;
    pub fn const_path() { let _ = Self::CONST; }
    pub fn type_control() { T::assoc(); }
    pub fn dot_control(&self) { self.method(); }
}
pub struct Other;
impl Other { pub fn assoc() {} }
pub struct G<'a>(&'a str);
impl<'a> G<'a> {
    pub fn g_assoc() {}
    pub fn lifetime_call() { Self::g_assoc(); }
}
pub struct W<U>(U);
impl<U> W<U> {
    pub fn w_assoc() {}
    pub fn generic_call() { Self::w_assoc(); }
}
pub trait Tr {
    fn a();
    fn b();
}
impl Tr for T {
    fn a() { Self::b(); }
    fn b() { Self::assoc(); }
}
pub trait DefaultTr {
    fn default_a() { Self::default_b(); }
    fn default_b() {}
}
impl DefaultTr for T { fn default_b() {} }
pub enum E { Variant(i32) }
impl E { pub fn enum_call() -> Self { Self::Variant(1) } }
pub struct Split;
impl Split { pub fn new() -> Self { Split } }
pub struct Same;
impl Same { pub fn same_target() {} pub fn same_call() { Self::same_target(); } }

impl Same { pub fn absent_in_this_type() { Self::foreign_only(); } }
pub struct Configured;
impl Configured {
    #[cfg(feature = "first")]
    pub fn choice() {}
    #[cfg(not(feature = "first"))]
    pub fn choice() {}
    pub fn configured_call() { Self::choice(); }
}
pub struct X;
pub struct Y;
impl From<X> for T { fn from(_: X) -> Self { T } }
impl From<Y> for T { fn from(_: Y) -> Self { T } }
impl T { pub fn from_x(x: X) -> Self { Self::from(x) } }
