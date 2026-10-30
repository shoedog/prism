macro_rules! make_observer { ($name:ident) => { #[track_caller] fn $name() -> u32 { std::panic::Location::caller().line() } }; }
make_observer!(position);
pub fn f() -> u32 {
    let ignored = 1;
    position()
}
#[test]
#[should_panic(expected="at line difference 4")]
fn t() { panic!("at line difference {}", position() - f()); }
