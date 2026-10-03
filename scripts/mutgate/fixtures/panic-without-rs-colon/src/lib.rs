mod observer;
use observer::position as here;
pub fn f() -> u32 {
    let ignored = 1;
    here()
}
#[test]
fn t() { let observed = f(); let expected = here() - 3; let payload = std::panic::catch_unwind(|| panic!("at line {}", observed)).unwrap_err(); assert_eq!(payload.downcast_ref::<String>().unwrap(), &format!("at line {}", expected)); }
