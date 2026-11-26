//! Focused direct-literal checks for site counts, precedence and writer refusals.

use super::object_literal_arg_tests::{build, spell};
use super::props_s2_test_support::check_counter;
use super::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence};
use crate::ast::ParsedFile;
use crate::call_graph::{CallSite, JsLocalBinding};
use crate::languages::Language;
use crate::resolution::{ResolutionConfidence, ResolutionKind};
use petgraph::visit::EdgeRef;
use std::collections::BTreeMap;

fn graph(ext: &str, source: &str) -> CodePropertyGraph {
    build(
        Language::from_extension(ext).unwrap(),
        &[("m", source)],
        ext,
    )
}

fn recorded<'a>(graph: &'a CodePropertyGraph, caller: &str, target: &str) -> Vec<&'a CallSite> {
    graph
        .call_graph
        .calls
        .iter()
        .filter(|(id, _)| id.name == caller)
        .flat_map(|(_, sites)| sites)
        .filter(|site| site.callee_name == target)
        .collect()
}

fn exact_site(graph: &CodePropertyGraph, caller: &str, callee: &str) {
    let sites = recorded(graph, caller, callee);
    assert_eq!(sites.len(), 1, "recorded {caller} -> {callee}");
    let resolved = graph.call_graph.resolve_call_site(sites[0]);
    assert_eq!(resolved.len(), 1, "{resolved:?}");
    assert_eq!(
        resolved[0].confidence,
        ResolutionConfidence::Exact,
        "{resolved:?}"
    );
    assert_eq!(resolved[0].target.name, callee);
    assert_eq!(resolved[0].kind, ResolutionKind::LocalDef, "{resolved:?}");
}

fn check(
    graph: &CodePropertyGraph,
    expected_edges: &[&str],
    counts: &[(&str, u64)],
) -> Result<(), String> {
    let mut actual: Vec<_> = graph
        .graph
        .edge_references()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(label) = *edge.weight() else {
                return None;
            };
            let CpgNode::Variable { function: from, .. } = graph.node(edge.source()) else {
                return None;
            };
            let CpgNode::Variable { function: to, .. } = graph.node(edge.target()) else {
                return None;
            };
            (from == "App" && to == "D").then(|| {
                let (from, _) = spell(graph, edge.source()).unwrap();
                let (to, use_node) = spell(graph, edge.target()).unwrap();
                assert_eq!(label, FlowConfidence::Exact, "{from} -> {to}");
                format!("{from} -> {to}({})", if use_node { "use" } else { "def" })
            })
        })
        .collect();
    actual.sort();
    let mut expected = expected_edges.to_vec();
    expected.sort();
    let mut errors = Vec::new();
    if actual != expected {
        errors.push(format!("edges expected {expected:?}, observed {actual:?}"));
    }
    let stats = serde_json::to_value(&graph.dfg_label_stats).unwrap();
    errors.extend(
        counts
            .iter()
            .filter_map(|(key, count)| check_counter(&stats, key, *count).err()),
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn check_cases(cases: &[(&str, &str, &[&str], &[(&str, u64)])]) {
    let mut errors = Vec::new();
    for (name, source, edges, counts) in cases {
        let built = graph("js", source);
        exact_site(&built, "App", "D");
        match check(&built, edges, counts) {
            Ok(()) => eprintln!("PROPS_FOCUSED {name} PASS"),
            Err(error) => {
                eprintln!("PROPS_FOCUSED {name} FAIL {error}");
                errors.push(name);
            }
        }
    }
    assert!(errors.is_empty(), "failed cases {errors:?}");
}

#[test]
fn props_s2_inventory_ignores_nondirect_and_jsx_only_sources() {
    for (name, ext, source) in [
        ("identifier", "js", "function D({a}) {\n  use(a);\n}\nfunction App(x) {\n  const obj = {a: x};\n  D(obj);\n}\n"),
        ("conditional", "js", "function D({a}) {\n  use(a);\n}\nfunction App(x) {\n  D(flag ? {a: x} : {});\n}\n"),
        ("jsx-only", "tsx", "function D({a}) {\n  use(a);\n}\nfunction App(x) {\n  return <D a={x}/>;\n}\n"),
        ("unrecorded", "tsx", "function App(x) {\n  return {a: x};\n}\n"),
    ] {
        let built = graph(ext, source);
        if name == "unrecorded" {
            assert!(!built.call_graph.calls.keys().any(|owner| owner.name == "App"));
        } else if name == "jsx-only" {
            let sites = recorded(&built, "App", "D");
            assert_eq!(sites.len(), 1);
            assert!(sites[0].jsx_element);
        } else {
            exact_site(&built, "App", "D");
        }
        assert_eq!(check(&built, &[], &[("sites_literal", 0)]), Ok(()), "{name}");
    }
}

#[test]
fn props_s2_inventory_exact_singleton_and_recorded_refusals() {
    let mut errors = Vec::new();
    for (name, source, callee, cardinality, confidence, counter, edges) in [
        ("exact", "function D({a}, {b}) {\n use(a, b);\n}\nfunction App(x, y) {\n D({a: x}, {b: y});\n}\n",
            "D", 1, Some(ResolutionConfidence::Exact), 0, &["App:x@5 -> D:a@1(def)", "App:y@5 -> D:b@1(def)"][..]),
        ("empty", "function App(x, y) {\n Missing({a: x}, {b: y});\n}\n",
            "Missing", 0, None, 1, &[]),
        ("nameonly", "class K {\n D({a}) {\n  return use(a);\n }\n}\nfunction App(o, x) {\n o.D({a: x});\n}\n",
            "D", 1, Some(ResolutionConfidence::NameOnly), 1, &[]),
        ("two-exact", "let D;\nif (cond) {\n D = function ({a}) { sink(a); };\n} else {\n D = function ({a}) { observe(a); };\n}\nfunction App(x) {\n D({a: x});\n}\n",
            "D", 2, Some(ResolutionConfidence::Exact), 1, &[]),
    ] {
        let built = graph("js", source);
        let sites = recorded(&built, "App", callee);
        assert_eq!(sites.len(), 1, "{name}");
        let resolved = built.call_graph.resolve_call_site(sites[0]);
        assert_eq!(resolved.len(), cardinality, "{name}: {resolved:?}");
        assert!(resolved.iter().all(|result| Some(result.confidence) == confidence), "{name}");
        if let Err(error) = check(&built, edges, &[("sites_literal", 1), ("refused_not_exact_singleton", counter)]) {
            errors.push(format!("{name}: {error}"));
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn props_s2_formal_presence_alias_default_and_spread_precedence() {
    check_cases(&[
        ("missing-formal", "function D() {\n use(0);\n}\nfunction App(x) {\n D({a: x});\n}\n", &[],
            &[("sites_literal", 1), ("refused_no_props_formal", 1)]),
        ("out-of-range", "function D({a}) {\n use(a);\n}\nfunction App(x, y) {\n D({a: x}, {b: y});\n}\n", &["App:x@5 -> D:a@1(def)"],
            &[("edges_to_def", 1), ("refused_no_props_formal", 1)]),
        ("root-array", "function D([a]) {\n use(a);\n}\nfunction App(x) {\n D({a: x});\n}\n", &[], &[("refused_no_props_formal", 1)]),
        ("root-rest", "function D(...{a}) {\n use(a);\n}\nfunction App(x) {\n D({a: x});\n}\n", &[], &[("refused_no_props_formal", 1)]),
        ("empty-formal", "function D({}, {b}) {\n use(b);\n}\nfunction App(x, y) {\n D({a: x}, {b: y});\n}\n", &["App:y@5 -> D:b@1(def)"],
            &[("refused_no_props_formal", 0), ("refused_key_not_read", 1)]),
        ("leaf-refused", "function D({a = effect()}) {\n use(a);\n}\nfunction App(x) {\n D({a: x});\n}\n", &[],
            &[("refused_no_props_formal", 0), ("refused_alias_pattern", 0), ("refused_key_not_read", 1)]),
        ("inert-default", "function D({a = null}, tail) {\n use(a, tail);\n}\nfunction App(x, y) {\n D({a: x}, y);\n}\n",
            &["App:x@5 -> D:a@1(def)"], &[("edges_to_def", 1), ("refused_alias_pattern", 0)]),
        ("alias-wins-tie", "function D(defaults, {a} = defaults, [b]) {\n use(a, b);\n}\nfunction App(x, y) {\n D(null, {a: x}, {b: y});\n}\n", &[],
            &[("sites_literal", 1), ("refused_alias_pattern", 1), ("refused_no_props_formal", 0)]),
        ("unsupported-wins-tie", "function D(defaults, [b], {a} = defaults) {\n use(b, a);\n}\nfunction App(x, y) {\n D(null, {b: x}, {a: y});\n}\n", &[],
            &[("sites_literal", 1), ("refused_alias_pattern", 0), ("refused_no_props_formal", 1)]),
        ("dense-spread", "function D({a}) {\n use(a);\n}\nfunction App(x) {\n D(...[], {a: x});\n}\n", &[],
            &[("sites_literal", 1), ("refused_argument_after_spread", 1), ("refused_no_props_formal", 0)]),
        ("earlier-safe", "function D({a}, {b}) {\n use(a, b);\n}\nfunction App(x, y) {\n D({a: x}, ...[], {b: y});\n}\n", &["App:x@5 -> D:a@1(def)"],
            &[("sites_literal", 1), ("edges_to_def", 1), ("refused_argument_after_spread", 1), ("refused_no_props_formal", 0)]),
        ("no-spread", "function D({a}, {b}) {\n use(a, b);\n}\nfunction App(x, y) {\n D({a: x}, {b: y});\n}\n",
            &["App:x@5 -> D:a@1(def)", "App:y@5 -> D:b@1(def)"], &[("edges_to_def", 2), ("refused_argument_after_spread", 0)]),
        ("mixed-plain-and-pattern", "function D(q, {a}) {\n use(q.b);\n use(a);\n}\nfunction App(x, y) {\n D({b: y}, {a: x});\n}\n",
            &["App:x@6 -> D:a@1(def)", "App:y@6 -> D:q.b@2(use)"], &[("edges_to_def", 1), ("edges_to_use", 1)]),
    ]);
}

#[test]
fn props_s2_typescript_this_does_not_consume_argument_position() {
    let source =
        "function D(this: void, {a}) {\n use(a);\n}\nfunction App(x: unknown) {\n D({a: x});\n}\n";
    let built = graph("ts", source);
    exact_site(&built, "App", "D");
    assert_eq!(
        check(
            &built,
            &["App:x@5 -> D:a@1(def)"],
            &[
                ("sites_literal", 1),
                ("edges_to_def", 1),
                ("refused_no_props_formal", 0)
            ]
        ),
        Ok(())
    );
}

#[test]
fn props_s2_pattern_entries_and_writer_displacements_are_counted_once() {
    check_cases(&[
        ("static-key-and-rest", "function D({a: v, ...rest}) {\n use(v, rest);\n}\nfunction App(x, y) {\n D({a: x, b: y});\n}\n",
            &["App:x@5 -> D:v@1(def)", "App:y@5 -> D:rest@1(def)"], &[("edges_to_def", 1), ("edges_to_rest", 1), ("refused_key_unknown", 0)]),
        ("unknown-entry-and-rest", "function D({['a']: v, ...rest}) {\n use(v, rest);\n}\nfunction App(x, y) {\n D({a: x, b: y});\n}\n", &[],
            &[("refused_key_unknown", 2), ("refused_key_not_read", 2), ("edges_to_rest", 0)]),
        ("unknown-empty-array-and-ancestor", "function D({[k]: {}, [j]: [v], [m]: {a, b}, keep, ...rest}) {\n use(v, a, b, keep, rest);\n}\nfunction App(x, y) {\n D({keep: x, extra: y});\n}\n", &["App:x@5 -> D:keep@1(def)"],
            &[("refused_key_unknown", 4), ("refused_key_not_read", 1)]),
        ("displaced-subtree-telemetry", "function D({box, keep}) {\n use(box, keep);\n}\nfunction App(x, y, z, w, obj) {\n D({box: {...obj, a: x, b: y}, box: {}, box: 1, keep: z});\n}\n",
            &["App:z@5 -> D:keep@1(def)"], &[("spread_sources_unbound", 1), ("refused_overwritten", 2), ("refused_value_not_data", 1)]),
        ("numeric-known-and-noncanonical", "function D({'1': v, keep}) {\n use(v, keep);\n}\nfunction App(x, y, z) {\n D({'1': x, Infinity: y, 1e0: x, 1: y, keep: z});\n}\n",
            &["App:z@5 -> D:keep@1(def)"], &[("refused_value_not_data", 0), ("refused_overwritten", 2)]),
        ("opaque-numeric-terminal-overwritten", "function D({'1': v, keep}) {\n use(v, keep);\n}\nfunction App(x, z) {\n D({1: 0, '1': x, keep: z});\n}\n",
            &["App:x@5 -> D:v@1(def)", "App:z@5 -> D:keep@1(def)"], &[("refused_overwritten", 1), ("refused_value_not_data", 0)]),
        ("accessor-final", "function D({a, keep}) {\n use(a, keep);\n}\nfunction App(x, y) {\n D({a: x, get a() { return x; }, keep: y});\n}\n",
            &["App:y@5 -> D:keep@1(def)"], &[("refused_overwritten", 1), ("refused_value_not_data", 1), ("refused_key_not_read", 0)]),
        ("legacy-visibility-and-read-kind", "function D(p) {\n observe(p);\n observe(p['a']);\n observe(p.a);\n}\nfunction App(x, y, z) {\n D({a: x, b: y, 'aria-label': z});\n}\n",
            &["App:x@7 -> D:p@2(use)", "App:y@7 -> D:p@2(use)", "App:x@7 -> D:p@3(use)", "App:y@7 -> D:p@3(use)", "App:x@7 -> D:p@4(use)"],
            &[("edges_to_bare_use", 2), ("edges_to_use", 3), ("refused_bracket_read", 0), ("refused_unproved_read", 0)]),
    ]);
}

#[test]
fn props_s2_namespace_export_query_refuses_injected_impostor() {
    let built = build(
        Language::TypeScript,
        &[
            (
                "m",
                "import * as api from './real';\nexport function App(x) {\n api.D({a: x});\n}\n",
            ),
            ("real", "export function D({a}) {\n return use(a);\n}\n"),
            ("impostor", "export function D({a}) {\n return use(a);\n}\n"),
        ],
        "ts",
    );
    let sites = recorded(&built, "App", "D");
    assert_eq!(sites.len(), 1);
    let site = sites[0];
    assert_eq!(site.qualifier.as_deref(), Some("api"));
    assert!(matches!(
        site.local_binding,
        JsLocalBinding::NamespaceImport { .. }
    ));
    let resolved = built.call_graph.resolve_call_site(site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(resolved[0].kind, ResolutionKind::ImportQualified);
    assert_eq!(resolved[0].target.file, "real.ts");
    assert!(built
        .call_graph
        .js_ts_namespace_export_matches(site, resolved[0].target));
    let impostor = built.call_graph.functions["D"]
        .iter()
        .find(|id| id.file == "impostor.ts")
        .unwrap();
    assert!(!built
        .call_graph
        .js_ts_namespace_export_matches(site, impostor));
}

#[test]
fn props_s2_recovered_literal_contributes_only_inventory() {
    let target = "export function D({a}) {\n return use(a);\n}\n";
    let mut failures = Vec::new();
    for (argument, recovered, edges, counters) in [
        (
            "{a: x, broken: }",
            true,
            &[][..],
            &[("sites_literal", 1)][..],
        ),
        (
            "{a: x}",
            false,
            &["App:x@3 -> D:a@1(def)"][..],
            &[("sites_literal", 1), ("edges_to_def", 1)][..],
        ),
    ] {
        let caller = format!("import {{D}} from './target';\nexport function App(x) {{\n return D({argument});\n}}\n");
        let files: BTreeMap<_, _> = [("caller.js", caller.as_str()), ("target.js", target)]
            .into_iter()
            .map(|(file, source)| {
                (
                    file.to_owned(),
                    ParsedFile::parse(file, source, Language::JavaScript).unwrap(),
                )
            })
            .collect();
        assert_eq!(files["caller.js"].tree.root_node().has_error(), recovered);
        let built = CodePropertyGraph::build(&files);
        let sites = recorded(&built, "App", "D");
        assert_eq!(sites.len(), 1);
        let resolved = built.call_graph.resolve_call_site(sites[0]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target.file, "target.js");
        assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(resolved[0].kind, ResolutionKind::ImportMember);
        if let Err(error) = check(&built, edges, counters) {
            failures.push(format!("{argument}: {error}"));
        }
        let actual: BTreeMap<_, _> = built
            .dfg_label_stats
            .dfg_props
            .iter()
            .filter(|(_, value)| **value != 0)
            .map(|(name, value)| (name.as_str(), *value))
            .collect();
        if recovered && actual != BTreeMap::from([("sites_literal", 1)]) {
            failures.push(format!("recovered positive counters: {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}
