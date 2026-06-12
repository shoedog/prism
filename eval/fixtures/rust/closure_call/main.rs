fn target_fn() {}

fn run() {
    let f = || target_fn();
    f();
}
