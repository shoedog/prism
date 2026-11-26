//! Occurrence-aware Step 5b parity across argument producers and cache routes.

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

#[path = "param_binding_bare_tests.rs"]
mod bare;

#[test]
fn bare_literal_leaves_respect_outgoing_free_single_reachability() {
    use crate::cpg::object_literal_arg_tests::{literal_edges, spell};

    const MAIN: &str = "const take = p => {\n  parseInt(p.a);\n  observe(p.a);\n};\n\
                       function main(v) {\n  take({ a: v });\n}\n";
    const LIB: &str = "export function parseInt(text) {\n  use(text);\n}\n";
    for (import, lib, route, keep_first_leaf) in [
        ("", Some(LIB), Some(ResolutionKind::FreeSingle), false),
        ("", None, None, true),
        (
            "import { parseInt } from './lib';",
            Some(LIB),
            Some(ResolutionKind::ImportMember),
            true,
        ),
    ] {
        let mut sources = BTreeMap::from([("m.js".into(), format!("{import}\n{MAIN}"))]);
        if let Some(lib) = lib {
            sources.insert("lib.js".into(), lib.into());
        }
        let files = parse(&sources);
        let cold = CodePropertyGraph::build(&files);
        let legacy = with_legacy(|| CodePropertyGraph::build(&files));
        let serial = super::step5b_reference::with_serial(|| CodePropertyGraph::build(&files));
        for (assembly, cpg) in [("production", cold), ("legacy", legacy), ("serial", serial)] {
            let context = format!("{assembly} with {route:?}");
            let site = bare::site(&cpg, &files["m.js"], "parseInt(p.a)", "parseInt");
            let resolved: Vec<_> = cpg
                .call_graph
                .resolve_call_site(site)
                .into_iter()
                .map(|arm| (arm.target.file.as_str(), arm.kind, arm.confidence))
                .collect();
            let expected: Vec<_> = route
                .into_iter()
                .map(|kind| ("lib.js", kind, ResolutionConfidence::Exact))
                .collect();
            assert_eq!(resolved, expected, "{context}");
            let has_outgoing = cpg.graph.edge_references().any(|edge| {
                *edge.weight() == CpgEdge::DataFlow(FlowConfidence::Exact)
                    && spell(&cpg, edge.source()) == Some(("take:p.a@3".into(), true))
                    && spell(&cpg, edge.target()) == Some(("parseInt:text@1".into(), false))
            });
            assert_eq!(has_outgoing, route.is_some(), "{context}");
            let mut expected =
                BTreeMap::from([("main:v@7 -> take:p.a@4".into(), FlowConfidence::Exact)]);
            if keep_first_leaf {
                expected.insert("main:v@7 -> take:p.a@3".into(), FlowConfidence::Exact);
            }
            assert_eq!(literal_edges(&cpg), expected, "{context}");
            let exact = cpg
                .graph
                .edge_weights()
                .filter(|edge| **edge == CpgEdge::DataFlow(FlowConfidence::Exact))
                .count();
            assert_eq!(cpg.dfg_label_stats.dfg_label_exact, exact, "{context}");
        }
    }
}

#[test]
fn bare_bindings_are_identical_across_every_route() {
    use bare::{assert_bindings, endpoint, site, target, BARE};
    let mut sources = BTreeMap::from([
        ("lib.js".into(), format!("export const take = {BARE};")),
        (
            "main.js".into(),
            "import { take } from './lib';\nfunction main(v) { take(v); take({ a: v }); }".into(),
        ),
        (
            "loose.js".into(),
            "export function loose(v) { take(v); take({ a: v }); }".into(),
        ),
        (
            "unrelated.js".into(),
            "export function unrelated() { return 1; }".into(),
        ),
    ]);
    let files = parse(&sources);
    let cold = CodePropertyGraph::build(&files);
    let assert_endpoints = |graph: &CodePropertyGraph| {
        let lib = &files["lib.js"];
        let owner = target(graph, lib, "take", BARE);
        for (file, admitted) in [("main.js", true), ("loose.js", false)] {
            let parsed = &files[file];
            for (call, leaf) in [("take(v)", false), ("take({ a: v })", true)] {
                let site = site(graph, parsed, call, "take");
                let edges = graph.graph.edge_indices().filter_map(|index| {
                    let (from, to) = graph.graph.edge_endpoints(index)?;
                    let edge = graph.graph[index].clone();
                    (matches!(edge, CpgEdge::DataFlow(_))
                        && matches!(graph.node(from), CpgNode::Variable { file, start_byte, end_byte, .. }
                            if file == &parsed.path && site.start_byte <= *start_byte && *end_byte <= site.end_byte)
                        && matches!(graph.node(to), CpgNode::Variable { file, function, .. }
                            if file == &lib.path && function == "take"))
                    .then_some(PendingArgumentFlow {
                        edge: (from, to, edge),
                        cross_file_free_single_exact: false,
                        bare_literal: false,
                    })
                }).collect::<Vec<_>>();
                let expected = [endpoint(lib, owner, leaf)];
                assert_bindings(
                    graph,
                    site,
                    parsed,
                    &edges,
                    if admitted { &expected } else { &[] },
                );
            }
        }
    };
    assert_endpoints(&cold);
    let cold_edges = edge_multiset(&cold);
    assert_step5b_parity(&files, &cold, &cold_edges);

    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(full) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected a full hit");
    };
    assert_endpoints(&full);
    assert_eq!(edge_multiset(&full), cold_edges);

    sources.insert(
        "unrelated.js".into(),
        "export function unrelated() { return 2; }".into(),
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
    assert_endpoints(&partial);
    assert_eq!(
        edge_multiset(&partial),
        edge_multiset(&CodePropertyGraph::build(&edited_files))
    );
}
