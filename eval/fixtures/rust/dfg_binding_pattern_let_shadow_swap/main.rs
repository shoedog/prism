fn f() {
    let (a, b) = (clean(), 1);
    let a = source();
    sink(a);
    consume(b);
}

fn g() {
    let a = clean();
    let (a, b) = (source(), 1);
    sink(a);
    consume(b);
}
