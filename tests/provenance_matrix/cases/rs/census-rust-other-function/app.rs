fn remote() -> String {
    let mode = std::env::var("MODE").unwrap_or_default();
    mode
}
fn label() -> String {
    let mode = "ready".to_owned();
    mode
}
