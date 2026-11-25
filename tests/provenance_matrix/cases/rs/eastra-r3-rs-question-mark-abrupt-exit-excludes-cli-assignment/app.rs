use std::env;
fn label() -> Option<String> {
    let mut edition = env::var("EDITION").ok()?;
    if let Some(arg) = env::args().nth(1) {
        edition = arg;
        None::<()>?;
    }
    Some(edition)
}
