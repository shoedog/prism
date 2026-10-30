pub struct T;
pub trait Tr { fn b(); }
impl Tr for T { fn b() {} }
impl T { pub fn b() {} pub fn caller() { Self::b(); } }
