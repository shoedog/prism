pub fn labels() -> (Option<String>, &'static str) {
    let parsed = std::env::var("EDITION").ok();
    let spelling = stringify!(std::env::var("EDITION"));
    (parsed, spelling)
}
