//! Shared admission for free-function candidate pools.
use crate::{
    call_graph::FunctionId,
    languages::{same_language_family, Language},
};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "resolution_free_tests.rs"]
mod tests;

pub(super) fn same_family(caller_file: &str, candidate: &FunctionId) -> bool {
    // Same-file lookup retains its existing behavior, including explicit-language
    // ParsedFile callers whose paths have no recognized extension.
    caller_file == candidate.file
        || match (
            Language::from_path(caller_file),
            Language::from_path(&candidate.file),
        ) {
            (Some(caller), Some(callee)) => same_language_family(caller, callee),
            _ => false,
        }
}

pub(super) fn candidates<'a>(
    ids: &'a [FunctionId],
    owners: &BTreeMap<FunctionId, String>,
    caller_file: &str,
) -> Vec<&'a FunctionId> {
    ids.iter()
        .filter(|fid| !owners.contains_key(*fid) && same_family(caller_file, fid))
        .collect()
}
