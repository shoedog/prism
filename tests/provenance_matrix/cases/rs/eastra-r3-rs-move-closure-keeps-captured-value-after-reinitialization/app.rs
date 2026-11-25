use std::env;
fn labels() -> Option<(String, String)> {
    let mut edition = env::var("EDITION").ok()?;
    let saved = move || {
        edition
    };
    edition = env::args().nth(1)?;
    Some((saved(), edition))
}
