pub struct Foo;
mod t { pub trait FA { fn a() -> u8 where Self: Sized; } }
pub trait Tr2 { fn b() -> u8; }
impl<X> Tr2 for X { fn b() -> u8 { 1 } }
type FA = Foo;
impl t::FA for FA { fn a() -> u8 { Self::b() } }
mod d { use crate::t::FA; #[allow(bare_trait_objects)] impl FA { pub fn b() -> bool { true } } }
