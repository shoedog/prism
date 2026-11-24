//! Return-flow walker crossing-policy tests (#602 S3b-C): RF01-RF04, and the shared fixtures of
//! RF05-RF08 (`taint_policy_return_parity_tests`).
//!
//! The fixtures are REAL parsed Python programs built by the ordinary graph builder, so call
//! resolution, the argument -> parameter DataFlow edge and the return-flow edge all come from the
//! producer. A test then CONTROLS the graph: it relabels or adds DataFlow edges on the
//! argument -> parameter pair, or (RF04c only) injects one ReturnFlow edge. Those edits are
//! synthetic and are marked CONTROLLED where they happen. Every fixture asserts its own
//! preconditions first, with a message starting "inadmissible precondition:".

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

use super::taint_policy_tests::{files, loc, EXACT, WEAK};
use super::{
    BoundaryEdge, BoundaryKind, CodePropertyGraph, CpgEdge, CpgNode, CrossingCeiling,
    FlowConfidence, Relation, ReturnFlowMode, TaintCrossingPolicy, Trace, VarAccess,
};
use crate::ast::ParsedFile;
use crate::finding_confidence::{EvidenceHop, EvidencePath};
use crate::languages::Language;

pub(super) const MAIN: &str = "main.py";
pub(super) const LIB: &str = "lib.py";

/// `run(user)` calls `helper(user)` on line 3 and uses the result on line 4.
pub(super) const MAIN_SRC: &str =
    "from lib import helper\ndef run(user):\n    value = helper(user)\n    sink(value)\n";
pub(super) const LIB_SRC: &str = "def helper(p):\n    q = p\n    return q\n";
/// `MAIN_SRC` plus `stray`, which never calls `helper` (RF04c injects an edge into it).
pub(super) const MAIN_STRAY_SRC: &str = "from lib import helper\ndef run(user):\n    value = helper(user)\n    sink(value)\ndef stray(z):\n    w = z\n";
/// The first call's argument is a Use of `user` (line 3); the second is a Use of `y` (line 5).
pub(super) const MAIN_TWICE_SRC: &str = "from lib import helper\ndef run(user):\n    value = helper(user)\n    y = user\n    other = helper(y)\n    sink(value, other)\n";
pub(super) const MAIN_ONCE_SRC: &str = "from lib import helper\ndef run(user):\n    value = helper(user)\n    y = user\n    sink(value, y)\n";

/// (entry label, ceiling, the descent is admitted).
pub(super) const ROWS: [(FlowConfidence, CrossingCeiling, bool); 4] = [
    (EXACT, CrossingCeiling::Exact, true),
    (EXACT, CrossingCeiling::NameOnly, true),
    (WEAK, CrossingCeiling::Exact, false),
    (WEAK, CrossingCeiling::NameOnly, true),
];

pub(super) fn build(programs: &[(&str, &str)]) -> CodePropertyGraph {
    let mut parsed = BTreeMap::new();
    for &(path, source) in programs {
        let file = ParsedFile::parse(path, source, Language::Python).unwrap();
        parsed.insert(path.to_string(), file);
    }
    CodePropertyGraph::build(&parsed)
}

pub(super) fn sources(file: &str, line: usize) -> Vec<(String, usize)> {
    vec![(file.to_string(), line)]
}

pub(super) fn dbg<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

pub(super) fn var_at(
    cpg: &CodePropertyGraph,
    file: &str,
    line: usize,
    name: &str,
    access: VarAccess,
) -> NodeIndex {
    let found: Vec<NodeIndex> = cpg
        .nodes_at(file, line)
        .into_iter()
        .filter(|&node| {
            matches!(
                cpg.node(node),
                CpgNode::Variable { path, access: a, .. }
                    if *a == access && path.to_string() == name
            )
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "inadmissible precondition: one {access:?} `{name}` at {file}:{line}, got {found:?}"
    );
    found[0]
}

pub(super) fn entry_labels(
    cpg: &CodePropertyGraph,
    from: NodeIndex,
    to: NodeIndex,
) -> Vec<FlowConfidence> {
    cpg.graph
        .edges(from)
        .filter(|edge| edge.target() == to)
        .filter_map(|edge| match edge.weight() {
            CpgEdge::DataFlow(label) => Some(*label),
            _ => None,
        })
        .collect()
}

/// CONTROLLED: relabel every DataFlow edge `from -> to`.
pub(super) fn relabel(
    cpg: &mut CodePropertyGraph,
    from: NodeIndex,
    to: NodeIndex,
    label: FlowConfidence,
) {
    let ids: Vec<_> = cpg
        .graph
        .edges(from)
        .filter(|edge| edge.target() == to && matches!(edge.weight(), CpgEdge::DataFlow(_)))
        .map(|edge| edge.id())
        .collect();
    assert!(
        !ids.is_empty(),
        "inadmissible precondition: a DataFlow edge {from:?} -> {to:?} to relabel"
    );
    for id in ids {
        cpg.graph[id] = CpgEdge::DataFlow(label);
    }
}

/// CONTROLLED: one more parallel DataFlow edge.
fn add_parallel(
    cpg: &mut CodePropertyGraph,
    from: NodeIndex,
    to: NodeIndex,
    label: FlowConfidence,
) {
    cpg.graph.add_edge(from, to, CpgEdge::DataFlow(label));
}

pub(super) fn without_cfg(cpg: &mut CodePropertyGraph) {
    cpg.graph
        .retain_edges(|graph, edge| !matches!(graph[edge], CpgEdge::ControlFlow));
}

pub(super) fn trace(
    cpg: &CodePropertyGraph,
    sources: &[(String, usize)],
    policy: Option<&TaintCrossingPolicy<'_>>,
) -> Trace {
    cpg.taint_trace_with_mode_policy(sources, ReturnFlowMode::On, policy)
}

pub(super) fn reached(trace: &Trace, root: NodeIndex, node: NodeIndex) -> bool {
    trace
        .frontier_by_root
        .get(&root)
        .is_some_and(|frontier| frontier.contains(&node))
}

pub(super) fn reached_in_file(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    root: NodeIndex,
    file: &str,
) -> bool {
    trace
        .frontier_by_root
        .get(&root)
        .is_some_and(|frontier| frontier.iter().any(|&n| cpg.node(n).file() == file))
}

pub(super) fn refused(trace: &Trace, root: NodeIndex, from: NodeIndex, to: NodeIndex) -> bool {
    trace.boundary.contains(&BoundaryEdge {
        root,
        from,
        to,
        kind: BoundaryKind::CrossFunction,
    })
}

pub(super) fn parent(
    trace: &Trace,
    root: NodeIndex,
    node: NodeIndex,
) -> Option<(NodeIndex, Relation)> {
    trace.parents_by_root.get(&(root, node)).copied()
}

/// The witness hops `(parent, child, relation)` from `node` back to `root`.
pub(super) fn chain(
    trace: &Trace,
    root: NodeIndex,
    node: NodeIndex,
) -> Vec<(NodeIndex, NodeIndex, Relation)> {
    let mut hops = Vec::new();
    let mut current = node;
    while current != root {
        let &(from, relation) = trace
            .parents_by_root
            .get(&(root, current))
            .expect("a witnessed parent");
        hops.push((from, current, relation));
        current = from;
    }
    hops
}

/// The part of `root`'s frontier a walk reaches without any call descent or return ascent: the
/// oracle for "the caller-side frontier is unchanged" when a descent is refused.
pub(super) fn without_crossing(trace: &Trace, root: NodeIndex) -> BTreeSet<NodeIndex> {
    let crosses = |mut node: NodeIndex| {
        while node != root {
            match trace.parents_by_root.get(&(root, node)) {
                Some(&(_, Relation::CallDescent | Relation::ReturnFlow)) | None => return true,
                Some(&(from, _)) => node = from,
            }
        }
        false
    };
    trace.frontier_by_root[&root]
        .iter()
        .copied()
        .filter(|&node| !crosses(node))
        .collect()
}

/// The single-call pair: nodes by role, located in the built graph.
pub(super) struct Pair {
    pub(super) cpg: CodePropertyGraph,
    pub(super) arg: NodeIndex,    // Use `user`, main.py:3
    pub(super) param: NodeIndex,  // Def `p`, lib.py:1
    pub(super) body: NodeIndex,   // Use `p`, lib.py:2
    pub(super) ret: NodeIndex,    // Use `q`, lib.py:3
    pub(super) result: NodeIndex, // Def `value`, main.py:3
}

impl Pair {
    /// Preconditions of the pristine fixture for the seed `s` whose root is `root`: the entry is
    /// ONE Exact DataFlow edge, and the policy-free return-flow trace really descends into the
    /// callee file, reaches a callee body node and ascends to the caller's result.
    pub(super) fn check(&self, s: &[(String, usize)], root: NodeIndex) {
        let (cpg, arg, param) = (&self.cpg, self.arg, self.param);
        assert_eq!(
            entry_labels(cpg, arg, param),
            vec![EXACT],
            "inadmissible precondition: the argument -> parameter edge is one Exact DataFlow edge"
        );
        assert_ne!(cpg.node(arg).file(), cpg.node(param).file());
        let free = trace(cpg, s, None);
        assert_eq!(
            parent(&free, root, param),
            Some((arg, Relation::CallDescent)),
            "inadmissible precondition: the policy-free trace descends through the call"
        );
        assert!(
            reached(&free, root, self.body),
            "inadmissible precondition: the callee body is reached"
        );
        assert_eq!(
            parent(&free, root, self.result),
            Some((self.ret, Relation::ReturnFlow)),
            "inadmissible precondition: the trace ascends back to the caller's result"
        );
    }
}

pub(super) fn pair_with(main_src: &str) -> Pair {
    let cpg = build(&[(MAIN, main_src), (LIB, LIB_SRC)]);
    let at = |file, line, name, access| var_at(&cpg, file, line, name, access);
    let found = Pair {
        arg: at(MAIN, 3, "user", VarAccess::Use),
        param: at(LIB, 1, "p", VarAccess::Def),
        body: at(LIB, 2, "p", VarAccess::Use),
        ret: at(LIB, 3, "q", VarAccess::Use),
        result: at(MAIN, 3, "value", VarAccess::Def),
        cpg,
    };
    found.check(&sources(MAIN, 3), found.arg);
    found
}

pub(super) fn pair() -> Pair {
    pair_with(MAIN_SRC)
}

#[test]
fn return_policy_gates_descent_into_context() {
    let ctx = files(&[LIB]);
    let s = sources(MAIN, 3);
    for (entry, ceiling, admitted) in ROWS {
        let mut p = pair();
        relabel(&mut p.cpg, p.arg, p.param, entry); // CONTROLLED
        let policy = TaintCrossingPolicy::new(&ctx, ceiling);
        let row = format!("{entry:?} {ceiling:?}");
        let free = trace(&p.cpg, &s, None);
        let gated = trace(&p.cpg, &s, Some(&policy));
        // The label never decides the policy-free walk: it descends whatever the entry label.
        assert!(reached(&free, p.arg, p.param), "{row}");
        assert!(!refused(&free, p.arg, p.arg, p.param), "{row}");
        assert_eq!(reached(&gated, p.arg, p.param), admitted, "{row}");
        if admitted {
            assert!(reached(&gated, p.arg, p.body), "{row}");
            assert!(reached(&gated, p.arg, p.result), "{row}");
            assert!(!refused(&gated, p.arg, p.arg, p.param), "{row}");
            assert_eq!(dbg(&gated), dbg(&free), "{row}");
            // The descent stays an unlabeled bridge: never an Exact hop.
            let path = EvidencePath::from_trace(&gated, p.arg, p.body);
            assert!(path.crossed_unlabeled, "{row}");
            let (from, to) = (loc(&p.cpg, p.arg), loc(&p.cpg, p.param));
            let entry_hop = |hop: &EvidenceHop| matches!(hop, EvidenceHop::DataFlow { from: f, to: t, .. } if *f == from && *t == to);
            assert!(!path.hops.iter().any(entry_hop), "{row}");
        } else {
            assert!(!reached_in_file(&p.cpg, &gated, p.arg, LIB), "{row}");
            assert!(!reached(&gated, p.arg, p.result), "{row}");
            // The refused descent is the existing CrossFunction boundary, never silently dropped.
            assert!(refused(&gated, p.arg, p.arg, p.param), "{row}");
            // The caller-side frontier is the policy-free one minus everything past the descent.
            let expected = without_crossing(&free, p.arg);
            assert_eq!(gated.frontier_by_root[&p.arg], expected, "{row}");
        }
    }
}

#[test]
fn return_policy_admits_parallel_exact_edge_in_either_order() {
    let ctx = files(&[LIB]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    let lenient = TaintCrossingPolicy::new(&ctx, CrossingCeiling::NameOnly);
    let s = sources(MAIN, 3);
    for labels in [[EXACT, WEAK], [WEAK, EXACT]] {
        let mut p = pair();
        relabel(&mut p.cpg, p.arg, p.param, labels[0]); // CONTROLLED
        add_parallel(&mut p.cpg, p.arg, p.param, labels[1]); // CONTROLLED
        assert_eq!(entry_labels(&p.cpg, p.arg, p.param).len(), 2, "{labels:?}");
        let cpg = &p.cpg;
        let label = |policy| cpg.admitted_data_flow_label(policy, p.arg, p.param);
        // One admitted parallel edge is enough, and the worst is taken over the admitted ones.
        assert_eq!(label(Some(&strict)), Some(EXACT), "{labels:?}");
        assert_eq!(label(Some(&lenient)), Some(WEAK), "{labels:?}");
        assert_eq!(label(None), Some(WEAK), "{labels:?}");
        let gated = trace(cpg, &s, Some(&strict));
        assert!(reached(&gated, p.arg, p.param), "{labels:?}");
        assert!(!refused(&gated, p.arg, p.arg, p.param), "{labels:?}");
        assert_eq!(dbg(&gated), dbg(&trace(cpg, &s, None)), "{labels:?}");
    }
    // A NameOnly-only pair has no admitted edge under Exact, and its label under NameOnly.
    let mut p = pair();
    relabel(&mut p.cpg, p.arg, p.param, WEAK); // CONTROLLED
    let cpg = &p.cpg;
    assert_eq!(
        cpg.admitted_data_flow_label(Some(&strict), p.arg, p.param),
        None
    );
    let lenient_label = cpg.admitted_data_flow_label(Some(&lenient), p.arg, p.param);
    assert_eq!(lenient_label, Some(WEAK));
    assert_eq!(
        cpg.admitted_data_flow_label(None, p.arg, p.param),
        Some(WEAK)
    );
    // The gate is the same predicate: a context set that is not the target never refuses.
    let elsewhere = files(&[MAIN]);
    let elsewhere = TaintCrossingPolicy::new(&elsewhere, CrossingCeiling::Exact);
    let label = cpg.admitted_data_flow_label(Some(&elsewhere), p.arg, p.param);
    assert_eq!(label, Some(WEAK));
    // A hop inside the context file is never refused, whatever its label.
    let mut p = pair();
    relabel(&mut p.cpg, p.param, p.body, WEAK); // CONTROLLED
    let label = p
        .cpg
        .admitted_data_flow_label(Some(&strict), p.param, p.body);
    assert_eq!(label, Some(WEAK));
    // No DataFlow edge, no label: no edge at all, and a ReturnFlow edge only.
    let cpg = &p.cpg;
    assert_eq!(cpg.admitted_data_flow_label(None, p.arg, p.result), None);
    assert!(cpg.graph.edges(p.ret).any(
        |edge| edge.target() == p.result && matches!(edge.weight(), CpgEdge::ReturnFlow { .. })
    ));
    assert_eq!(cpg.admitted_data_flow_label(None, p.ret, p.result), None);
}

#[test]
fn return_policy_keeps_allowed_route_after_refused_descent() {
    let ctx = files(&[LIB]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    // The signature line seeds the parameter itself: a degraded, pure-taint seed.
    let s = sources(MAIN, 2);
    let located = |cpg: &CodePropertyGraph| {
        let at = |file, line, name, access| var_at(cpg, file, line, name, access);
        (
            at(MAIN, 2, "user", VarAccess::Def),
            at(MAIN, 3, "user", VarAccess::Use),
            at(LIB, 1, "p", VarAccess::Def),
            at(LIB, 2, "p", VarAccess::Use),
        )
    };
    let mut cpg = build(&[(MAIN, MAIN_TWICE_SRC), (LIB, LIB_SRC)]);
    let (root, first, param, body) = located(&cpg);
    let later = var_at(&cpg, MAIN, 5, "y", VarAccess::Use);
    for arg in [first, later] {
        assert_eq!(
            entry_labels(&cpg, arg, param),
            vec![EXACT],
            "inadmissible precondition: Exact entry from {arg:?}"
        );
    }
    relabel(&mut cpg, first, param, WEAK); // CONTROLLED: only the first call's entry is NameOnly
    let free = trace(&cpg, &s, None);
    assert!(
        free.degraded,
        "inadmissible precondition: a signature-line seed degrades to pure taint"
    );
    assert_eq!(
        parent(&free, root, param),
        Some((first, Relation::CallDescent)),
        "inadmissible precondition: without a policy the first call's argument is the parent"
    );
    let gated = trace(&cpg, &s, Some(&strict));
    // The refused hop is recorded; the later Exact call still reaches the parameter and the body.
    assert!(refused(&gated, root, first, param));
    assert!(reached(&gated, root, first));
    assert_eq!(
        parent(&gated, root, param),
        Some((later, Relation::CallDescent))
    );
    assert!(reached(&gated, root, body));
    let route = chain(&gated, root, body);
    assert!(route.contains(&(later, param, Relation::CallDescent)));
    assert!(!route
        .iter()
        .any(|&(from, to, _)| (from, to) == (first, param)));
    // Denied-only control: with the second call removed the callee is out of reach under Exact.
    let mut cpg = build(&[(MAIN, MAIN_ONCE_SRC), (LIB, LIB_SRC)]);
    let (root, first, param, body) = located(&cpg);
    relabel(&mut cpg, first, param, WEAK); // CONTROLLED
    let free = trace(&cpg, &s, None);
    assert!(reached(&free, root, body), "inadmissible precondition");
    let gated = trace(&cpg, &s, Some(&strict));
    assert!(!reached_in_file(&cpg, &gated, root, LIB));
    assert!(refused(&gated, root, first, param));
}

#[test]
fn return_policy_keeps_singleton_exact_ascent_under_both_ceilings() {
    // (a) A context callee returning to an original caller: the ascent lands in an original file
    // and is never constrained. Only the descent entry (Exact here) is tested.
    let p = pair();
    let s = sources(MAIN, 3);
    let free = trace(&p.cpg, &s, None);
    let ctx = files(&[LIB]);
    for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
        let policy = TaintCrossingPolicy::new(&ctx, ceiling);
        let gated = trace(&p.cpg, &s, Some(&policy));
        let ascent = Some((p.ret, Relation::ReturnFlow));
        assert_eq!(parent(&gated, p.arg, p.result), ascent, "{ceiling:?}");
        assert_eq!(dbg(&gated), dbg(&free), "{ceiling:?}");
    }
    // (b) An original callee, a context caller, the seed in the callee: the ascent INTO the
    // context file is taken under both ceilings exactly as without a policy.
    let s = sources(LIB, 2);
    let free = trace(&p.cpg, &s, None);
    let ascended = |trace: &Trace| {
        trace
            .parents_by_root
            .iter()
            .any(|(&(_, child), &(from, rel))| {
                child == p.result && from == p.ret && rel == Relation::ReturnFlow
            })
    };
    assert!(
        ascended(&free),
        "inadmissible precondition: the callee-seeded trace ascends to the caller"
    );
    let ctx = files(&[MAIN]);
    for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
        let policy = TaintCrossingPolicy::new(&ctx, ceiling);
        let gated = trace(&p.cpg, &s, Some(&policy));
        assert!(ascended(&gated), "{ceiling:?}");
        assert_eq!(dbg(&gated), dbg(&free), "{ceiling:?}");
    }
    // (c) CONTROLLED: a ReturnFlow edge into a function that never calls the callee has no
    // singleton-Exact call proof, so it stays a boundary under either ceiling.
    let mut p = pair_with(MAIN_STRAY_SRC);
    let stray = var_at(&p.cpg, MAIN, 6, "w", VarAccess::Def);
    p.cpg.graph.add_edge(
        p.ret,
        stray,
        CpgEdge::ReturnFlow {
            suppress_shortcut: false,
        },
    );
    let free = trace(&p.cpg, &s, None);
    let bounded = |trace: &Trace| {
        trace
            .boundary
            .iter()
            .any(|b| b.from == p.ret && b.to == stray && b.kind == BoundaryKind::CrossFunction)
    };
    assert!(
        bounded(&free) && !free.in_frontier(stray),
        "inadmissible precondition: the injected edge is refused without a policy"
    );
    for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
        let policy = TaintCrossingPolicy::new(&ctx, ceiling);
        let gated = trace(&p.cpg, &s, Some(&policy));
        assert!(bounded(&gated) && !gated.in_frontier(stray), "{ceiling:?}");
        assert_eq!(dbg(&gated), dbg(&free), "{ceiling:?}");
    }
}
