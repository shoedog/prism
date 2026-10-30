use std::panic::Location as Site;
pub fn f() -> u32 {
    let ignored = 1;
    Site::caller().line()
}
#[test]
fn t() { assert_eq!(Site::caller().line() - f(), 3); }
