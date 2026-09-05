fn f() {
    let mut x = source();
    let thunk = || sink(x);
    x = clean();
    thunk();
}
