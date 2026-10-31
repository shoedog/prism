//! Ordered per-file signature maps on the same large-stack pool as the build.
use super::{Imports, ParsedFile};
use crate::languages::Language;
use rayon::prelude::*;
use std::collections::BTreeMap;

pub(super) fn prepare<'a>(
    files: &'a BTreeMap<String, ParsedFile>,
    local_paths: &BTreeMap<String, String>,
) -> Vec<(&'a str, Imports)> {
    let jobs: Vec<_> = files
        .iter()
        .filter(|(_, parsed)| parsed.language == Language::Go)
        .collect();
    if jobs.is_empty() {
        return Vec::new();
    }
    crate::build_pool::install(|| {
        crate::build_pool::per_item(jobs)
            .map(|(path, parsed)| {
                (
                    path.as_str(),
                    super::super::canon::signature_imports_unmeasured(
                        parsed,
                        local_paths.get(path).map(String::as_str),
                    ),
                )
            })
            .collect()
    })
}

// Set on the build worker by equivalence tests, so the production call sites
// use the original lazy serial computation for a complete reference build.
#[cfg(test)]
thread_local! {
    static SERIAL_REFERENCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn serial_reference_enabled() -> bool {
    SERIAL_REFERENCE.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(crate) fn with_serial_reference<R>(op: impl FnOnce() -> R) -> R {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            SERIAL_REFERENCE.with(|flag| flag.set(self.0));
        }
    }
    let _reset = Reset(SERIAL_REFERENCE.with(|flag| flag.replace(true)));
    op()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod work_sites;
