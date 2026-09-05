fn f(c: bool) {
    let mut x = source();
    if c {
        x = clean();
    } else {
        sink(x);
    }
}
