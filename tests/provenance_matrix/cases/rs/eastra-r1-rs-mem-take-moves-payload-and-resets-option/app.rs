pub fn labels() -> (Option<String>, Option<String>) {
    let mut current = std::env::var("EDITION").ok();
    let prior = std::mem::take(&mut current);
    (prior, current)
}
