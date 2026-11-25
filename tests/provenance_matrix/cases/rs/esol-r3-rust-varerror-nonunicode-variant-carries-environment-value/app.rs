use std::env::{self, VarError};
use std::ffi::OsString;

pub fn read() -> OsString {
    match env::var("PREVIEW_LABEL") {
        Ok(text) => text.into(),
        Err(VarError::NotUnicode(raw)) => raw,
        Err(VarError::NotPresent) => OsString::new(),
    }
}
