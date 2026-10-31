//! Shared admission for free-function candidate pools.
use crate::{
    call_graph::FunctionId,
    languages::{same_language_family, Language},
};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "resolution_free_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "resolution_family_tests.rs"]
mod family_tests;

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
) -> Vec<&'a FunctionId> {
    ids.iter()
        .filter(|fid| !owners.contains_key(*fid))
        .collect()
}

// Admission runs after the legacy rung has selected targets and confidence.
// A removed candidate is neither evidence of uniqueness nor permission to try
// a broader rung. This also preserves Go profile partitions and telemetry.
//
// Method rungs (R1 `T::m`, R3b qualifier owner, R4b implicit-this, the
// self/receiver-variable and recovered-receiver owner lookups, R6) pool owners
// from `CallGraph::methods`/`functions` without a language key, so they are
// admitted once, on the public full-resolution result (`resolve_call_site_full`
// in `js_anonymous.rs` and its Go-memo twin): the rung, kind and confidence are
// the unfiltered pool's, foreign-family targets are then removed, and a pool
// left empty refuses with no later rung consulted (#585).
impl super::ResolutionOutcome<'_> {
    pub(crate) fn retain_family(mut self, caller_file: &str) -> Self {
        if !self.resolved.is_empty() {
            self.resolved.retain(|r| same_family(caller_file, r.target));
            if self.resolved.is_empty() {
                self.drop = Some(super::DropReason::UnknownName);
            }
        }
        self
    }
}

use super::{dir_of, go_visible_value_candidates};

/// P5 (Go func-value callbacks, S2): resolve a bare identifier used as a
/// VALUE (not a call) to the unique in-repo free function it names, using the
/// SAME same-package/import conventions as ordinary Go name resolution
/// (R4/R4.5) — same-file wins, else a single same-directory (package) free
/// function. Deliberately stops there: "NEVER bare cross-package name
/// matching" (spec-review MAJOR) means we do not fall through to a
/// repo-wide/FreeMulti search the way R5 does for calls.
pub fn resolve_go_bare_value_ref(
    functions: &BTreeMap<String, Vec<FunctionId>>,
    method_owners: &BTreeMap<FunctionId, String>,
    go_file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    ambiguous_counter: &mut usize,
    caller_file: &str,
    name: &str,
) -> Option<FunctionId> {
    let ids = functions.get(name)?;
    let free = candidates(ids, method_owners);
    let local: Vec<&FunctionId> = free
        .iter()
        .copied()
        .filter(|f| f.file == caller_file)
        .collect();
    if local.len() == 1 {
        return Some(local[0].clone());
    }
    if !local.is_empty() {
        return None; // >1 same-file free fn of this name: ambiguous, not our problem to disambiguate
    }
    let dir = dir_of(caller_file);
    let same_pkg: Vec<&FunctionId> = free
        .iter()
        .copied()
        .filter(|f| dir_of(&f.file) == dir)
        .collect();
    let raw_ambiguous = same_pkg.len() > 1;
    if raw_ambiguous {
        *ambiguous_counter += 1;
    }
    let same_pkg = go_visible_value_candidates(caller_file, &same_pkg, go_file_profiles);
    if same_pkg.len() == 1 {
        let (target, exact_allowed) = same_pkg[0];
        // A foreign-only result is refused, not an ambiguous Go value.
        if !same_family(caller_file, target) {
            return None;
        }
        if !exact_allowed {
            if !raw_ambiguous {
                *ambiguous_counter += 1;
            }
            return None;
        }
        return Some(target.clone());
    }
    None
}
