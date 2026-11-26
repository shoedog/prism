//! #594: one literal-bearing fixture compared as a complete occurrence-aware edge multiset
//! (both endpoints with bytes, the edge label) across the production assembly, the frozen
//! legacy twin, the serial Step-5b reference, a cache round trip (full hit) and an
//! incremental rebuild (partial hit), and pinned to the literal edges it must contain so
//! the suite's RAYON_NUM_THREADS=1 and 4 runs compare against the same answer.
use super::super::callee_legacy::with_legacy;
use super::*;
use crate::cpg_cache::{self, CacheResult};
use crate::languages::Language;

const CALLEE: &str = "export function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nexport function g(opts) {\n  sinkP(opts);\n  opts.c = clean();\n  sinkC(opts.c);\n  sinkD(opts.d);\n}\n";
const CALLER: &str = "import { f, g } from \"./a\";\nexport function h(x, y, z) {\n  f({ a: x, b: y });\n  f({ a: x }); f({ a: x });\n  g({ c: z, d: z });\n}\n";
const OTHER: &str = "export function unrelated() { return 1; }\n";

fn sources(other: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("a.js".to_string(), CALLEE.to_string()),
        ("b.js".to_string(), CALLER.to_string()),
        ("c.js".to_string(), other.to_string()),
    ])
}

fn parse(sources: &BTreeMap<String, String>) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            let file = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
            assert_eq!(file.parse_error_count, 0, "{path}");
            (path.clone(), file)
        })
        .collect()
}

/// Every edge with both endpoints and the label, sorted, duplicates kept.
fn multiset(cpg: &CodePropertyGraph) -> Vec<String> {
    let mut out: Vec<String> = cpg
        .graph
        .edge_indices()
        .map(|edge| {
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            format!(
                "{:?} -> {:?} [{:?}]",
                cpg.node(from),
                cpg.node(to),
                cpg.graph[edge]
            )
        })
        .collect();
    out.sort();
    out
}

/// Cross-function DataFlow edges that end at a Use node: the literal edges.
fn literal_edges(cpg: &CodePropertyGraph) -> Vec<String> {
    let mut out: Vec<String> = cpg
        .graph
        .edge_indices()
        .filter_map(|edge| {
            if !matches!(cpg.graph[edge], CpgEdge::DataFlow(_)) {
                return None;
            }
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            let (
                CpgNode::Variable {
                    function: from_fn,
                    path: from_path,
                    line: from_line,
                    ..
                },
                CpgNode::Variable {
                    function: to_fn,
                    path: to_path,
                    line: to_line,
                    access: VarAccess::Use,
                    ..
                },
            ) = (cpg.node(from), cpg.node(to))
            else {
                return None;
            };
            (from_fn != to_fn).then(|| {
                format!("{from_fn}:{from_path}@{from_line} -> {to_fn}:{to_path}@{to_line}")
            })
        })
        .collect();
    out.sort();
    out
}

#[test]
fn literal_bearing_graph_is_identical_across_every_route() {
    let sources = sources(OTHER);
    let files = parse(&sources);
    let cold = CodePropertyGraph::build(&files);

    // The pinned answer: `x` feeds only `p.a`'s read (line 2), `y` only `p.b`'s (line 3);
    // each of the three `f` call sites has its own `x` occurrence; `g`'s `c` key is
    // overwritten by `opts.c = ...`, so only `d` reaches the formal, through the bare read
    // (line 6) and `opts.d` (line 9).
    let pinned = literal_edges(&cold);
    assert_eq!(
        pinned,
        [
            "h:x@3 -> f:p.a@2",
            "h:x@4 -> f:p.a@2",
            "h:x@4 -> f:p.a@2",
            "h:y@3 -> f:p.b@3",
            "h:z@5 -> g:opts@6",
            "h:z@5 -> g:opts@9",
        ]
    );
    let cold_multiset = multiset(&cold);

    // Frozen legacy twin.
    let cg = CallGraph::build(&files);
    let dfg = DataFlowGraph::build(&files);
    let legacy = with_legacy(|| CodePropertyGraph::assemble_graph(cg, dfg, &files, None));
    assert_eq!(multiset(&legacy), cold_multiset, "legacy twin");

    // Serial reference against the parallel collection.
    let par = CodePropertyGraph::collect_step5b_edges(
        &cold.call_graph,
        &cold.var_index,
        &cold.graph,
        &files,
    );
    let serial = CodePropertyGraph::collect_step5b_edges_reference(
        &cold.call_graph,
        &cold.var_index,
        &cold.graph,
        &files,
    );
    assert_eq!(par, serial, "serial reference");

    // Full hit.
    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(restored) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected a full hit");
    };
    assert_eq!(multiset(&restored), cold_multiset, "full hit");

    // Partial hit: an unrelated file changed.
    let edited = sources_with(OTHER, "export function unrelated() { return 2; }\n");
    let edited_hashes = cpg_cache::compute_file_hashes(&edited);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&edited_hashes, false, dir.path())
    else {
        panic!("expected a partial hit");
    };
    let edited_files = parse(&edited);
    let partial = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited_files,
        None,
    );
    let cold_edited = CodePropertyGraph::build(&edited_files);
    assert_eq!(multiset(&partial), multiset(&cold_edited), "partial hit");
    assert_eq!(literal_edges(&partial), pinned, "partial hit literal edges");
    assert_eq!(literal_edges(&restored), pinned, "full hit literal edges");
}

fn sources_with(old: &str, new: &str) -> BTreeMap<String, String> {
    let mut edited = sources(old);
    edited.insert("c.js".to_string(), new.to_string());
    edited
}

fn pattern_sources(edited: bool) -> BTreeMap<String, String> {
    let mut inputs = sources(OTHER);
    let (key, local, operand) = if edited {
        ("b", "next", "y")
    } else {
        ("a", "local", "x")
    };
    inputs.get_mut("a.js").unwrap().push_str(&format!(
        "export function D({{{key}: {local}}}) {{\n  return use({local});\n}}\n"
    ));
    let caller = inputs.get_mut("b.js").unwrap();
    *caller = caller.replace("f, g", "f, g, D");
    caller.truncate(caller.len() - 2);
    caller.push_str(&format!("  D({{{key}: {operand}, ignored: z}});\n}}\n"));
    inputs
}

fn pattern_hops(
    cpg: &CodePropertyGraph,
) -> Vec<(String, usize, usize, String, usize, usize, FlowConfidence)> {
    cpg.graph
        .edge_references()
        .filter_map(|edge| {
            use petgraph::visit::EdgeRef;
            let CpgEdge::DataFlow(label) = *edge.weight() else {
                return None;
            };
            let CpgNode::Variable {
                function: caller,
                path: source,
                start_byte: lo,
                end_byte: hi,
                access: VarAccess::Use,
                ..
            } = cpg.node(edge.source())
            else {
                return None;
            };
            let CpgNode::Variable {
                function: callee,
                path: dest,
                start_byte: start,
                end_byte: end,
                access: VarAccess::Def,
                ..
            } = cpg.node(edge.target())
            else {
                return None;
            };
            (caller == "h" && callee == "D").then(|| {
                (
                    source.to_string(),
                    *lo,
                    *hi,
                    dest.to_string(),
                    *start,
                    *end,
                    label,
                )
            })
        })
        .collect()
}

fn expect_pattern_answer(
    cpg: &CodePropertyGraph,
    sources: &BTreeMap<String, String>,
    edited: bool,
) {
    let (source, local, key) = if edited {
        ("y", "next", "b")
    } else {
        ("x", "local", "a")
    };
    let from = sources["b.js"]
        .find(&format!("D({{{key}: {source}"))
        .unwrap()
        + "D({a: ".len();
    let to = sources["a.js"]
        .find(&format!("D({{{key}: {local}"))
        .unwrap()
        + "D({a: ".len();
    let map = BTreeMap::from([
        ("sites_literal".into(), 5),
        ("edges_to_def".into(), 1),
        ("edges_to_use".into(), 5),
        ("edges_to_bare_use".into(), 1),
        ("refused_key_not_read".into(), 2),
    ]);
    assert_eq!(
        (pattern_hops(cpg), &cpg.dfg_label_stats.dfg_props),
        (
            vec![(
                source.into(),
                from,
                from + 1,
                local.into(),
                to,
                to + local.len(),
                FlowConfidence::Exact
            )],
            &map
        ),
        "pinned hop and counters for pattern, plain, bare and negative sibling",
    );
}

#[test]
fn props_s2_pattern_formal_graph_and_pending_parity() {
    use crate::cpg::props_s2_test_support::{capture, Assembly};

    let inputs = pattern_sources(false);
    let files = parse(&inputs);
    let (cold, production) = capture(&files, || CodePropertyGraph::build(&files));
    let assemble = || {
        CodePropertyGraph::assemble_graph(
            CallGraph::build(&files),
            DataFlowGraph::build(&files),
            &files,
            None,
        )
    };
    let (legacy, legacy_pending) = capture(&files, || with_legacy(assemble));
    let (serial, serial_pending) = capture(&files, || {
        super::super::step5b_reference::with_serial(assemble)
    });
    let compare_pending = |other: Assembly| {
        let base = production.pre_binding.as_ref().unwrap();
        let input = other.pre_binding.unwrap();
        let snapshot = |graph: &DiGraph<CpgNode, CpgEdge>| {
            let nodes = graph
                .node_weights()
                .map(|node| format!("{node:?}"))
                .collect::<Vec<_>>();
            let edges = graph
                .edge_indices()
                .map(|id| (graph.edge_endpoints(id).unwrap(), graph[id].clone()))
                .collect::<Vec<_>>();
            (nodes, edges)
        };
        assert_eq!(
            snapshot(&base.graph),
            snapshot(&input.graph),
            "same pre-Step-5b graph"
        );
        assert_eq!(base.lookup, input.lookup, "same masked legacy lookup");
        assert_eq!(base.exact, input.exact, "same masked exact lookup");
        assert_eq!(
            production.pending, other.pending,
            "same pending order, labels and guard flags"
        );
        assert_eq!(production.literal_attempts, other.literal_attempts);
        assert_eq!(production.literal_witnesses, other.literal_witnesses);
        let before_guard = production.guard_input.as_ref().unwrap();
        let other_guard = other.guard_input.unwrap();
        assert_eq!(before_guard.counters, other_guard.counters);
        assert_eq!(before_guard.seeds, other_guard.seeds);
        assert_eq!(before_guard.bare_literals, other_guard.bare_literals);
    };
    assert_eq!(production.literal_attempts.len(), 5);
    assert!(production
        .literal_witnesses
        .iter()
        .any(|witness| witness.class == "edges_to_def"));
    compare_pending(legacy_pending);
    compare_pending(serial_pending);
    let same_final = |actual: &CodePropertyGraph, expected: &CodePropertyGraph| {
        assert_eq!(multiset(actual), multiset(expected));
        assert_eq!(actual.dfg_label_stats, expected.dfg_label_stats);
    };
    same_final(&legacy, &cold);
    same_final(&serial, &cold);

    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&inputs);
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(full) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected full hit");
    };
    same_final(&full, &cold);

    let mut unrelated = inputs.clone();
    unrelated.insert("c.js".into(), OTHER.replace('1', "2"));
    let pattern_edit = pattern_sources(true);
    let mut partials = Vec::new();
    for (sources, changed, edited) in [
        (&unrelated, &["c.js"][..], false),
        (&pattern_edit, &["a.js", "b.js"][..], true),
    ] {
        let CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } = cpg_cache::load_cache(&cpg_cache::compute_file_hashes(sources), false, dir.path())
        else {
            panic!("expected partial hit for {changed:?}");
        };
        assert_eq!(
            changed_files,
            changed.iter().map(|file| file.to_string()).collect()
        );
        let parsed = parse(sources);
        let partial = CodePropertyGraph::build_incremental(
            cached_call_graph,
            cached_dfg,
            &changed_files,
            &parsed,
            None,
        );
        same_final(&partial, &CodePropertyGraph::build(&parsed));
        assert_eq!(
            literal_edges(&partial),
            literal_edges(&cold),
            "preserved plain and bare hops"
        );
        partials.push((partial, sources, edited));
    }
    // Evaluate both literal expectations even on the pre-change graph.
    let answers: Vec<_> = std::iter::once((&cold, &inputs, false))
        .chain(
            partials
                .iter()
                .map(|(graph, sources, edited)| (graph, *sources, *edited)),
        )
        .map(|(graph, sources, edited)| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                expect_pattern_answer(graph, sources, edited)
            }))
            .is_ok()
        })
        .collect();
    assert_eq!(
        answers,
        [true, true, true],
        "cold, unrelated edit, changed pattern"
    );
}
