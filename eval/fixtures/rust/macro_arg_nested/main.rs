fn compute(x: i32) -> i32 {
    x + 1
}

fn run() {
    let v = vec![
        compute(1),
        compute(2),
    ];
}
