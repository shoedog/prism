fn f(value: Option<i32>) {
    let x = source();
    if let Some(x) = value
        && check(x)
    {
        sink(x);
    }
    sink(x);
}
