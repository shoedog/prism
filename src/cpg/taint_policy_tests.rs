//! Crossing-policy tests on hand-built graphs (#602 S3a.2): CP01-CP04, CP07 and CP09.
//!
//! Node and edge insertion order is explicit in every fixture, because discovery order depends
//! on it. Every surviving target is checked through its actual evidence path.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::NodeIndex;

use super::{
    CodePropertyGraph, CpgEdge, CpgNode, CrossingCeiling, FlowConfidence, FlowDoubt,
    TaintCrossingPolicy, VarAccess,
};
use crate::access_path::AccessPath;
use crate::data_flow::VarLocation;
use crate::finding_confidence::{EvidenceHop, EvidencePath};

pub(super) const EXACT: FlowConfidence = FlowConfidence::Exact;
pub(super) const WEAK: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::SameLine);

pub(super) type Walk = (
    BTreeSet<VarLocation>,
    BTreeMap<NodeIndex, FlowConfidence>,
    BTreeMap<NodeIndex, EvidencePath>,
);

pub(super) fn files(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// A Variable of `owner` = (file, function, function start line), indexed like a built one.
pub(super) fn var_in(
    cpg: &mut CodePropertyGraph,
    owner: (&str, &str, usize),
    line: usize,
    name: &str,
    access: VarAccess,
) -> NodeIndex {
    let (file, function, function_start_line) = owner;
    let idx = cpg.graph.add_node(CpgNode::Variable {
        path: AccessPath::simple(name),
        file: file.to_string(),
        function: function.to_string(),
        function_start_line,
        line,
        access,
        start_byte: line * 10,
        end_byte: line * 10 + 1,
    });
    let key = (file.to_string(), line);
    cpg.location_index.entry(key).or_default().push(idx);
    assert_eq!(cpg.var_index.insert_first_wins(&cpg.graph, idx), idx);
    idx
}

/// A Variable in function `f` starting on line 1 of `file`.
pub(super) fn var(
    cpg: &mut CodePropertyGraph,
    file: &str,
    line: usize,
    name: &str,
    access: VarAccess,
) -> NodeIndex {
    var_in(cpg, (file, "f", 1), line, name, access)
}

pub(super) fn flow(
    cpg: &mut CodePropertyGraph,
    from: NodeIndex,
    to: NodeIndex,
    label: FlowConfidence,
) {
    cpg.graph.add_edge(from, to, CpgEdge::DataFlow(label));
}

pub(super) fn loc(cpg: &CodePropertyGraph, idx: NodeIndex) -> VarLocation {
    cpg.to_var_location(idx).expect("a Variable node")
}

pub(super) fn walk(
    cpg: &CodePropertyGraph,
    start: NodeIndex,
    policy: Option<&TaintCrossingPolicy<'_>>,
) -> Walk {
    cpg.dfg_forward_reachable_with_evidence_policy(&loc(cpg, start), policy)
}

pub(super) fn reached(cpg: &CodePropertyGraph, walk: &Walk, node: NodeIndex) -> bool {
    walk.0.contains(&loc(cpg, node))
}

pub(super) fn hops_of(path: &EvidencePath) -> Vec<(VarLocation, VarLocation, FlowConfidence)> {
    let hop = |hop: &EvidenceHop| match hop {
        EvidenceHop::DataFlow {
            from,
            to,
            confidence,
        } => (from.clone(), to.clone(), *confidence),
        other => panic!("unexpected hop {other:?}"),
    };
    path.hops.iter().map(hop).collect()
}

/// The evidence of `target` holds exactly `want` (every hop's ends and label) and
/// `crossed_unlabeled == crossed`.
pub(super) fn assert_path(
    cpg: &CodePropertyGraph,
    evidence: &BTreeMap<NodeIndex, EvidencePath>,
    target: NodeIndex,
    want: &[(NodeIndex, NodeIndex, FlowConfidence)],
    crossed: bool,
) {
    let path = evidence.get(&target).expect("evidence for the target");
    let want: Vec<_> = want
        .iter()
        .map(|&(from, to, label)| (loc(cpg, from), loc(cpg, to), label))
        .collect();
    assert_eq!(hops_of(path), want);
    assert_eq!(path.crossed_unlabeled, crossed);
}

/// `s` (line 1, Def of `from_file`) -> `t` (line 2, Use of `to_file`), one edge per label in order.
pub(super) fn pair(
    from_file: &str,
    to_file: &str,
    labels: &[FlowConfidence],
) -> (CodePropertyGraph, Vec<NodeIndex>) {
    let mut cpg = CodePropertyGraph::empty();
    let s = var(&mut cpg, from_file, 1, "s", VarAccess::Def);
    let t = var(&mut cpg, to_file, 2, "t", VarAccess::Use);
    for &label in labels {
        flow(&mut cpg, s, t, label);
    }
    (cpg, vec![s, t])
}

/// CP03: S -WEAK-> T, S -EXACT-> M -EXACT-> T, T -EXACT-> K; nodes S, M, T, K in that order.
pub(super) fn detour(include_m: bool, reversed: bool) -> (CodePropertyGraph, Vec<NodeIndex>) {
    let mut cpg = CodePropertyGraph::empty();
    let s = var(&mut cpg, "o1.py", 1, "s", VarAccess::Def);
    let m = var(&mut cpg, "o2.py", 2, "m", VarAccess::Def);
    let t = var(&mut cpg, "c.py", 3, "t", VarAccess::Def);
    let k = var(&mut cpg, "c.py", 4, "k", VarAccess::Use);
    let mut edges = vec![(s, t, WEAK), (s, m, EXACT), (m, t, EXACT), (t, k, EXACT)];
    if !include_m {
        edges.retain(|&(from, to, _)| from != m && to != m);
    }
    if reversed {
        edges.reverse();
    }
    for (from, to, label) in edges {
        flow(&mut cpg, from, to, label);
    }
    (cpg, vec![s, m, t, k])
}

/// CP04: S -first-> M (same file) -second-> T (`c.py`); nodes S, M, T in that order.
pub(super) fn chain(
    first: FlowConfidence,
    second: FlowConfidence,
) -> (CodePropertyGraph, Vec<NodeIndex>) {
    let mut cpg = CodePropertyGraph::empty();
    let s = var(&mut cpg, "o.py", 1, "s", VarAccess::Def);
    let m = var(&mut cpg, "o.py", 2, "m", VarAccess::Def);
    let t = var(&mut cpg, "c.py", 3, "t", VarAccess::Use);
    flow(&mut cpg, s, m, first);
    flow(&mut cpg, m, t, second);
    (cpg, vec![s, m, t])
}

/// CP07: Use `u` and Def `d` share owner and line, Def `other` shares the line but not the owner;
/// then `d` -WEAK-> `weak` and `d` -EXACT-> `exact`, both in `c.py`. Returns [u, d, other, weak, exact].
pub(super) fn fences() -> (CodePropertyGraph, Vec<NodeIndex>) {
    let mut cpg = CodePropertyGraph::empty();
    let u = var_in(&mut cpg, ("o.py", "f", 1), 2, "x", VarAccess::Use);
    let d = var_in(&mut cpg, ("o.py", "f", 1), 2, "y", VarAccess::Def);
    let other = var_in(&mut cpg, ("o.py", "g", 2), 2, "z", VarAccess::Def);
    let weak = var(&mut cpg, "c.py", 5, "w", VarAccess::Use);
    let exact = var(&mut cpg, "c.py", 6, "e", VarAccess::Use);
    flow(&mut cpg, d, weak, WEAK);
    flow(&mut cpg, d, exact, EXACT);
    (cpg, vec![u, d, other, weak, exact])
}

/// Every hand-built fixture above, with its walk's start node (the legacy-parity inputs of CP08).
pub(super) fn fixtures() -> Vec<(CodePropertyGraph, NodeIndex)> {
    let label_sets: [&[FlowConfidence]; 4] = [&[EXACT], &[WEAK], &[EXACT, WEAK], &[WEAK, EXACT]];
    let mut all = Vec::new();
    for labels in label_sets {
        let (cpg, nodes) = pair("o.py", "c.py", labels);
        all.push((cpg, nodes[0]));
    }
    for (include_m, reversed) in [(true, false), (true, true), (false, false), (false, true)] {
        let (cpg, nodes) = detour(include_m, reversed);
        all.push((cpg, nodes[0]));
    }
    for (first, second) in [(WEAK, EXACT), (EXACT, WEAK), (WEAK, WEAK)] {
        let (cpg, nodes) = chain(first, second);
        all.push((cpg, nodes[0]));
    }
    let (cpg, nodes) = fences();
    all.push((cpg, nodes[0]));
    all
}

#[test]
fn ordinary_policy_gates_only_context_entry() {
    // (from file, to file, the edge enters a context file from another file)
    let rows = [
        ("o1.py", "o1.py", false), // same original file
        ("o1.py", "o2.py", false), // original -> other original
        ("o1.py", "c1.py", true),  // original -> context
        ("c1.py", "c1.py", false), // same context file
        ("c1.py", "c2.py", true),  // context -> other context
        ("c1.py", "o1.py", false), // context -> original
    ];
    let ctx = files(&["c1.py", "c2.py"]);
    let (mut total, mut refused) = (0usize, 0usize);
    for (from_file, to_file, enters) in rows {
        for label in [EXACT, WEAK] {
            for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
                let policy = TaintCrossingPolicy::new(&ctx, ceiling);
                let admitted = !(enters && ceiling == CrossingCeiling::Exact && label != EXACT);
                let row = format!("{from_file}->{to_file} {label:?} {ceiling:?}");
                total += 1;
                refused += usize::from(!admitted);
                assert_eq!(policy.admits(from_file, to_file, label), admitted, "{row}");
                let (cpg, n) = pair(from_file, to_file, &[label]);
                let w = walk(&cpg, n[0], Some(&policy));
                assert_eq!(reached(&cpg, &w, n[1]), admitted, "{row}");
                if admitted {
                    assert_eq!(w.1[&n[1]], label, "{row}");
                    assert_path(&cpg, &w.2, n[1], &[(n[0], n[1], label)], false);
                } else {
                    assert!(w.2.is_empty(), "{row}");
                }
            }
        }
    }
    assert_eq!((total, refused), (24, 2));
}

#[test]
fn ordinary_policy_filters_parallel_edges_before_parent_selection() {
    let ctx = files(&["c.py"]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    let lenient = TaintCrossingPolicy::new(&ctx, CrossingCeiling::NameOnly);
    // Both insertion orders: a node's out-edges are visited in one fixed order, so one of them
    // lets the legacy walk pick the NameOnly edge; the policy must not depend on which.
    for labels in [[EXACT, WEAK], [WEAK, EXACT]] {
        let (cpg, n) = pair("o.py", "c.py", &labels);
        let w = walk(&cpg, n[0], Some(&strict));
        assert!(reached(&cpg, &w, n[1]), "{labels:?}");
        assert_eq!(w.1[&n[1]], EXACT, "{labels:?}");
        assert_path(&cpg, &w.2, n[1], &[(n[0], n[1], EXACT)], false);
        // The NameOnly ceiling is exactly the policy-free walk, whichever edge legacy picks.
        let legacy = format!("{:?}", walk(&cpg, n[0], None));
        assert_eq!(format!("{:?}", walk(&cpg, n[0], Some(&lenient))), legacy);
    }
    // Control: an Exact-only pair is followed under both ceilings.
    let (cpg, n) = pair("o.py", "c.py", &[EXACT]);
    for policy in [&strict, &lenient] {
        let w = walk(&cpg, n[0], Some(policy));
        assert_path(&cpg, &w.2, n[1], &[(n[0], n[1], EXACT)], false);
    }
    // Control: a NameOnly-only pair is refused under Exact and followed under NameOnly.
    let (cpg, n) = pair("o.py", "c.py", &[WEAK]);
    let refused = walk(&cpg, n[0], Some(&strict));
    assert!(!reached(&cpg, &refused, n[1]));
    assert!(refused.2.is_empty());
    let kept = walk(&cpg, n[0], Some(&lenient));
    assert_path(&cpg, &kept.2, n[1], &[(n[0], n[1], WEAK)], false);
}

#[test]
fn ordinary_policy_keeps_allowed_route_after_denied_short_route() {
    let ctx = files(&["c.py"]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    for reversed in [false, true] {
        let (cpg, n) = detour(true, reversed);
        let (s, m, t, k) = (n[0], n[1], n[2], n[3]);
        // Precondition: with no policy the short route through the denied edge discovers T.
        let legacy = walk(&cpg, s, None);
        assert_path(&cpg, &legacy.2, t, &[(s, t, WEAK)], false);
        let w = walk(&cpg, s, Some(&strict));
        assert!(
            reached(&cpg, &w, t) && reached(&cpg, &w, k),
            "reversed={reversed}"
        );
        let route = [(s, m, EXACT), (m, t, EXACT)];
        assert_path(&cpg, &w.2, t, &route, false);
        let onward = [(s, m, EXACT), (m, t, EXACT), (t, k, EXACT)];
        assert_path(&cpg, &w.2, k, &onward, false);
    }
    // Control: without the allowed route the denied edge leaves T and its descendant unreached.
    for reversed in [false, true] {
        let (cpg, n) = detour(false, reversed);
        assert!(
            reached(&cpg, &walk(&cpg, n[0], None), n[3]),
            "reversed={reversed}"
        );
        let w = walk(&cpg, n[0], Some(&strict));
        assert!(!reached(&cpg, &w, n[2]) && !reached(&cpg, &w, n[3]));
        assert!(w.2.is_empty());
    }
}

#[test]
fn ordinary_policy_uses_current_edge_not_accumulated_label() {
    let ctx = files(&["c.py"]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    let lenient = TaintCrossingPolicy::new(&ctx, CrossingCeiling::NameOnly);
    // A NameOnly hop inside the original file does not taint the Exact context entry after it.
    let (cpg, n) = chain(WEAK, EXACT);
    let w = walk(&cpg, n[0], Some(&strict));
    assert_path(
        &cpg,
        &w.2,
        n[2],
        &[(n[0], n[1], WEAK), (n[1], n[2], EXACT)],
        false,
    );
    assert_eq!((w.1[&n[1]], w.1[&n[2]]), (WEAK, WEAK));
    // The entry's own label is what counts: a NameOnly entry is refused, its predecessor kept.
    for (first, second) in [(WEAK, WEAK), (EXACT, WEAK)] {
        let (cpg, n) = chain(first, second);
        let w = walk(&cpg, n[0], Some(&strict));
        assert!(
            reached(&cpg, &w, n[1]) && !reached(&cpg, &w, n[2]),
            "{first:?} {second:?}"
        );
    }
    let (cpg, n) = chain(WEAK, WEAK);
    assert!(reached(&cpg, &walk(&cpg, n[0], Some(&lenient)), n[2]));
}

#[test]
fn ordinary_policy_preserves_assignment_and_owner_fences() {
    let ctx = files(&["c.py"]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    let (cpg, n) = fences();
    let (u, d, other, weak, exact) = (n[0], n[1], n[2], n[3], n[4]);
    let legacy = walk(&cpg, u, None);
    let gated = walk(&cpg, u, Some(&strict));
    // Same-file assignment propagation and its owner fence hold with and without a policy.
    for w in [&legacy, &gated] {
        assert!(reached(&cpg, w, d));
        assert_path(&cpg, &w.2, d, &[], true);
        assert!(!reached(&cpg, w, other));
        assert!(reached(&cpg, w, exact));
        assert_path(&cpg, &w.2, exact, &[(d, exact, EXACT)], true);
    }
    // A context entry after the local propagation is still gated.
    assert!(reached(&cpg, &legacy, weak));
    assert_path(&cpg, &legacy.2, weak, &[(d, weak, WEAK)], true);
    assert!(!reached(&cpg, &gated, weak));
}

#[test]
fn ordinary_policy_public_dfg_entry_uses_same_gate() {
    let ctx = files(&["c.py"]);
    for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
        let policy = TaintCrossingPolicy::new(&ctx, ceiling);
        for (label, admitted) in [(EXACT, true), (WEAK, ceiling == CrossingCeiling::NameOnly)] {
            let (cpg, n) = pair("o.py", "c.py", &[label]);
            let public = cpg.dfg_forward_reachable_with_policy(&loc(&cpg, n[0]), &policy);
            assert_eq!(public, walk(&cpg, n[0], Some(&policy)).0);
            assert_eq!(
                public.contains(&loc(&cpg, n[1])),
                admitted,
                "{label:?} {ceiling:?}"
            );
        }
    }
}
