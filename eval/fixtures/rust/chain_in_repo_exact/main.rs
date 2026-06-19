pub struct Builder;
impl Builder {
    pub fn new() -> Builder { Builder }
    pub fn cfg(&self, n: u8) -> Builder { Builder }
    pub fn run(&self) {}
}
pub struct Other;
impl Other {
    pub fn run(&self) {}
}
pub fn drive() {
    Builder::new().cfg(1).run();
}
