pub struct T;
impl T {
 pub fn b() -> bool { true } pub fn b() -> u8 { 1 }
 pub fn caller() { Self::b(); }
}
