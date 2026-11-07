use super::*;
use crate::languages::Language;

fn build_go(source: &str) -> CallGraph {
    let files = BTreeMap::from([(
        "callback.go".to_string(),
        ParsedFile::parse("callback.go", source, Language::Go).expect("parse Go fixture"),
    )]);
    CallGraph::build(&files)
}

fn build_go_module(files: &[(&str, &str)], module: &str) -> CallGraph {
    let files = files
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let root = tempfile::tempdir().expect("temporary Go module");
    std::fs::write(
        root.path().join("go.mod"),
        format!("module {module}\n\ngo 1.24\n"),
    )
    .expect("write go.mod fixture");
    let inputs = crate::repo_loader::scope_graph_build_inputs(root.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn function<'a>(graph: &'a CallGraph, name: &str) -> &'a FunctionId {
    graph.functions[name].first().expect("function")
}

#[test]
fn level3_literal_callback_mints_exact_target_at_hof_invocation() {
    let graph = build_go(
            "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { invoke(safe) }\n",
        );
    let invoke = function(&graph, "invoke");
    let safe = function(&graph, "safe");
    let sites = graph
        .calls
        .get(invoke)
        .into_iter()
        .flatten()
        .filter(|site| site.origin == CallSiteOrigin::IndirectResolution)
        .collect::<Vec<_>>();

    assert_eq!(graph.level3_indirect_resolved, 1);
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].callee_name, "safe");
    assert_eq!(sites[0].source_callee_name.as_deref(), Some("cb"));
    assert_eq!(sites[0].pre_resolved_target.as_ref(), Some(safe));
    assert_eq!(graph.go_level3_b1_telemetry.candidates, 1);
    assert_eq!(graph.go_level3_b1_telemetry.exact_inbound_sites, 1);
    assert_eq!(graph.go_level3_b1_telemetry.accepted_inbound_sites, 1);
    assert_eq!(graph.go_level3_b1_telemetry.unique_targets, 1);
    assert_eq!(graph.go_level3_b1_telemetry.edges, 1);
    assert!(graph.go_level3_b1_telemetry.drops.is_empty());
    let resolved = graph.resolve_call_site_full(sites[0]);
    assert_eq!(resolved.resolved.len(), 1);
    assert_eq!(resolved.resolved[0].target, safe);
    assert_eq!(
        resolved.resolved[0].confidence,
        crate::resolution::ResolutionConfidence::Exact
    );
    assert_eq!(
        resolved.resolved[0].kind,
        crate::resolution::ResolutionKind::ParameterCallback
    );
}

#[test]
fn level3_named_grouped_callback_uses_nonzero_slot() {
    let graph = build_go(
            "package p\ntype Handler func(int)\nfunc invoke(prefix string, cb Handler) { cb(1) }\nfunc safe(value int) {}\nfunc caller() { invoke(\"x\", safe) }\n",
        );
    let invoke = function(&graph, "invoke");
    let safe = function(&graph, "safe");

    assert!(graph.calls[invoke].iter().any(|site| {
        site.origin == CallSiteOrigin::IndirectResolution
            && site.pre_resolved_target.as_ref() == Some(safe)
    }));
}

#[test]
fn level3_qualified_hof_requires_exact_import_identity() {
    let graph = build_go_module(
            &[
                (
                    "lib/hof.go",
                    "package lib\nfunc Invoke(cb func()) { cb() }\n",
                ),
                (
                    "app/main.go",
                    "package app\nimport callback \"example.com/project/lib\"\nfunc safe() {}\nfunc caller() { callback.Invoke(safe) }\n",
                ),
            ],
            "example.com/project",
        );
    let invoke = graph.functions["Invoke"]
        .iter()
        .find(|function| function.file == "lib/hof.go")
        .expect("Invoke");
    let safe = graph.functions["safe"]
        .iter()
        .find(|function| function.file == "app/main.go")
        .expect("safe");

    assert!(graph.calls[invoke].iter().any(|site| {
        site.origin == CallSiteOrigin::IndirectResolution
            && site.source_callee_name.as_deref() == Some("cb")
            && site.pre_resolved_target.as_ref() == Some(safe)
    }));
}

#[test]
fn level3_negative_cases_conserve_one_stable_drop() {
    for (label, source, expected_drop) in [
            (
                "target shadow",
                "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { safe := func() {}; invoke(safe) }\n",
                "local_binding_or_mutation",
            ),
            (
                "hof shadow",
                "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { invoke := func(func()) {}; invoke(safe) }\n",
                "local_binding_or_mutation",
            ),
            (
                "signature mismatch",
                "package p\nfunc invoke(cb func(int)) { cb(1) }\nfunc safe(string) {}\nfunc caller() { invoke(safe) }\n",
                "signature_unproven_or_mismatch",
            ),
            (
                "parameter mutation",
                "package p\nfunc safe() {}\nfunc other() {}\nfunc invoke(cb func()) { cb = other; cb() }\nfunc caller() { invoke(safe) }\n",
                "callback_parameter_shadow_or_mutation",
            ),
            (
                "address escape",
                "package p\nfunc mutate(*func()) {}\nfunc safe() {}\nfunc invoke(cb func()) { mutate(&cb); cb() }\nfunc caller() { invoke(safe) }\n",
                "callback_parameter_address_escape",
            ),
            (
                "nested invocation",
                "package p\nfunc safe() {}\nfunc invoke(cb func()) { func() { cb() }() }\nfunc caller() { invoke(safe) }\n",
                "nested_callable_invocation",
            ),
            (
                "non-bare argument",
                "package p\nfunc invoke(cb func()) { cb() }\nfunc caller() { invoke(func() {}) }\n",
                "non_bare_argument",
            ),
            (
                "unknown default import",
                "package p\nimport \"example.com/external\"\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { invoke(safe) }\n",
                "strict_import_name_unavailable",
            ),
            (
                "dot import",
                "package p\nimport . \"example.com/external\"\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { invoke(safe) }\n",
                "dot_import",
            ),
            (
                "no direct invocation",
                "package p\nfunc invoke(cb func()) {}\nfunc safe() {}\nfunc caller() { invoke(safe) }\n",
                "no_direct_parameter_invocation",
            ),
            (
                "spread argument",
                "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { callbacks := []func(){safe}; invoke(callbacks...) }\n",
                "missing_slot_or_argument_identity",
            ),
        ] {
            let graph = build_go(source);
            assert_eq!(graph.level3_indirect_resolved, 0, "{label}");
            assert_eq!(graph.go_level3_b1_telemetry.candidates, 1, "{label}");
            assert_eq!(
                graph.go_level3_b1_telemetry.accepted_inbound_sites,
                0,
                "{label}"
            );
            assert_eq!(
                graph
                    .go_level3_b1_telemetry
                    .drops
                    .values()
                    .sum::<usize>(),
                1,
                "{label}"
            );
            assert_eq!(
                graph.go_level3_b1_telemetry.drops.get(expected_drop),
                Some(&1),
                "{label}: {:?}",
                graph.go_level3_b1_telemetry.drops
            );
        }
}

#[test]
fn level3_variadic_callback_slot_is_not_a_candidate() {
    let graph = build_go(
            "package p\nfunc invoke(callbacks ...func()) { callbacks[0]() }\nfunc safe() {}\nfunc caller() { invoke(safe) }\n",
        );

    assert_eq!(graph.level3_indirect_resolved, 0);
    assert_eq!(graph.go_level3_b1_telemetry.candidates, 0);
}

#[test]
fn level3_preserves_multiple_slots_targets_and_invocation_spans() {
    let graph = build_go(
            "package p\nfunc invoke(left, right func()) { left(); right(); left() }\nfunc first() {}\nfunc second() {}\nfunc caller() { invoke(first, second) }\n",
        );
    let invoke = function(&graph, "invoke");
    let indirect = graph.calls[invoke]
        .iter()
        .filter(|site| site.origin == CallSiteOrigin::IndirectResolution)
        .collect::<Vec<_>>();

    assert_eq!(graph.go_level3_b1_telemetry.candidates, 2);
    assert_eq!(graph.go_level3_b1_telemetry.accepted_inbound_sites, 2);
    assert_eq!(graph.go_level3_b1_telemetry.unique_targets, 2);
    assert_eq!(graph.go_level3_b1_telemetry.edges, 3);
    assert_eq!(indirect.len(), 3);
    assert_eq!(
        indirect
            .iter()
            .filter(|site| site.callee_name == "first")
            .map(|site| site.start_byte)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    assert_eq!(
        indirect
            .iter()
            .filter(|site| site.callee_name == "second")
            .count(),
        1
    );
}

#[test]
fn level3_multiple_inbound_targets_share_one_hof_invocation_without_collapsing_identity() {
    let graph = build_go(
            "package p\nfunc invoke(cb func()) { cb() }\nfunc first() {}\nfunc second() {}\nfunc caller1() { invoke(first) }\nfunc caller2() { invoke(second) }\n",
        );
    let invoke = function(&graph, "invoke");
    let targets = graph.calls[invoke]
        .iter()
        .filter(|site| site.origin == CallSiteOrigin::IndirectResolution)
        .filter_map(|site| site.pre_resolved_target.as_ref())
        .map(|target| target.name.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(targets, BTreeSet::from(["first", "second"]));
    assert_eq!(graph.go_level3_b1_telemetry.candidates, 2);
    assert_eq!(graph.go_level3_b1_telemetry.accepted_inbound_sites, 2);
    assert_eq!(graph.go_level3_b1_telemetry.edges, 2);
}

#[test]
fn level3_same_line_inbound_calls_preserve_distinct_candidate_spans() {
    let graph = build_go(
            "package p\nfunc invoke(cb func()) { cb() }\nfunc first() {}\nfunc second() {}\nfunc caller() { invoke(first); invoke(second) }\n",
        );
    let starts = graph
        .go_level3_b1_telemetry
        .sites
        .iter()
        .map(|site| site.inbound_start_byte)
        .collect::<BTreeSet<_>>();

    assert_eq!(graph.go_level3_b1_telemetry.candidates, 2);
    assert_eq!(graph.go_level3_b1_telemetry.accepted_inbound_sites, 2);
    assert_eq!(starts.len(), 2);
}

#[test]
fn level3_same_named_targets_in_two_packages_keep_exact_file_identity() {
    let graph = build_go_module(
            &[
                ("hof/hof.go", "package hof\nfunc Invoke(cb func()) { cb() }\n"),
                (
                    "left/caller.go",
                    "package left\nimport h \"example.com/project/hof\"\nfunc safe() {}\nfunc caller() { h.Invoke(safe) }\n",
                ),
                (
                    "right/caller.go",
                    "package right\nimport h \"example.com/project/hof\"\nfunc safe() {}\nfunc caller() { h.Invoke(safe) }\n",
                ),
            ],
            "example.com/project",
        );
    let invoke = graph.functions["Invoke"]
        .iter()
        .find(|function| function.file == "hof/hof.go")
        .expect("Invoke");
    let target_files = graph.calls[invoke]
        .iter()
        .filter(|site| site.origin == CallSiteOrigin::IndirectResolution)
        .filter_map(|site| site.pre_resolved_target.as_ref())
        .map(|target| target.file.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        target_files,
        BTreeSet::from(["left/caller.go", "right/caller.go"])
    );
    assert_eq!(graph.go_level3_b1_telemetry.unique_targets, 2);
    assert_eq!(graph.go_level3_b1_telemetry.edges, 2);
}

#[test]
fn level3_test_file_inbound_site_drops_without_an_edge() {
    let graph = build_go_module(
        &[
            (
                "p/callback.go",
                "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\n",
            ),
            (
                "p/callback_test.go",
                "package p\nfunc testCaller() { invoke(safe) }\n",
            ),
        ],
        "example.com/project",
    );

    assert_eq!(graph.go_level3_b1_telemetry.candidates, 1);
    assert_eq!(graph.go_level3_b1_telemetry.accepted_inbound_sites, 0);
    assert_eq!(
        graph.go_level3_b1_telemetry.drops.get("non_go_or_test"),
        Some(&1)
    );
    assert_eq!(graph.go_level3_b1_telemetry.edges, 0);
}
