//! Bulk label handling for owners with very many edges.
//!
//! One-by-one `BTreeMap::entry` of cloned `(VarLocation, VarLocation)` keys
//! costs a full tree descent of string comparisons per edge. For an owner whose
//! labels are produced in one piece they are sorted once and built in bulk.
use super::Producer;
use crate::cpg::FlowConfidence;
use crate::data_flow::{FlowEdge, VarLocation};
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub(super) type Labels = BTreeMap<(VarLocation, VarLocation), FlowConfidence>;

/// Below this many labels the plain per-edge insert is cheaper than a batch.
pub(super) const BULK_MIN: usize = 4096;

#[cfg(test)]
thread_local! {
    static REFERENCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static BULK_RUNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
/// Bulk label builds and appends executed on the workers of `pool`; a
/// differential that sets `REFERENCE` on its own pool asserts this is zero for
/// the reference build and positive for the new one.
#[cfg(test)]
pub(in crate::data_flow) fn bulk_runs_on(pool: &rayon::ThreadPool) -> usize {
    pool.broadcast(|_| BULK_RUNS.with(|n| n.get())).iter().sum()
}
#[cfg(test)]
fn note_bulk() {
    BULK_RUNS.with(|n| n.set(n.get() + 1));
}
/// Test switch: run the per-edge `insert_label` and the unshortened
/// `store_supplemental_exact` that predate the owner-bulk paths.
#[cfg(test)]
pub(in crate::data_flow) fn set_reference_paths(on: bool) {
    REFERENCE.with(|r| r.set(on));
}
pub(super) fn reference_paths() -> bool {
    #[cfg(test)]
    return REFERENCE.with(|r| r.get());
    #[cfg(not(test))]
    false
}

fn same_owner_prefix(a: &VarLocation, b: &VarLocation) -> bool {
    a.function_start_line == b.function_start_line && a.function == b.function && a.file == b.file
}

/// `VarLocation::cmp` without the file/function/start-line prefix. Only valid
/// when both operands share that prefix (checked by the caller).
fn cmp_tail(a: &VarLocation, b: &VarLocation) -> Ordering {
    a.line
        .cmp(&b.line)
        .then_with(|| a.path.cmp(&b.path))
        .then_with(|| a.kind.cmp(&b.kind))
}

impl Producer {
    /// The labels `insert_label` would build from `edges`, all carrying
    /// `label`: keys unique by `VarLocation` identity, the first-inserted key
    /// kept for equal identities (they can differ in bytes).
    pub(super) fn fallback_owner_labels(edges: &[FlowEdge], label: FlowConfidence) -> Labels {
        #[cfg(test)]
        note_bulk();
        let Some(first) = edges.first() else {
            return Labels::new();
        };
        let anchor = &first.from;
        let shared = edges
            .iter()
            .all(|e| same_owner_prefix(&e.from, anchor) && same_owner_prefix(&e.to, anchor));
        let compare = |a: &FlowEdge, b: &FlowEdge| -> Ordering {
            if shared {
                cmp_tail(&a.from, &b.from).then_with(|| cmp_tail(&a.to, &b.to))
            } else {
                (&a.from, &a.to).cmp(&(&b.from, &b.to))
            }
        };
        let mut order: Vec<usize> = (0..edges.len()).collect();
        // Edges are produced definition by definition in document order, so
        // they usually arrive sorted already; sorting is skipped then.
        let ascending = edges
            .windows(2)
            .all(|pair| compare(&pair[0], &pair[1]) != Ordering::Greater);
        if !ascending {
            // Stable: among equal identities the earliest edge stays first.
            order.sort_by(|&a, &b| compare(&edges[a], &edges[b]));
        }
        let mut sorted = Vec::with_capacity(edges.len());
        let mut previous: Option<&FlowEdge> = None;
        for index in order {
            let edge = &edges[index];
            if previous.is_some_and(|p| compare(p, edge) == Ordering::Equal) {
                continue;
            }
            previous = Some(edge);
            sorted.push(((edge.from.clone(), edge.to.clone()), label));
        }
        sorted.into_iter().collect()
    }

    /// Fold one owner's labels into the file's. `owner_unique` means no other
    /// owner of the file shares this owner's `(name, start line)`, so no key
    /// can already be present and `append` equals one `insert_label` per key.
    pub(super) fn merge_owner_labels(labels: &mut Labels, mut batch: Labels, owner_unique: bool) {
        if batch.is_empty() {
            return;
        }
        if labels.is_empty() {
            #[cfg(test)]
            note_bulk();
            *labels = batch;
        } else if owner_unique
            && batch.len() >= BULK_MIN
            && batch.len().saturating_mul(16) >= labels.len()
        {
            #[cfg(test)]
            note_bulk();
            labels.append(&mut batch);
        } else {
            for (key, label) in batch {
                Self::insert_label(labels, key, label);
            }
        }
    }
}
