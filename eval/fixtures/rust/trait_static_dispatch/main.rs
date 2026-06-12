trait Runner {
    fn go(&self);
}

struct Fast;

impl Runner for Fast {
    fn go(&self) {}
}

fn run(f: Fast) {
    f.go();
}
