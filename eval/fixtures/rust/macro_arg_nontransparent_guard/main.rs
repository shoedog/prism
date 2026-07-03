fn check(x: i32) -> bool {
    x > 0
}

fn run() {
    stringify!(check(1));
    my_macro!(check(2));
}
