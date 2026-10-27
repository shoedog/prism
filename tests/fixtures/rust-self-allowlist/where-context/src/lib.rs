pub struct T;
impl T {
    pub const fn count() -> usize { 1 }
    pub fn caller() where [(); Self::count()]: Sized {}
    pub fn inline() { let _ = const { Self::count() }; }
}
