fn f(value: Option<i32>) {
    let x = source();
    match value {
        Some(x) => {
            sink(x);
        }
        None => {
            sink(x);
        }
    }
    sink(x);
}
