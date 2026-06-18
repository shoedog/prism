struct R;

impl R {
    fn go(&self) {}
}

fn make() -> R {
    R
}

fn run() {
    let x = make();
    x.go();
}
