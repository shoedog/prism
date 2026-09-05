fn f() {
    let x = source();
    {
        let x = clean();
        sink(x);
    }
    sink(x);
}
