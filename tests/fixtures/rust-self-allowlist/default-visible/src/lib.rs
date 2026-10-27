pub struct T;
pub trait Left { fn b() -> u8 { 1 } }
impl Left for T {}
mod right { pub trait Right { fn b() -> bool; } }
impl right::Right for T { fn b() -> bool { true } }
impl T { pub fn caller() -> u8 { Self::b() } }
