//! Occurrence-aware Step-5b parity for spread arguments and chained calls.

use super::callee_legacy::with_legacy;
use super::*;
use crate::cpg_cache::{self, CacheResult};
use crate::languages::Language;

const FIXTURE: &str = r#"function f(a, b, c) {
  use(a, b, c);
}
function spread(list, blocked, prefix, empty, one, dense, first, shifted, sparse, nested) {
  f(...list, blocked);
  f(...list);
  f(prefix, ...list);
  f(...[], empty);
  f(...[0], one);
  f(...[0, 1], dense);
  f(first, ...[0], shifted);
  f(...[, 1], sparse);
  f(...[...list], nested);
}
class B {
  m(b1, b2) {
    use(b1, b2);
  }
}
class A {
  m(a1, a2 = 0) {
    use(a1, a2);
    return new B();
  }
  chain(xs, v, inner, w, z) {
    this.m(...xs, v).m(w, z);
    this.m(inner).m(w, z);
  }
}
function rest(first, ...later) {
  use(first, later);
}
function rest_all(...items) { use(items); }
function rest_unknown_all(xs) { rest_all(...xs); }
function rest_unknown_tail(prefix, xs) { rest(prefix, ...xs); }
function rest_control(prefix, surplus) {
  rest(...[], prefix, surplus, surplus);
}
function rest_spread(still_fixed, dense, trailing) {
  rest(...still_fixed, ...[0, dense], trailing);
}
function rest_opaque(prefix) {
  const proven = [1];
  rest(prefix, ...proven);
}
"#;

fn parse(sources: &BTreeMap<String, String>) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
            assert_eq!(parsed.parse_error_count, 0, "{path}");
            (path.clone(), parsed)
        })
        .collect()
}

fn edge_multiset(cpg: &CodePropertyGraph) -> Vec<String> {
    let mut edges: Vec<_> = cpg
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
    edges.sort();
    edges
}

fn fixture_bindings(cpg: &CodePropertyGraph) -> Vec<String> {
    let mut edges: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(confidence) = &cpg.graph[edge] else {
                return None;
            };
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            let (
                CpgNode::Variable {
                    function: from_fn,
                    path: from_path,
                    access: VarAccess::Use,
                    ..
                },
                CpgNode::Variable {
                    function: to_fn,
                    path: to_path,
                    access: VarAccess::Def,
                    ..
                },
            ) = (cpg.node(from), cpg.node(to))
            else {
                return None;
            };
            let pinned = matches!(
                (from_fn.as_str(), to_fn.as_str()),
                ("spread", "f")
                    | (
                        "rest_control" | "rest_opaque" | "rest_spread" | "rest_unknown_tail",
                        "rest"
                    )
                    | ("rest_unknown_all", "rest_all")
            ) || (from_fn == "chain"
                && to_fn == "m"
                && matches!(to_path.base.as_str(), "a1" | "a2"));
            pinned.then(|| format!("{from_fn}:{from_path} -> {to_fn}:{to_path} [{confidence:?}]"))
        })
        .collect();
    edges.sort();
    edges
}

fn assert_step5b_parity(
    files: &BTreeMap<String, ParsedFile>,
    cold: &CodePropertyGraph,
    cold_edges: &[String],
) {
    let cg = CallGraph::build(files);
    let dfg = DataFlowGraph::build(files);
    let legacy = with_legacy(|| CodePropertyGraph::assemble_graph(cg, dfg, files, None));
    assert_eq!(edge_multiset(&legacy), cold_edges, "legacy twin");

    let parallel = CodePropertyGraph::collect_step5b_edges(
        &cold.call_graph,
        &cold.var_index,
        &cold.graph,
        files,
    );
    let serial = CodePropertyGraph::collect_step5b_edges_reference(
        &cold.call_graph,
        &cold.var_index,
        &cold.graph,
        files,
    );
    assert_eq!(parallel, serial, "serial reference and emission order");
}

fn assert_spread_bindings(path: &str, source: &str, language: Language, expected: &str) {
    let parsed = ParsedFile::parse(path, source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{path}: {source}");
    let files = BTreeMap::from([(path.to_string(), parsed)]);
    let cold = CodePropertyGraph::build(&files);
    assert_eq!(fixture_bindings(&cold), [expected], "{path}: {source}");
    assert_step5b_parity(&files, &cold, &edge_multiset(&cold));
}

fn assert_js_comment_binding(comment: &str) {
    let source = format!(
        "function f(a, b, c, d = 0) {{ use(a, b, c, d); }}\nfunction spread(x) {{ f(...[0, 1],\n{comment}\nx); }}"
    );
    assert_spread_bindings(
        "fixture.js",
        &source,
        Language::JavaScript,
        "spread:x -> f:c [Exact]",
    );
}

#[test]
fn html_open_comment_does_not_shift_spread_binding() {
    assert_js_comment_binding("<!-- ignored");
}

#[test]
fn html_close_comment_does_not_shift_spread_binding() {
    assert_js_comment_binding("--> ignored");
}

fn assert_instantiation_binding(path: &str, language: Language) {
    for type_args in ["", "<number>"] {
        let source = format!(
            "type G = [number] & (<T>() => T);\nfunction f(a = 11, b = 22) {{ use(a, b); }}\nfunction spread(x: number) {{ f(...([0] as G){type_args}, x); }}"
        );
        assert_spread_bindings(path, &source, language, "spread:x -> f:b [Exact]");
    }
}

#[test]
fn ts_instantiation_preserves_spread_binding() {
    assert_instantiation_binding("fixture.ts", Language::TypeScript);
}

#[test]
fn tsx_instantiation_preserves_spread_binding() {
    assert_instantiation_binding("fixture.tsx", Language::Tsx);
}

#[test]
fn spread_and_rest_graph_is_identical_across_every_route() {
    let mut sources = BTreeMap::from([
        ("fixture.js".to_string(), FIXTURE.to_string()),
        (
            "unrelated.js".to_string(),
            "export function unrelated() { return 1; }\n".to_string(),
        ),
    ]);
    let files = parse(&sources);
    let cold = CodePropertyGraph::build(&files);

    // a01/a02: unknown spread and its source add no edges; the prefix still binds.
    // a05: only the inner non-spread argument binds to A.m. In particular, neither
    // outer argument nor the argument after the unknown spread can reach A.m.
    // a06: dense lengths shift the slot; holes and nested spreads stop binding.
    // b02b/c: both opaque operands resolve to a singleton rest destination but
    // have no yield witness. The mixed call still binds its fixed prefix.
    for (caller, callee) in [
        ("rest_unknown_all", "rest_all"),
        ("rest_unknown_tail", "rest"),
    ] {
        let site = cold
            .call_graph
            .calls
            .values()
            .flatten()
            .find(|site| site.caller.name == caller && site.callee_name == callee)
            .unwrap();
        let resolved = cold.call_graph.resolve_call_site(site);
        assert!(
            object_literal_args::exact_singleton(&resolved),
            "{caller}: {resolved:?}"
        );
        assert_eq!(resolved[0].target.name, callee);
    }
    // Preserve both occurrences of the surplus, the dense element and the
    // separately proved opaque spread.
    let pinned = fixture_bindings(&cold);
    assert_eq!(
        pinned,
        [
            "chain:inner -> m:a1 [Exact]",
            "rest_control:prefix -> rest:first [Exact]",
            "rest_control:surplus -> rest:later [Exact]",
            "rest_control:surplus -> rest:later [Exact]",
            "rest_opaque:prefix -> rest:first [Exact]",
            "rest_opaque:proven -> rest:later [Exact]",
            "rest_spread:dense -> rest:later [Exact]",
            "rest_spread:trailing -> rest:later [Exact]",
            "rest_unknown_tail:prefix -> rest:first [Exact]",
            "spread:dense -> f:c [Exact]",
            "spread:empty -> f:a [Exact]",
            "spread:first -> f:a [Exact]",
            "spread:one -> f:b [Exact]",
            "spread:prefix -> f:a [Exact]",
            "spread:shifted -> f:c [Exact]",
        ],
        "positional binding edges"
    );
    let cold_edges = edge_multiset(&cold);
    assert_step5b_parity(&files, &cold, &cold_edges);

    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(restored) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected a full hit");
    };
    assert_eq!(edge_multiset(&restored), cold_edges, "full hit");

    sources.insert(
        "unrelated.js".to_string(),
        "export function unrelated() { return 2; }\n".to_string(),
    );
    let edited_hashes = cpg_cache::compute_file_hashes(&sources);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&edited_hashes, false, dir.path())
    else {
        panic!("expected a partial hit");
    };
    let edited_files = parse(&sources);
    let partial = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited_files,
        None,
    );
    let cold_edited = CodePropertyGraph::build(&edited_files);
    assert_eq!(edge_multiset(&partial), edge_multiset(&cold_edited));
    assert_eq!(fixture_bindings(&partial), pinned, "partial hit bindings");
}
