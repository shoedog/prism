trait Ext {
    fn ext(&self);
}

impl Ext for String {
    fn ext(&self) {}
}

fn run(s: String) {
    s.ext();
}
