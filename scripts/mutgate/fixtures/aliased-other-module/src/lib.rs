mod observer;
use observer::position as here;
pub fn f() -> u32 {
    let ignored = 1;
    here()
}
#[test]
fn t() { assert_eq!(here() - f(), 3); }
