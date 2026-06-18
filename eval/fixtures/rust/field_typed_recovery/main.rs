struct Inner;

impl Inner {
    fn poke(&self) {}
}

struct Outer {
    inner: Inner,
}

fn run(o: Outer) {
    let x = o.inner;
    x.poke();
}
