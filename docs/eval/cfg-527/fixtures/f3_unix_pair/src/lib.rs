pub struct S;
impl S {
    #[cfg(unix)]
    pub fn p(&self) -> u8 { 1 }
    #[cfg(not(unix))]
    pub fn p(&self) -> u8 { 2 }
    pub fn q(&self) -> u8 { self.p() }
}
