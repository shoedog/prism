//! Projection of a complete taint trace to the roots allowed to cause loading.
use crate::algorithms::taint::{TaintRootOrigin, TaintTrace};
use std::collections::{BTreeMap, BTreeSet};

/// Keep Diff/Explicit roots whose source file is original and loaded; copy whole, in order.
pub(crate) fn project_loading_trace<V>(
    full: &TaintTrace,
    original_files: &BTreeSet<String>,
    loaded_files: &BTreeMap<String, V>,
) -> TaintTrace {
    let roots = full
        .roots
        .iter()
        .filter(|root| {
            matches!(
                root.origin,
                TaintRootOrigin::Diff | TaintRootOrigin::Explicit
            )
        })
        .filter(|root| original_files.contains(&root.seed.file))
        .filter(|root| loaded_files.contains_key(&root.seed.file))
        .cloned()
        .collect();
    TaintTrace { roots }
}
