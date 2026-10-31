//! Phase-local recovery facts: the immutable file is scanned once, not once
//! for every callback admission consumer. No cache survives source replacement.
use super::{contains_recovery_unmeasured, Language, ParsedFile};
use rayon::prelude::*;
use std::collections::BTreeMap;

pub(super) struct RecoveryFacts<'a> {
    rows: BTreeMap<&'a str, bool>,
    #[cfg(test)]
    serial: bool,
}

impl<'a> RecoveryFacts<'a> {
    pub(super) fn new(
        files: &'a BTreeMap<String, ParsedFile>,
        profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    ) -> Self {
        #[cfg(test)]
        if crate::go_type_alias::import_memo::parallel::serial_reference_enabled() {
            return Self {
                rows: BTreeMap::new(),
                serial: true,
            };
        }
        let jobs: Vec<_> = files
            .iter()
            .filter(|(_, parsed)| parsed.language == Language::Go)
            // Union of admissible_file and clean_package_dirs' preconditions.
            // Keep both path- and profile-based test checks, even for supplied
            // profiles whose is_test_file disagrees with the path spelling.
            .filter(|(path, _)| {
                profiles.get(*path).is_some_and(|profile| {
                    !profile.package_clause.is_empty()
                        && (!profile.is_test_file
                            || (!path.ends_with("_test.go") && !profile.build_unparsed))
                })
            })
            .collect();
        let rows: Vec<_> = if jobs.is_empty() {
            Vec::new()
        } else {
            crate::build_pool::install(|| {
                crate::build_pool::per_item(jobs)
                    .map(|(path, parsed)| {
                        (
                            path.as_str(),
                            contains_recovery_unmeasured(parsed.tree.root_node()),
                        )
                    })
                    .collect()
            })
        };
        #[cfg(test)]
        record_scans(rows.len());
        Self {
            rows: rows.into_iter().collect(),
            #[cfg(test)]
            serial: false,
        }
    }

    pub(super) fn contains(&self, path: &str, parsed: &ParsedFile) -> bool {
        #[cfg(test)]
        if self.serial {
            return super::contains_recovery(parsed.tree.root_node());
        }
        // Every Go input was prepared. A missing entry fails closed.
        let _ = parsed;
        self.rows.get(path).copied().unwrap_or(true)
    }
}

#[cfg(test)]
thread_local! {
    static SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn record_scans(n: usize) {
    SCANS.with(|count| count.set(count.get() + n));
}

#[cfg(test)]
fn measured<R>(op: impl FnOnce() -> R) -> (R, usize) {
    SCANS.with(|count| count.set(0));
    let result = op();
    (result, SCANS.with(std::cell::Cell::get))
}

#[cfg(test)]
mod tests;
