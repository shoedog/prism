pub struct T;
pub trait Left { fn b() {} }
pub trait Right { fn b(); }
impl Left for T {}
impl Right for T { fn b() {} }
impl T { pub fn caller() { Self::b(); } }
