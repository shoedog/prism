fn f(o: Option<i32>) {
    let mut x = source();
    {
        let Some(v) = o else {
            x = clean();
            return;
        };
        sink(x);
    }
    sink(x);
}
