pub struct LocalA;
impl LocalA { pub fn count(&self) {} }
pub struct LocalB;
impl LocalB { pub fn count(&self) {} }
pub fn drive(v: Vec<u8>) {
    v.iter().count();
}
