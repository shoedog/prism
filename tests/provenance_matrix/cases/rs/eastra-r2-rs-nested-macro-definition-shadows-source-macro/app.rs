macro_rules! edition {
    () => { std::env::var("EDITION").ok() };
}
fn labels() -> (Option<String>, &'static str) {
    let original = edition!();
    let preview = {
        macro_rules! edition { () => { "preview" }; }
        edition!()
    };
    (original, preview)
}
