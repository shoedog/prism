fn f() {
    let mut x = source();
    {
        let x = clean();
        sink(x);
    }
    x = clean();
    sink(x);
}
