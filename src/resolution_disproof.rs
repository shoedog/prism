//! The disproof seam (spec §3): a sound candidate-elimination primitive shared by
//! owner-keyed call resolution.
//!
//! A [`DisproofPredicate`] **proves a candidate is not the callee at a site**; it
//! must be sound — return "not disproved" whenever uncertain. [`prune`] composes
//! predicates by **intersection**: a candidate survives unless *some* predicate
//! disproves it. Recall-safe by construction (P1): adding predicates can only
//! shrink the surviving set, never wrongly drop the true target, and a prune that
//! would empty a non-empty pool returns the ORIGINAL pool (no-confidence, never a
//! drop).
//!
//! This slice ships exactly one predicate — [`ScopeResolution`] (in `resolution.rs`,
//! wired in Task 5). The seam is the extensibility deliverable: future
//! precision-recovery (reachability, arity, receiver-type, trait-bound) becomes new
//! predicates composed into the same intersection.

use crate::call_graph::{CallSite, FunctionId};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::types::{FileId, ScopeId};

/// Read-only context a [`DisproofPredicate`] may consult. Borrows the scope graph
/// and the call site's already-resolved enclosing scope (the `(file, from)` the
/// caller computed via `rust_authoritative_scope`), so a predicate need not
/// recompute authority.
#[derive(Clone, Copy)]
pub struct DisproofCx<'a> {
    /// The whole-repo scope graph (authoritative — the caller gated on `complete`).
    pub graph: &'a ScopeGraph,
    /// The call site's containing file in the graph.
    pub file: FileId,
    /// The call site's enclosing lexical scope (from `enclosing_scope`).
    pub from: ScopeId,
}

/// A sound disproof predicate.
///
/// SOUND iff `disproves` returns `true` ONLY when it can prove `cand` is not the
/// target at `site`. Implementations must return `false` (not disproved) on any
/// uncertainty.
pub trait DisproofPredicate {
    fn disproves(&self, cand: &FunctionId, site: &CallSite, cx: &DisproofCx<'_>) -> bool;
}

/// Prune `pool` to the candidates no predicate disproves.
///
/// Intersection semantics: a candidate is kept unless *some* predicate disproves
/// it. If pruning would empty a non-empty `pool`, the ORIGINAL `pool` is returned
/// — a disproof that eliminates everything is treated as no-confidence (P1), never
/// a drop. An empty input `pool` returns empty.
pub fn prune<'a>(
    pool: Vec<&'a FunctionId>,
    site: &CallSite,
    cx: &DisproofCx<'_>,
    preds: &[&dyn DisproofPredicate],
) -> Vec<&'a FunctionId> {
    if pool.is_empty() {
        return pool;
    }
    let kept: Vec<&'a FunctionId> = pool
        .iter()
        .copied()
        .filter(|cand| !preds.iter().any(|p| p.disproves(cand, site, cx)))
        .collect();
    if kept.is_empty() {
        pool
    } else {
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_graph::{CallSiteOrigin, FunctionId};
    use crate::name_resolution::graph::ScopeGraph;
    use crate::name_resolution::types::{FileId, ScopeId};

    fn fid(name: &str) -> FunctionId {
        FunctionId {
            file: "a.rs".to_string(),
            name: name.to_string(),
            start_line: 1,
            end_line: 1,
        }
    }

    fn dummy_site() -> CallSite {
        CallSite {
            caller: fid("caller"),
            callee_name: "Foo::m".to_string(),
            line: 1,
            kind: Default::default(),
            start_byte: 0,
            end_byte: 0,
            qualifier: None,
            receiver_type: None,
            receiver_owner_identity: None,
            receiver_recovery: None,
            receiver_materialized: false,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
            origin: CallSiteOrigin::Source,
            pre_resolved_target: None,
        }
    }

    fn dummy_cx(graph: &ScopeGraph) -> DisproofCx<'_> {
        DisproofCx {
            graph,
            file: FileId(0),
            from: ScopeId(0),
        }
    }

    /// A trivial predicate that disproves exactly the candidates whose name is in
    /// its deny-list. Used to validate the seam mechanics (not shipped).
    struct DenyNames(Vec<String>);
    impl DisproofPredicate for DenyNames {
        fn disproves(&self, cand: &FunctionId, _site: &CallSite, _cx: &DisproofCx<'_>) -> bool {
            self.0.contains(&cand.name)
        }
    }

    #[test]
    fn prune_keeps_undisproved_candidates() {
        let graph = ScopeGraph::new();
        let a = fid("keep_a");
        let b = fid("deny_b");
        let pool = vec![&a, &b];
        let pred = DenyNames(vec!["deny_b".to_string()]);
        let kept = prune(pool, &dummy_site(), &dummy_cx(&graph), &[&pred]);
        assert_eq!(kept, vec![&a], "only the disproved candidate is removed");
    }

    #[test]
    fn prune_to_empty_returns_original_pool() {
        // P1: a prune that eliminates EVERYTHING is no-confidence → keep the
        // original pool, never a drop.
        let graph = ScopeGraph::new();
        let a = fid("deny_a");
        let b = fid("deny_b");
        let pred = DenyNames(vec!["deny_a".to_string(), "deny_b".to_string()]);
        let kept = prune(vec![&a, &b], &dummy_site(), &dummy_cx(&graph), &[&pred]);
        assert_eq!(
            kept,
            vec![&a, &b],
            "prune-to-empty returns the original pool"
        );
    }

    #[test]
    fn prune_with_no_predicates_is_identity() {
        let graph = ScopeGraph::new();
        let a = fid("a");
        let b = fid("b");
        let kept = prune(vec![&a, &b], &dummy_site(), &dummy_cx(&graph), &[]);
        assert_eq!(kept, vec![&a, &b]);
    }

    #[test]
    fn prune_empty_input_is_empty() {
        let graph = ScopeGraph::new();
        let pred = DenyNames(vec![]);
        let kept: Vec<&FunctionId> = prune(Vec::new(), &dummy_site(), &dummy_cx(&graph), &[&pred]);
        assert!(kept.is_empty());
    }

    #[test]
    fn prune_intersection_across_two_predicates() {
        // A candidate survives only if NO predicate disproves it.
        let graph = ScopeGraph::new();
        let a = fid("a");
        let b = fid("b");
        let c = fid("c");
        let p1 = DenyNames(vec!["b".to_string()]);
        let p2 = DenyNames(vec!["c".to_string()]);
        let kept = prune(
            vec![&a, &b, &c],
            &dummy_site(),
            &dummy_cx(&graph),
            &[&p1, &p2],
        );
        assert_eq!(kept, vec![&a]);
    }
}
