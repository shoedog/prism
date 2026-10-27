fn f(pair: (i32, i32)) {
    let a = source();
    {
        let (a, b) = pair;
        sink(a);
        consume(b);
    }
    sink(a);
}

fn g(pair: (i32, i32)) {
    let a = source();
    let (a, b) = pair;
    sink(a);
    consume(b);
}
