use a::Foo;
pub trait Tr { fn a() -> u8; }
pub trait Tr2 { fn b() -> u8; }
impl<X> Tr2 for X { fn b() -> u8 { 1 } }
impl Tr for Foo { fn a() -> u8 { Self::b() } }
