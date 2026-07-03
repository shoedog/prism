fn Foo(x: i32) -> i32 {
    x
}

fn run() {
    assert!(Foo(1).0 == 1);
}
