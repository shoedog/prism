macro_rules! make_observer { ($name:ident) => { #[track_caller] fn $name() -> u32 { std::panic::Location::caller().line() } }; }
make_observer!(position);
pub fn f() -> u32 {
    let ignored = 1;
    position()
}
#[test]
fn t() { assert_eq!(position() - f(), 3); }
