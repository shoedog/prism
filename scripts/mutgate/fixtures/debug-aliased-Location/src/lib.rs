use std::panic::Location as Site;
pub fn f() -> String {
    let ignored = 1;
    format!("{:?}", Site::caller())
}
#[test]
fn t() { let expected = Site::caller().line() - 3; assert!(f().contains(&format!("line: {}", expected))); }
