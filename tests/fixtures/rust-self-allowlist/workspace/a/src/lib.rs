pub struct T;
impl T { pub fn b() {} pub fn caller_a() { Self::b(); } }
