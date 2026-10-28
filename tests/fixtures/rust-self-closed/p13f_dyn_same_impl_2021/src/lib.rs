pub trait Tr { fn x(&self) {} }
impl dyn Tr { pub fn b() -> bool { true } pub fn caller() -> bool { Self::b() } }
