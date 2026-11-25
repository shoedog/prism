use std::ffi::OsString;
fn sample() -> Option<(OsString, OsString)> {
    let (key, value) = std::env::vars_os().next()?;
    Some((key, value))
}
