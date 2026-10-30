pub struct T;
impl T {
    pub const fn b() -> u8 { 1 }
    pub fn caller() { const C: u8 = Self::b(); }
    pub fn inline() { let _ = const { Self::b() }; }
}
