//! Independent Go tree scans, collected in file order on the shared build pool.
use super::{scan_go, BTreeMap, BTreeSet, Language, ParsedFile};
use rayon::prelude::*;

pub(super) fn collect(files: &BTreeMap<String, ParsedFile>) -> BTreeSet<String> {
    #[cfg(test)]
    if crate::go_type_alias::import_memo::parallel::serial_reference_enabled() {
        return reference(files);
    }
    let mut live = BTreeSet::new();
    for row in prepare(files) {
        live.extend(row);
    }
    live
}

fn prepare(files: &BTreeMap<String, ParsedFile>) -> Vec<BTreeSet<String>> {
    let jobs: Vec<_> = files
        .values()
        .filter(|parsed| parsed.language == Language::Go)
        .collect();
    if jobs.is_empty() {
        return Vec::new();
    }
    crate::build_pool::install(|| {
        crate::build_pool::per_item(jobs)
            .map(|parsed| {
                let mut live = BTreeSet::new();
                scan_go(parsed, &mut live);
                live
            })
            .collect()
    })
}

#[cfg(test)]
fn reference(files: &BTreeMap<String, ParsedFile>) -> BTreeSet<String> {
    let mut live = BTreeSet::new();
    for parsed in files.values() {
        if parsed.language == Language::Go {
            scan_go(parsed, &mut live);
        }
    }
    live
}

#[cfg(test)]
mod tests;
