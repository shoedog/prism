//! Crossing-policy tests through the CFG adapter, the no-CFG fallback and the legacy entry points
//! (#602 S3a.2): CP05, CP06 and CP08.
//!
//! The graph is a real parsed C function. The "controlled cross-file edges" are added by hand: one
//! Exact and one NameOnly edge into a context file, and one NameOnly edge into another original
//! file, all from the Def on the source line.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::NodeIndex;

use super::taint_policy_tests::{files, fixtures, flow, hops_of, loc, var, EXACT, WEAK};
use super::{CodePropertyGraph, CpgEdge, CpgNode, CrossingCeiling, TaintCrossingPolicy, VarAccess};
use crate::ast::ParsedFile;
use crate::data_flow::{FlowPath, VarLocation};
use crate::finding_confidence::EvidencePath;
use crate::languages::Language;

const SRC: &str = "test.c";
const CTX: &str = "ctx.c";
const OTHER: &str = "other.c";

type Taint = (
    Vec<FlowPath>,
    BTreeMap<(VarLocation, VarLocation), EvidencePath>,
);

/// A parsed function: line 3 defines `x`, line 4 returns, line 5 is dead code reading `x`.
fn real_cpg() -> (CodePropertyGraph, NodeIndex) {
    let source = "\nvoid f(char* input) {\n    char* x = input;\n    return;\n    exec(x);\n}\n";
    let parsed = ParsedFile::parse(SRC, source, Language::C).unwrap();
    let mut parsed_files = BTreeMap::new();
    parsed_files.insert(SRC.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&parsed_files);
    let is_def = |idx: &NodeIndex| {
        matches!(
            cpg.node(*idx),
            CpgNode::Variable {
                access: VarAccess::Def,
                ..
            }
        )
    };
    let src = cpg.nodes_at(SRC, 3).into_iter().find(is_def);
    (cpg, src.expect("a Def on the source line"))
}

/// Controlled cross-file edges from `src`: [Exact into context, NameOnly into context,
/// NameOnly into another original file].
fn inject(cpg: &mut CodePropertyGraph, src: NodeIndex) -> [NodeIndex; 3] {
    [(CTX, 10, EXACT), (CTX, 11, WEAK), (OTHER, 12, WEAK)].map(|(file, line, label)| {
        let node = var(cpg, file, line, "e", VarAccess::Use);
        flow(cpg, src, node, label);
        node
    })
}

fn without_cfg(cpg: &mut CodePropertyGraph) {
    cpg.graph
        .retain_edges(|graph, edge| !matches!(graph[edge], CpgEdge::ControlFlow));
}

fn key(cpg: &CodePropertyGraph, node: NodeIndex) -> (String, usize) {
    (cpg.node(node).file().to_string(), cpg.node(node).line())
}

fn target_lines(result: &Taint) -> BTreeSet<(String, usize)> {
    let targets = |path: &FlowPath| {
        path.edges
            .iter()
            .map(|edge| (edge.to.file.clone(), edge.to.line))
            .collect::<Vec<_>>()
    };
    result.0.iter().flat_map(targets).collect()
}

/// The policy-aware CFG adapter on the source line: under the Exact ceiling exactly the NameOnly
/// context entry disappears and the Exact one keeps its one-hop evidence; under the NameOnly
/// ceiling the result is the legacy result. Returns the legacy target lines.
fn check_entries(
    cpg: &CodePropertyGraph,
    src: NodeIndex,
    [exact_entry, weak_entry, other_entry]: [NodeIndex; 3],
) -> BTreeSet<(String, usize)> {
    let sources = [(SRC.to_string(), 3)];
    let ctx = files(&[CTX]);
    let legacy = cpg.taint_forward_cfg_labeled(&sources);
    let legacy_lines = target_lines(&legacy);
    for entry in [exact_entry, weak_entry, other_entry] {
        assert!(
            legacy_lines.contains(&key(cpg, entry)),
            "legacy reaches {entry:?}"
        );
    }
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    let gated = cpg.taint_forward_cfg_labeled_with_policy(&sources, &strict);
    let mut want = legacy_lines.clone();
    want.remove(&key(cpg, weak_entry));
    assert_eq!(target_lines(&gated), want);
    let from = loc(cpg, src);
    let evidence = &gated.1[&(from.clone(), loc(cpg, exact_entry))];
    assert_eq!(
        hops_of(evidence),
        [(from.clone(), loc(cpg, exact_entry), EXACT)]
    );
    assert!(!evidence.crossed_unlabeled);
    assert!(!gated.1.contains_key(&(from.clone(), loc(cpg, weak_entry))));
    assert!(gated.1.contains_key(&(from, loc(cpg, other_entry))));
    let lenient = TaintCrossingPolicy::new(&ctx, CrossingCeiling::NameOnly);
    let lenient = cpg.taint_forward_cfg_labeled_with_policy(&sources, &lenient);
    assert_eq!(format!("{lenient:?}"), format!("{legacy:?}"));
    legacy_lines
}

#[test]
fn ordinary_policy_cfg_adapter_forwards_and_still_prunes() {
    let (mut cpg, src) = real_cpg();
    let injected = inject(&mut cpg, src);
    assert!(cpg.has_cfg_edges());
    // Preconditions: the data flow alone reaches the dead line; the legacy CFG walk prunes it.
    let dead = (SRC.to_string(), 5);
    let dfg_only = cpg.taint_forward_labeled(&[(SRC.to_string(), 3)]);
    assert!(
        target_lines(&dfg_only).contains(&dead),
        "fixture precondition: the DFG-only walk must reach the dead line"
    );
    let legacy_lines = check_entries(&cpg, src, injected);
    assert!(!legacy_lines.contains(&dead));
}

#[test]
fn ordinary_policy_no_cfg_fallback_forwards() {
    let (mut cpg, src) = real_cpg();
    let injected = inject(&mut cpg, src);
    without_cfg(&mut cpg);
    let sources = [(SRC.to_string(), 3)];
    assert!(!cpg.has_cfg_edges());
    assert!(!target_lines(&cpg.taint_forward_cfg_labeled(&sources)).is_empty());
    check_entries(&cpg, src, injected);
    // The fallback and the public DFG sibling agree on the source node's reachable set.
    let from = loc(&cpg, src);
    let ctx = files(&[CTX]);
    for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
        let policy = TaintCrossingPolicy::new(&ctx, ceiling);
        let (_, evidence) = cpg.taint_forward_cfg_labeled_with_policy(&sources, &policy);
        let from_adapter: BTreeSet<VarLocation> = evidence
            .keys()
            .filter(|(start, _)| *start == from)
            .map(|(_, target)| target.clone())
            .collect();
        assert_eq!(
            from_adapter,
            cpg.dfg_forward_reachable_with_policy(&from, &policy)
        );
    }
}

/// The old entry points equal the policy-free internal path, and an empty context set equals it
/// under either ceiling: sets, label maps, FlowPath order and the evidence Debug text.
fn assert_parity(cpg: &CodePropertyGraph, start: NodeIndex, empty: &BTreeSet<String>) {
    let from = loc(cpg, start);
    let sources = [(cpg.node(start).file().to_string(), cpg.node(start).line())];
    let internal = cpg.dfg_forward_reachable_with_evidence_policy(&from, None);
    assert_eq!(cpg.dfg_forward_reachable(&from), internal.0);
    let labeled = (internal.0.clone(), internal.1.clone());
    assert_eq!(cpg.dfg_forward_reachable_labeled(&from), labeled);
    let legacy = format!("{:?}", cpg.taint_forward_cfg_labeled(&sources));
    assert_eq!(
        legacy,
        format!("{:?}", cpg.taint_forward_cfg_labeled_policy(&sources, None))
    );
    if !cpg.has_cfg_edges() {
        assert_eq!(legacy, format!("{:?}", cpg.taint_forward_labeled(&sources)));
    }
    for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
        let policy = TaintCrossingPolicy::new(empty, ceiling);
        let walked = cpg.dfg_forward_reachable_with_evidence_policy(&from, Some(&policy));
        assert_eq!(format!("{walked:?}"), format!("{internal:?}"));
        let taint = cpg.taint_forward_cfg_labeled_with_policy(&sources, &policy);
        assert_eq!(format!("{taint:?}"), legacy);
    }
}

#[test]
fn ordinary_policy_absent_preserves_all_legacy_outputs() {
    let empty = BTreeSet::new();
    // Hand-built positive, negative, parallel-order, detour, chain and assignment fixtures.
    for (cpg, start) in fixtures() {
        assert_parity(&cpg, start, &empty);
    }
    // A real parsed function with controlled entries, with and without its CFG edges.
    let (mut cpg, src) = real_cpg();
    inject(&mut cpg, src);
    assert!(cpg.has_cfg_edges());
    assert_parity(&cpg, src, &empty);
    without_cfg(&mut cpg);
    assert_parity(&cpg, src, &empty);
    // A nonexistent source location is the same empty result with and without a policy.
    let ctx = files(&[CTX]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    let mut missing = loc(&cpg, src);
    missing.file = "missing.c".to_string();
    let gone = cpg.dfg_forward_reachable_with_evidence_policy(&missing, None);
    assert!(gone.0.is_empty() && gone.1.is_empty() && gone.2.is_empty());
    let gated = cpg.dfg_forward_reachable_with_evidence_policy(&missing, Some(&strict));
    assert_eq!(format!("{gated:?}"), format!("{gone:?}"));
    let nothing =
        cpg.taint_forward_cfg_labeled_with_policy(&[("missing.c".to_string(), 99)], &strict);
    assert!(nothing.0.is_empty() && nothing.1.is_empty());
}
