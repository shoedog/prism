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
