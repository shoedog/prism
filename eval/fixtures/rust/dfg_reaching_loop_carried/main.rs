fn f() {
    let mut x = source();
    loop {
        sink(x);
        x = next_value();
    }
}
