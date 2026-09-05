//! §7.3 plumb-through parity: consumers must select the same DataFlow edge
//! set regardless of the `FlowConfidence` payload each edge carries (§3
//! non-goal 1 — labeling must never add, remove, or otherwise affect which
//! edges a traversal selects).
//!
//! The real build must contain both RD poles. This test compares it with an
//! otherwise identical build whose DataFlow payloads are all forced to
//! `NameOnly(CfgIncomplete)` through a test-only relabel seam. Because the
//! relabel only overwrites edge WEIGHTS in place (via the public
//! `CodePropertyGraph::graph` field), the two graphs have IDENTICAL
//! node/edge topology and NodeIndex/EdgeIndex spaces (pinned by
//! `tests/infra/parallel_equality_test.rs`'s determinism tests) — so any
//! difference in a consumer's output between the two graphs can only be
//! caused by that consumer's predicate being label-sensitive.

use prism::cpg::{CodePropertyGraph, CpgContext, CpgEdge, FlowConfidence, FlowDoubt};
use prism::cpg_cache::{self, CacheResult};
use prism::data_flow::{VarAccessKind, VarLocation};
use prism::diff::{DiffInfo, DiffInput, ModifyType};
use prism::languages::Language;
use prism::repo_loader::{load_repo, LoadedRepo};
use std::collections::{BTreeMap, BTreeSet};

/// Same corpus as `tests/infra/parallel_equality_test.rs:111` — a multi-file
/// subset of the repo (src/navigation, ~15 Rust files with real cross-file
/// calls and data flow) instead of the whole repo, for the same reason: a
/// whole-repo CPG build is far too slow for a gate (`tests/infra`'s own
/// comment: 4 debug-mode whole-repo builds took ~20 minutes).
fn corpus() -> LoadedRepo {
    let mut repo =
        load_repo(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/navigation"))
            .unwrap();
    let cycle_path = "__dfg_label_parity_cycle.py";
    let cycle_source = "def left(x):\n    return right(x)\n\ndef right(x):\n    return left(x)\n";
    repo.files.insert(
        cycle_path.to_string(),
        prism::ast::ParsedFile::parse(cycle_path, cycle_source, Language::Python).unwrap(),
    );
    repo
}

/// Test-only relabel seam (§7.3, task-1-brief.md Step 3): force every
/// `CpgEdge::DataFlow` edge's payload to `forced`, leaving every other edge
/// kind and the whole node/edge topology untouched. Defined only in this
/// test file — no production API surface exists for it.
fn relabel_all_dataflow_edges(cpg: &mut CodePropertyGraph, forced: FlowConfidence) {
    for weight in cpg.graph.edge_weights_mut() {
        if matches!(weight, CpgEdge::DataFlow(_)) {
            *weight = CpgEdge::DataFlow(forced);
        }
    }
}

/// All distinct `(NodeIndex, VarLocation, kind)` for `CpgNode::Variable`
/// nodes, kept in a stable (BTreeMap-backed) order.
fn variable_locations(
    cpg: &CodePropertyGraph,
) -> Vec<(
    petgraph::graph::NodeIndex,
    prism::data_flow::VarLocation,
    prism::data_flow::VarAccessKind,
)> {
    let mut out = Vec::new();
    for idx in cpg.node_indices() {
        if let prism::cpg::CpgNode::Variable { access, .. } = cpg.node(idx) {
            if let Some(loc) = cpg.to_var_location(idx) {
                let kind = *access;
                let kind = match kind {
                    prism::cpg::VarAccess::Def => prism::data_flow::VarAccessKind::Def,
                    prism::cpg::VarAccess::Use => prism::data_flow::VarAccessKind::Use,
                };
                out.push((idx, loc, kind));
            }
        }
    }
    out
}

/// Every DataFlow edge, selected the way `CpgEdge::is_data_flow` selects it
/// (consumer #9, `src/cpg/types.rs:314`) — the ONE consumer of the ten that
/// calls the named predicate method rather than inlining its own `matches!`.
/// Every other consumer in this file inlines `matches!(_, DataFlow(_))`
/// directly in production code and is therefore unaffected by a bug
/// isolated to `is_data_flow`'s own body; this is the check that would catch
/// exactly that bug (RED record b, task-1-brief.md Step 4c).
fn dataflow_edges_via_is_data_flow(
    cpg: &CodePropertyGraph,
) -> BTreeSet<(petgraph::graph::NodeIndex, petgraph::graph::NodeIndex)> {
    cpg.graph
        .edge_indices()
        .filter(|&e| cpg.graph[e].is_data_flow())
        .filter_map(|e| cpg.graph.edge_endpoints(e))
        .collect()
}

/// Step-5b interprocedural floor, mirroring
/// `tests/infra/parallel_equality_test.rs:108-142`'s
/// `edge_steps_corpus_has_call_dataflow_and_controlflow_edges`: a DataFlow
/// edge whose endpoints are two `Variable` nodes in DIFFERENT
/// `(file, function, function_start_line)` triples can only be a Step-5b
/// arg->param edge (Step 4 is strictly intraprocedural). Payload-insensitive
/// by construction (`CpgEdge::DataFlow(_)`).
fn interproc_dataflow_count(cpg: &CodePropertyGraph) -> usize {
    let g = &cpg.graph;
    let mut count = 0usize;
    for e in g.edge_indices() {
        if let CpgEdge::DataFlow(_) = g[e] {
            if let Some((a, b)) = g.edge_endpoints(e) {
                if let (
                    prism::cpg::CpgNode::Variable {
                        file: fa,
                        function: fna,
                        function_start_line: la,
                        ..
                    },
                    prism::cpg::CpgNode::Variable {
                        file: fb,
                        function: fnb,
                        function_start_line: lb,
                        ..
                    },
                ) = (&g[a], &g[b])
                {
                    if (fa, fna, la) != (fb, fnb, lb) {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// A `DiffInput` that marks every line of every corpus file as changed, so
/// the diff-driven algorithms (`gradient_slice`, `circular_slice`) have
/// maximal signal to traverse from, regardless of which lines happen to
/// carry real data flow in this corpus.
fn whole_corpus_diff(repo: &LoadedRepo) -> DiffInput {
    DiffInput {
        files: repo
            .files
            .iter()
            .map(|(path, parsed)| DiffInfo {
                file_path: path.clone(),
                modify_type: ModifyType::Modified,
                diff_lines: (1..=parsed.source.lines().count().max(1)).collect(),
            })
            .collect(),
    }
}

#[test]
fn consumer_edge_sets_ignore_payload() {
    let repo = corpus();

    let exact_ctx = CpgContext::build(&repo.files, None);
    let real_labels: BTreeSet<FlowConfidence> = exact_ctx
        .cpg
        .graph
        .edge_weights()
        .filter_map(|weight| match weight {
            CpgEdge::DataFlow(label) => Some(*label),
            _ => None,
        })
        .collect();
    assert!(
        real_labels.contains(&FlowConfidence::Exact),
        "real RD build produced no Exact DataFlow payload"
    );
    assert!(
        real_labels.iter().any(|label| !label.is_exact()),
        "real RD build produced no NameOnly DataFlow payload"
    );

    let mut nameonly_ctx = CpgContext::build(&repo.files, None);
    relabel_all_dataflow_edges(
        &mut nameonly_ctx.cpg,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );

    // Non-vacuity guard: the corpus must actually have DataFlow edges, or
    // every check below passes vacuously on two empty sets.
    let df_edge_count = exact_ctx
        .cpg
        .graph
        .edge_weights()
        .filter(|w| matches!(w, CpgEdge::DataFlow(_)))
        .count();
    assert!(
        df_edge_count > 50,
        "corpus too small to surface a label-sensitivity bug: {df_edge_count} DataFlow edges"
    );

    // 1. `data_flow_cycles` (query.rs:239, over strongly_connected_components :218).
    assert_eq!(
        exact_ctx.cpg.data_flow_cycles(),
        nameonly_ctx.cpg.data_flow_cycles(),
        "data_flow_cycles selected a different SCC set depending on payload"
    );

    // 2 & 3. `dfg_forward_reachable` (:679) and `dfg_backward_reachable` (:745)
    // over every Variable location in the corpus.
    let exact_locs = variable_locations(&exact_ctx.cpg);
    let nameonly_locs = variable_locations(&nameonly_ctx.cpg);
    assert_eq!(
        exact_locs.len(),
        nameonly_locs.len(),
        "relabeling changed the node set, which must never happen"
    );
    let mut forward_exact: BTreeMap<
        prism::data_flow::VarLocation,
        BTreeSet<prism::data_flow::VarLocation>,
    > = BTreeMap::new();
    let mut forward_nameonly: BTreeMap<
        prism::data_flow::VarLocation,
        BTreeSet<prism::data_flow::VarLocation>,
    > = BTreeMap::new();
    let mut backward_exact: BTreeMap<
        prism::data_flow::VarLocation,
        BTreeSet<prism::data_flow::VarLocation>,
    > = BTreeMap::new();
    let mut backward_nameonly: BTreeMap<
        prism::data_flow::VarLocation,
        BTreeSet<prism::data_flow::VarLocation>,
    > = BTreeMap::new();
    for (_, loc, _) in &exact_locs {
        forward_exact.insert(loc.clone(), exact_ctx.cpg.dfg_forward_reachable(loc));
        backward_exact.insert(loc.clone(), exact_ctx.cpg.dfg_backward_reachable(loc));
    }
    for (_, loc, _) in &nameonly_locs {
        forward_nameonly.insert(loc.clone(), nameonly_ctx.cpg.dfg_forward_reachable(loc));
        backward_nameonly.insert(loc.clone(), nameonly_ctx.cpg.dfg_backward_reachable(loc));
    }
    assert_eq!(
        forward_exact, forward_nameonly,
        "dfg_forward_reachable selected a different reachable set depending on payload"
    );
    assert_eq!(
        backward_exact, backward_nameonly,
        "dfg_backward_reachable selected a different reachable set depending on payload"
    );
    let any_nonempty_forward = forward_exact.values().any(|s| !s.is_empty());
    let any_nonempty_backward = backward_exact.values().any(|s| !s.is_empty());
    assert!(
        any_nonempty_forward && any_nonempty_backward,
        "reachability checks were vacuous: every forward/backward set was empty"
    );

    // 4. `dfg_chop` (:811): sample consecutive (file, line) points that host a
    // Variable node so the sampled pairs actually have candidate paths.
    let mut points: Vec<(String, usize)> = exact_locs
        .iter()
        .map(|(_, loc, _)| (loc.file.clone(), loc.line))
        .collect();
    points.sort();
    points.dedup();
    assert!(
        points.len() > 10,
        "too few distinct points to sample chop pairs"
    );
    let mut any_nonempty_chop = false;
    for window in [1usize, 3, 8] {
        for i in 0..points.len() {
            let j = i + window;
            if j >= points.len() {
                continue;
            }
            let (sf, sl) = &points[i];
            let (kf, kl) = &points[j];
            let exact_chop = exact_ctx.cpg.dfg_chop(sf, *sl, kf, *kl);
            let nameonly_chop = nameonly_ctx.cpg.dfg_chop(sf, *sl, kf, *kl);
            assert_eq!(
                exact_chop, nameonly_chop,
                "dfg_chop({sf}:{sl} -> {kf}:{kl}) selected a different statement set depending on payload"
            );
            if !exact_chop.is_empty() {
                any_nonempty_chop = true;
            }
        }
    }
    assert!(
        any_nonempty_chop,
        "dfg_chop sampling was vacuous: every sampled pair was empty"
    );

    // 5. `taint_neighbors` (trace.rs:852), via the public `taint_trace` entry
    // point, seeded from every distinct Def (file, line).
    let def_points: Vec<(String, usize)> = exact_locs
        .iter()
        .filter(|(_, _, kind)| *kind == prism::data_flow::VarAccessKind::Def)
        .map(|(_, loc, _)| (loc.file.clone(), loc.line))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert!(
        !def_points.is_empty(),
        "no Def locations to seed taint_trace from"
    );
    let exact_trace = exact_ctx.cpg.taint_trace(&def_points);
    let nameonly_trace = nameonly_ctx.cpg.taint_trace(&def_points);
    assert_eq!(
        exact_trace.frontier_by_root, nameonly_trace.frontier_by_root,
        "taint_trace's frontier (built over taint_neighbors) differs depending on payload"
    );
    assert_eq!(
        exact_trace.parents_by_root, nameonly_trace.parents_by_root,
        "taint_trace's witness parents differ depending on payload"
    );
    assert!(
        !exact_trace.frontier_by_root.is_empty(),
        "taint_trace produced an empty frontier for every seed: vacuous check"
    );

    // 6. `has_incoming_dataflow` (shape.rs:165), via the public
    // `sink_nodes_at` wrapper, over every distinct (file, line) location.
    let mut sink_exact: BTreeMap<(String, usize), Vec<petgraph::graph::NodeIndex>> =
        BTreeMap::new();
    let mut sink_nameonly: BTreeMap<(String, usize), Vec<petgraph::graph::NodeIndex>> =
        BTreeMap::new();
    for (file, line) in &points {
        sink_exact.insert(
            (file.clone(), *line),
            prism::reasoning::shape::sink_nodes_at(&exact_ctx.cpg, file, *line),
        );
        sink_nameonly.insert(
            (file.clone(), *line),
            prism::reasoning::shape::sink_nodes_at(&nameonly_ctx.cpg, file, *line),
        );
    }
    assert_eq!(
        sink_exact, sink_nameonly,
        "sink_nodes_at (has_incoming_dataflow) selected different nodes depending on payload"
    );
    assert!(
        sink_exact.values().any(|nodes| !nodes.is_empty()),
        "sink_nodes_at selected no nodes at all: vacuous check"
    );

    // 7. `gradient_slice::slice` (:110) and 8. `circular_slice::slice` (:239),
    // over a whole-corpus diff — compared via JSON serialization since
    // `SliceResult` does not derive `PartialEq`.
    let diff = whole_corpus_diff(&repo);
    let gradient_config = prism::algorithms::gradient_slice::GradientConfig::default();
    let exact_gradient =
        prism::algorithms::gradient_slice::slice(&exact_ctx, &diff, &gradient_config)
            .expect("gradient_slice::slice failed on the Exact-labeled graph");
    let nameonly_gradient =
        prism::algorithms::gradient_slice::slice(&nameonly_ctx, &diff, &gradient_config)
            .expect("gradient_slice::slice failed on the NameOnly-labeled graph");
    assert_eq!(
        exact_gradient.to_json().unwrap(),
        nameonly_gradient.to_json().unwrap(),
        "gradient_slice::slice produced different output depending on payload"
    );
    assert!(
        !exact_gradient.blocks.is_empty(),
        "gradient_slice produced no blocks: vacuous check"
    );

    let exact_circular = prism::algorithms::circular_slice::slice(&exact_ctx, &diff)
        .expect("circular_slice::slice failed on the Exact-labeled graph");
    let nameonly_circular = prism::algorithms::circular_slice::slice(&nameonly_ctx, &diff)
        .expect("circular_slice::slice failed on the NameOnly-labeled graph");
    assert_eq!(
        exact_circular.to_json().unwrap(),
        nameonly_circular.to_json().unwrap(),
        "circular_slice::slice produced different output depending on payload"
    );
    assert!(
        !exact_circular.blocks.is_empty(),
        "circular_slice produced no blocks: vacuous check"
    );

    // 10. Step-5b interprocedural floor
    // (tests/infra/parallel_equality_test.rs:108-142).
    let exact_interproc = interproc_dataflow_count(&exact_ctx.cpg);
    let nameonly_interproc = interproc_dataflow_count(&nameonly_ctx.cpg);
    assert_eq!(
        exact_interproc, nameonly_interproc,
        "the Step-5b interprocedural DataFlow floor differs depending on payload"
    );
    assert!(
        exact_interproc > 0,
        "no interprocedural DataFlow edges found: corpus cannot surface a Step-5b regression"
    );

    // 9. `CpgEdge::is_data_flow` (types.rs:314) — the ONE consumer that calls
    // the named predicate method rather than inlining `matches!`. Checked
    // LAST, after every other consumer above has already asserted equal:
    // when this one is deliberately broken (RED record b), it is the ONLY
    // assertion in this test that fails, localizing the label-sensitivity
    // bug to `is_data_flow` itself.
    assert_eq!(
        dataflow_edges_via_is_data_flow(&exact_ctx.cpg),
        dataflow_edges_via_is_data_flow(&nameonly_ctx.cpg),
        "CpgEdge::is_data_flow selected a different edge set depending on payload"
    );
    assert!(
        !dataflow_edges_via_is_data_flow(&exact_ctx.cpg).is_empty(),
        "is_data_flow selected no edges at all: vacuous check"
    );
}

fn graph_dataflow_payloads(
    cpg: &CodePropertyGraph,
) -> BTreeMap<(VarLocation, VarLocation), FlowConfidence> {
    let mut payloads = BTreeMap::new();
    for edge_index in cpg.graph.edge_indices() {
        let CpgEdge::DataFlow(label) = cpg.graph[edge_index] else {
            continue;
        };
        let Some((from_index, to_index)) = cpg.graph.edge_endpoints(edge_index) else {
            continue;
        };
        let (Some(from), Some(to)) = (
            cpg.to_var_location(from_index),
            cpg.to_var_location(to_index),
        ) else {
            continue;
        };
        payloads
            .entry((from, to))
            .and_modify(|stored: &mut FlowConfidence| *stored = stored.worst(label))
            .or_insert(label);
    }
    payloads
}

fn expected_step5b_labels(
    cpg: &CodePropertyGraph,
    from: &VarLocation,
    to: &VarLocation,
) -> BTreeSet<FlowConfidence> {
    let mut labels = BTreeSet::new();
    for (caller, sites) in &cpg.call_graph.calls {
        if (
            caller.file.as_str(),
            caller.name.as_str(),
            caller.start_line,
        ) != (
            from.file.as_str(),
            from.function.as_str(),
            from.function_start_line,
        ) {
            continue;
        }
        for site in sites {
            for resolved in cpg.call_graph.resolve_call_site(site) {
                if (
                    resolved.target.file.as_str(),
                    resolved.target.name.as_str(),
                    resolved.target.start_line,
                ) == (
                    to.file.as_str(),
                    to.function.as_str(),
                    to.function_start_line,
                ) {
                    labels.insert(FlowConfidence::from(resolved.confidence));
                }
            }
        }
    }
    labels
}

#[test]
fn every_graph_dataflow_payload_equals_its_dfg_label() {
    let repo = corpus();
    let cpg = CodePropertyGraph::build(&repo.files);
    let payloads = graph_dataflow_payloads(&cpg);
    assert!(
        payloads.len() > 50,
        "fixture must exercise real DataFlow edges"
    );

    for ((from, to), payload) in payloads {
        if let Some(label) = cpg.dfg.labels.get(&(from.clone(), to.clone())) {
            assert_eq!(
                payload, *label,
                "Step-4 payload diverged from dfg.labels for {from:?} -> {to:?}"
            );
            continue;
        }

        assert_eq!(from.kind, VarAccessKind::Use);
        assert_eq!(to.kind, VarAccessKind::Def);
        assert_ne!(
            (
                from.file.as_str(),
                from.function.as_str(),
                from.function_start_line
            ),
            (
                to.file.as_str(),
                to.function.as_str(),
                to.function_start_line
            ),
            "a same-function DataFlow payload is missing from dfg.labels"
        );
        let expected = expected_step5b_labels(&cpg, &from, &to);
        assert!(
            expected.contains(&payload),
            "Step-5b payload {payload:?} has no matching resolved-callee floor for {from:?} -> {to:?}; candidates={expected:?}"
        );
    }
}

fn parse_python_sources(
    sources: &BTreeMap<String, String>,
) -> BTreeMap<String, prism::ast::ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            (
                path.clone(),
                prism::ast::ParsedFile::parse(path, source, Language::Python).unwrap(),
            )
        })
        .collect()
}

fn assert_dfg_label_membership_complete(cpg: &CodePropertyGraph) {
    let edge_keys: BTreeSet<_> = cpg
        .dfg
        .edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect();
    assert_eq!(
        edge_keys,
        cpg.dfg.labels.keys().cloned().collect(),
        "an existing DFG edge is missing a primary label, or a label has no edge"
    );
}

#[test]
fn cache_cold_full_hit_and_partial_hit_agree_on_every_label() {
    let sources = BTreeMap::from([
        (
            "a.py".to_string(),
            "def a():\n    x = source()\n    sink(x)\n".to_string(),
        ),
        (
            "b.py".to_string(),
            "def b():\n    y = source()\n    sink(y)\n".to_string(),
        ),
        (
            "c.py".to_string(),
            "def c():\n    q = source()\n    sink(q)\n".to_string(),
        ),
    ]);
    let files = parse_python_sources(&sources);
    let cold = CodePropertyGraph::build(&files);
    assert_dfg_label_membership_complete(&cold);
    let cold_labels = cold.dfg.labels.clone();
    let cold_rd_function_stats = cold.dfg.rd_function_stats.clone();
    let cold_payloads = graph_dataflow_payloads(&cold);
    assert!(cold_labels.values().any(|label| label.is_exact()));

    let cache_dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cold, &hashes, false, cache_dir.path()).unwrap();
    let full_hit = match cpg_cache::load_cache(&hashes, false, cache_dir.path()) {
        CacheResult::Hit(cpg) => cpg,
        CacheResult::PartialHit { .. } => panic!("expected full cache hit, got partial hit"),
        CacheResult::Miss => panic!("expected full cache hit, got miss"),
    };
    assert_dfg_label_membership_complete(&full_hit);
    assert_eq!(full_hit.dfg.labels, cold_labels);
    assert_eq!(full_hit.dfg.rd_function_stats, cold_rd_function_stats);
    assert_eq!(graph_dataflow_payloads(&full_hit), cold_payloads);

    let mut edited_sources = sources.clone();
    edited_sources.insert(
        "b.py".to_string(),
        "def b():\n    z = source()\n    z = clean()\n    sink(z)\n".to_string(),
    );
    let edited_files = parse_python_sources(&edited_sources);
    let edited_hashes = cpg_cache::compute_file_hashes(&edited_sources);
    let incremental = match cpg_cache::load_cache(&edited_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(changed_files, BTreeSet::from(["b.py".to_string()]));
            CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &edited_files,
                None,
            )
        }
        CacheResult::Hit(_) => panic!("one-file edit unexpectedly produced a full hit"),
        CacheResult::Miss => panic!("one-file edit unexpectedly produced a miss"),
    };
    let edited_cold = CodePropertyGraph::build(&edited_files);
    assert_dfg_label_membership_complete(&incremental);
    assert_dfg_label_membership_complete(&edited_cold);
    assert_eq!(incremental.dfg.labels, edited_cold.dfg.labels);
    assert_eq!(
        incremental.dfg.rd_function_stats,
        edited_cold.dfg.rd_function_stats
    );
    assert_eq!(
        incremental.dfg.rd_function_stats["a.py"], cold_rd_function_stats["a.py"],
        "the retained file's RD counters changed during a one-file edit"
    );
    assert_eq!(
        graph_dataflow_payloads(&incremental),
        graph_dataflow_payloads(&edited_cold)
    );

    assert!(incremental
        .dfg
        .labels
        .keys()
        .any(|(from, _)| from.file == "a.py"));
    assert!(!incremental
        .dfg
        .labels
        .keys()
        .any(|(from, to)| from.file == "b.py" && (from.path.base == "y" || to.path.base == "y")));
    assert!(incremental
        .dfg
        .labels
        .keys()
        .any(|(from, to)| from.file == "b.py" && (from.path.base == "z" || to.path.base == "z")));
}
