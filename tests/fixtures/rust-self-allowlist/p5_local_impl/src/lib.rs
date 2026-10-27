pub struct T;
impl T {
    pub fn z() {}
    pub fn outer() {
        struct Inner;
        impl Inner {
            fn x() {}
            fn y() { Self::x(); }
        }
        Inner::y();
        Self::z();
    }
}
pub trait Dx { fn x() {} }
pub struct U;
impl U { pub fn x() {} }
pub fn free() {
    struct U;
    impl Dx for U {}
    impl U {
        fn y() { Self::x(); }
    }
    U::y();
}
