trait Runner {
    fn go(&self);
}

struct Fast;

impl Runner for Fast {
    fn go(&self) {}
}

fn run(r: &dyn Runner) {
    r.go();
}
