#![allow(uncommon_codepoints, bare_trait_objects)]
pub struct K;
mod t { pub trait K { fn a() -> u8 where Self: Sized; } }
pub trait Tr2 { fn b() -> u8; }
impl<X> Tr2 for X { fn b() -> u8 { 1 } }
use t::*;
impl t::K for K { fn a() -> u8 { Self::b() } }
mod d { use crate::t::K; impl K { pub fn b() -> bool { true } } }
