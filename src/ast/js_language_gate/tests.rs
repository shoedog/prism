use super::fixtures;
use super::{legacy, ADMISSIONS};
use crate::ast::ParsedFile;
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

fn nodes(p: &ParsedFile) -> Vec<Node<'_>> {
    let mut pending = vec![p.tree.root_node()];
    let mut nodes = Vec::new();
    while let Some(node) = pending.pop() {
        nodes.push(node);
        let mut cursor = node.walk();
        pending.extend(node.named_children(&mut cursor));
    }
    nodes
}

#[test]
fn family_and_extensions() {
    for language in Language::all() {
        assert_eq!(
            language.is_js_family(),
            [Language::JavaScript, Language::TypeScript, Language::Tsx].contains(&language)
        );
    }
    for ext in ["js", "jsx", "mjs", "cjs", "ts", "tsx"] {
        assert!(Language::from_extension(ext).unwrap().is_js_family());
    }
}

#[test]
fn non_js_helpers_are_neutral() {
    for (path, language, source) in fixtures::non_js() {
        let p = ParsedFile::parse(path, &source, language).unwrap();
        assert!(!p.tree.root_node().has_error(), "fixture parse: {path}");
        let mut owners = p.all_functions();
        owners.push(p.tree.root_node()); // Also exercise a non-callable search owner.
        ADMISSIONS.with(|n| n.set(0));
        for owner in owners {
            assert!(p.js_ts_parameter_refusals(&owner).is_empty());
            assert!(!p.js_ts_seam_binding(&owner, "x"));
            for node in nodes(&p) {
                let (s, e) = (node.start_byte(), node.end_byte());
                assert_eq!(p.js_ts_def_scope(&owner, "x", s), None, "{path}");
                assert_eq!(p.js_ts_def_scope_id(s, "x"), None, "{path}");
                assert!(!p.js_ts_occurrence_binds_formal(&owner, "x", s), "{path}");
                assert!(!p.js_ts_span_in_own_scope(&owner, s, e), "{path}");
                assert!(!p.js_ts_span_is_non_reference(s, e), "{path}");
                assert!(!p.js_ts_span_is_not_a_read(s, e), "{path}");
                assert_eq!(p.js_ts_own_alias_target(s, e), None, "{path}");
                assert!(!p.js_ts_identifier_is_not_a_read(node), "{path}");
                assert!(!p.js_ts_this_rebound_between(&node, &owner), "{path}");
                assert!(!p.js_ts_span_this_rebound_between(s, e, &owner), "{path}");
                assert!(!p.js_ts_span_in_with_body(s, e), "{path}");
                assert!(!p.js_ts_reference_in_with_body(node), "{path}");
                assert!(!p.js_ts_reference_fenced(&node, &owner, "x", s), "{path}");
            }
        }
        assert_eq!(ADMISSIONS.with(|n| n.get()), 0, "JS body admitted: {path}");
    }
}

#[test]
fn js_fixtures_match_legacy() {
    for (path, language, source) in fixtures::js() {
        let p = ParsedFile::parse(path, source, language).unwrap();
        assert!(!p.tree.root_node().has_error(), "fixture parse: {path}");
        let owner = p.all_functions()[0];
        let body = owner.child_by_field_name("body").unwrap();
        assert!(p.js_ts_span_in_own_scope(&owner, body.start_byte(), body.end_byte()));
        compare_dfg(p, true);
    }
}

fn compare_dfg(p: ParsedFile, js: bool) {
    let path = p.path.clone();
    let has_owner = !p.all_functions().is_empty();
    let files = BTreeMap::from([(path.clone(), p)]);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    pool.install(|| {
        ADMISSIONS.with(|n| n.set(0));
        let after = DataFlowGraph::build_on_current_pool(&files);
        let after_admissions = ADMISSIONS.with(|n| n.replace(0));
        let before = legacy(|| DataFlowGraph::build_on_current_pool(&files));
        let before_admissions = ADMISSIONS.with(|n| n.replace(0));
        assert_eq!(after_admissions > 0, js && has_owner, "{path}");
        if has_owner {
            assert!(before_admissions > 0, "reference bypass missed worker: {path}");
            assert!(!after.edges().is_empty(), "vacuous comparison: {path}");
        }
        // Full serialization includes defs/uses, occurrence bytes, all edges,
        // labels (legacy and exact), indices and per-function RD statistics.
        assert_eq!(bincode::serialize(&after).unwrap(), bincode::serialize(&before).unwrap(), "complete DFG: {path}");
        println!("DFG {path}: edges={}, labels={}, exact_labels={}, stats={:?}; byte-identical; admitted after={after_admissions}, before={before_admissions}", after.edges().len(), after.labels().count(), after.exact_labels().count(), after.rd_function_stats());
    });
}

#[test]
fn non_js_dfg_matches_legacy() {
    for (path, language, source) in fixtures::non_js() {
        let p = ParsedFile::parse(path, &source, language).unwrap();
        assert!(!p.tree.root_node().has_error(), "fixture parse: {path}");
        compare_dfg(p, false);
    }
}

#[test]
fn legacy_helper_differences_are_explicit() {
    let mut differing = BTreeSet::new();
    for (path, language, source) in fixtures::non_js() {
        let p = ParsedFile::parse(path, &source, language).unwrap();
        let mut owners = p.all_functions();
        owners.push(p.tree.root_node());
        let mut counts = BTreeMap::<&str, usize>::new();
        for owner in owners {
            for node in nodes(&p) {
                let (s, e) = (node.start_byte(), node.end_byte());
                let old = legacy(|| {
                    [
                        ("own_scope", p.js_ts_span_in_own_scope(&owner, s, e)),
                        ("def_scope", p.js_ts_def_scope(&owner, "x", s).is_some()),
                        ("binding_at", p.js_ts_def_scope_id(s, "x").is_some()),
                        ("formal", p.js_ts_occurrence_binds_formal(&owner, "x", s)),
                        ("non_reference", p.js_ts_span_is_non_reference(s, e)),
                        ("not_read", p.js_ts_span_is_not_a_read(s, e)),
                        ("alias", p.js_ts_own_alias_target(s, e).is_some()),
                        (
                            "identifier_not_read",
                            p.js_ts_identifier_is_not_a_read(node),
                        ),
                        ("this_rebound", p.js_ts_this_rebound_between(&node, &owner)),
                        ("with_body", p.js_ts_span_in_with_body(s, e)),
                    ]
                });
                for (helper, changed) in old {
                    if changed {
                        *counts.entry(helper).or_default() += 1;
                        differing.insert(helper);
                    }
                }
            }
        }
        println!("legacy non-neutral {path}: {counts:?}; gated verdicts all neutral");
    }
    for expected in [
        "own_scope",
        "def_scope",
        "binding_at",
        "non_reference",
        "not_read",
        "alias",
        "identifier_not_read",
        "this_rebound",
        "with_body",
    ] {
        assert!(
            differing.contains(expected),
            "missing collision witness: {expected}"
        );
    }
}

#[test]
fn legacy_guard_restores_after_nesting_and_unwind() {
    let p = ParsedFile::parse("empty.go", "package fixture", Language::Go).unwrap();
    assert!(!p.js_ts_binding_enabled());
    legacy(|| {
        assert!(p.js_ts_binding_enabled());
        legacy(|| assert!(p.js_ts_binding_enabled()));
        assert!(p.js_ts_binding_enabled());
    });
    let result = std::panic::catch_unwind(|| legacy(|| panic!("oracle unwind")));
    assert!(result.is_err());
    assert!(!p.js_ts_binding_enabled());
}
