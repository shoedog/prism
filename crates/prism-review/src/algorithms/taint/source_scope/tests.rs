use super::loading::project_loading_trace;
use crate::algorithms::taint::{
    slice_with_trace, TaintConfig, TaintReached, TaintRoot, TaintRootOrigin, TaintSeed, TaintTrace,
};
use petgraph::graph::NodeIndex;
use prism::{
    access_path::AccessPath,
    ast::ParsedFile,
    cpg::{CpgContext, CpgEdge, FlowConfidence, FlowDoubt},
    data_flow::{VarAccessKind, VarLocation},
    diff::{DiffInfo, DiffInput, ModifyType},
    finding_confidence::{EvidenceHop, EvidencePath},
    languages::Language,
    slice::SliceResult,
};
use std::collections::{BTreeMap, BTreeSet};

mod real;

use TaintRootOrigin::{Diff, Explicit, Framework, Ipc};

fn parse(files: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    let parse = |path: &str, source: &str| {
        ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap()
    };
    files
        .iter()
        .map(|&(path, source)| (path.to_string(), parse(path, source)))
        .collect()
}

fn diff(path: &str, lines: &[usize]) -> DiffInput {
    DiffInput {
        files: vec![DiffInfo {
            file_path: path.into(),
            modify_type: ModifyType::Modified,
            diff_lines: lines.iter().copied().collect(),
        }],
    }
}

fn snapshot(root: &TaintRoot) -> String {
    format!("{root:?}")
}

fn originals(files: &[&str]) -> BTreeSet<String> {
    files.iter().map(|file| file.to_string()).collect()
}

fn loaded(files: &[&str]) -> BTreeMap<String, ()> {
    files.iter().map(|file| (file.to_string(), ())).collect()
}

fn project_same(trace: &TaintTrace, files: &[&str]) -> TaintTrace {
    project_loading_trace(trace, &originals(files), &loaded(files))
}

fn unique_var_node(
    ctx: &CpgContext,
    file: &str,
    line: usize,
    base: &str,
    kind: VarAccessKind,
) -> NodeIndex {
    let hits: Vec<_> = (ctx.cpg.nodes_at(file, line).into_iter())
        .filter_map(|node| Some((node, ctx.cpg.to_var_location(node)?)))
        .filter(|(_, loc)| loc.path.base == base && loc.kind == kind)
        .map(|(node, _)| node)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "inadmissible precondition: {base}@{line} {kind:?}"
    );
    hits[0]
}

fn add_flow(ctx: &mut CpgContext, from: NodeIndex, to: NodeIndex, label: FlowConfidence) {
    ctx.cpg.graph.add_edge(from, to, CpgEdge::DataFlow(label));
}

fn traced(
    files: &BTreeMap<String, ParsedFile>,
    input: &DiffInput,
    config: &TaintConfig,
    wire: impl FnOnce(&mut CpgContext),
) -> (SliceResult, TaintTrace) {
    let mut ctx = CpgContext::build(files, None);
    wire(&mut ctx);
    slice_with_trace(&ctx, input, config).expect("taint trace")
}

fn loc(file: &str, line: usize, base: &str, start: usize) -> VarLocation {
    VarLocation {
        file: file.into(),
        function: "h".into(),
        function_start_line: 1,
        line,
        path: AccessPath::simple(base),
        start_byte: start,
        end_byte: start + 1,
        kind: VarAccessKind::Use,
    }
}

fn reached(line: usize, k: usize) -> TaintReached {
    let (from, to) = (loc("a.go", line, "x", 1), loc("c.go", line, "y", 100 + k));
    let hop = |label| {
        let hop = EvidenceHop::DataFlow {
            from: from.clone(),
            to: to.clone(),
            confidence: label,
        };
        EvidencePath::from_hops(vec![hop])
    };
    let evidence = match k % 3 {
        0 => hop(FlowConfidence::Exact),
        1 => hop(FlowConfidence::NameOnly(FlowDoubt::SameLine)),
        _ => EvidencePath::unlabeled_for("taint"), // a collision-refused witness
    };
    TaintReached {
        location: to,
        evidence,
    }
}

fn root(file: &str, origin: TaintRootOrigin, n: usize, count: usize) -> TaintRoot {
    TaintRoot {
        seed: TaintSeed::line(file.into(), n),
        origin,
        location: loc(file, n, "x", 10 * n),
        reached: (0..count).map(|k| reached(n, k)).collect(),
    }
}

/// 12 eligible roots (two original, loaded files) among 15 ineligible ones.
fn population() -> (TaintTrace, Vec<String>) {
    let mut roots = vec![(root("ctx.go", Diff, 200, 1), false)]; // loaded, not original
    for n in 0..12 {
        let file = ["a.go", "b.go"][n % 2];
        let origin = if n % 3 == 0 { Explicit } else { Diff };
        let mut eligible = root(file, origin, n + 1, if n == 5 { 13 } else { n % 3 });
        if n == 7 {
            eligible.location.file = "gone.go".into(); // identity is seed.file
        }
        roots.push((eligible, true));
        let other = if n % 2 == 0 { Framework } else { Ipc };
        roots.push((root(file, other, 100 + n, 2), false));
    }
    roots.push((root("gone.go", Explicit, 201, 1), false)); // original, not loaded
    let mut misattributed = root("ctx.go", Diff, 202, 1);
    misattributed.location.file = "a.go".into();
    roots.push((misattributed, false));
    let expected = roots
        .iter()
        .filter(|(_, keep)| *keep)
        .map(|(r, _)| snapshot(r))
        .collect();
    (
        TaintTrace {
            roots: roots.into_iter().map(|(r, _)| r).collect(),
        },
        expected,
    )
}

fn population_scope() -> (BTreeSet<String>, BTreeMap<String, ()>) {
    (
        originals(&["a.go", "b.go", "gone.go"]),
        loaded(&["a.go", "b.go", "ctx.go"]),
    )
}

// Metadata projection oracle; producer collision behavior is proven by the unchanged RT11-RT13.
#[test]
fn loading_projection_is_complete_and_evidence_preserving() {
    let (trace, expected) = population();
    let (original, loaded) = population_scope();
    assert_eq!(
        (trace.roots.len(), expected.len()),
        (27, 12),
        "inadmissible precondition"
    );
    let projected = project_loading_trace(&trace, &original, &loaded);
    let got: Vec<_> = projected.roots.iter().map(snapshot).collect();
    assert_eq!(
        got, expected,
        "exactly the eligible roots, in relative order"
    );
    let wide = &projected.roots[5];
    assert_eq!(wide.reached.len(), 13);
    let kinds =
        |f: fn(&EvidencePath) -> bool| wide.reached.iter().filter(|r| f(&r.evidence)).count();
    assert_eq!(
        kinds(|e| e.crossed_unlabeled && e.hops.is_empty()),
        4,
        "unlabeled witnesses"
    );
    assert_eq!(
        kinds(|e| !e.hops.is_empty() && e.hops.iter().all(EvidenceHop::is_exact)),
        5
    );
    assert_eq!(
        kinds(|e| e.hops.iter().any(|h| !h.is_exact())),
        4,
        "NameOnly hops"
    );
    assert!(projected
        .roots
        .iter()
        .all(|r| matches!(r.origin, Diff | Explicit)));
    assert!(projected
        .roots
        .iter()
        .all(|r| ["a.go", "b.go"].contains(&r.seed.file.as_str())));
    assert!(
        projected.roots.iter().any(|r| r.reached.is_empty()),
        "isolated root kept"
    );
}

#[test]
fn loading_projection_of_empty_trace_is_empty() {
    let (original, loaded) = population_scope();
    assert!(
        project_loading_trace(&TaintTrace::default(), &original, &loaded)
            .roots
            .is_empty()
    );
}

#[test]
fn loading_projection_requires_original_files() {
    let (trace, _) = population();
    let (_, loaded) = population_scope();
    assert!(project_loading_trace(&trace, &BTreeSet::new(), &loaded)
        .roots
        .is_empty());
}

#[test]
fn loading_projection_requires_loaded_files() {
    let (trace, _) = population();
    let (original, _) = population_scope();
    let none: BTreeMap<String, ()> = BTreeMap::new();
    assert!(project_loading_trace(&trace, &original, &none)
        .roots
        .is_empty());
}

#[test]
fn loading_projection_does_not_modify_its_input() {
    let (trace, _) = population();
    let (original, loaded) = population_scope();
    let before = format!("{trace:?}");
    let _ = project_loading_trace(&trace, &original, &loaded);
    assert_eq!(before, format!("{trace:?}"));
}

// Tests copy identity, not parser recall: the roots are built by hand.
#[test]
fn loading_projection_keeps_legacy_equal_occurrences_and_scopes() {
    let first = loc("app.js", 2, "x", 10);
    let second = VarLocation {
        start_byte: 20,
        end_byte: 21,
        ..first.clone()
    };
    assert_eq!(
        first, second,
        "inadmissible precondition: legacy equality ignores bytes"
    );
    let mut roots = Vec::new();
    for span in [(10, 15), (20, 25)] {
        let path = AccessPath::simple("x");
        let seed = TaintSeed::target_scoped(
            "app.js".into(),
            2,
            path,
            Some(span.0),
            Some(span),
            Some(span),
        );
        for location in [&first, &second] {
            let evidence = EvidencePath::unlabeled_for("taint");
            let reached = vec![TaintReached {
                location: location.clone(),
                evidence,
            }];
            roots.push(TaintRoot {
                seed: seed.clone(),
                origin: Diff,
                location: location.clone(),
                reached,
            });
        }
    }
    let trace = TaintTrace { roots };
    let expected: Vec<_> = trace.roots.iter().map(snapshot).collect();
    assert_eq!(
        expected.iter().collect::<BTreeSet<_>>().len(),
        4,
        "inadmissible precondition"
    );
    let projected = project_same(&trace, &["app.js"]);
    assert_eq!(
        projected.roots.iter().map(snapshot).collect::<Vec<_>>(),
        expected
    );
}
