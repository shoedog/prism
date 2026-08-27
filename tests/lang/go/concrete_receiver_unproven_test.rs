use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::DropReason;
use std::collections::BTreeMap;

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    CallGraph::build(&files)
}

fn site(cg: &CallGraph) -> &CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "run" && site.callee_name == "M")
        .expect("missing run->M call site")
}

fn assert_bare_telemetry(cg: &CallGraph, sites: usize, hits: usize, edges: usize) {
    let stats = prism::navigation::queries::call_stats(cg);
    assert_eq!(
        stats["go_unproven_receiver_bare_fallback_sites"],
        serde_json::json!(sites)
    );
    assert_eq!(
        stats["go_unproven_receiver_bare_fallback_hits"],
        serde_json::json!(hits)
    );
    assert_eq!(
        stats["go_unproven_receiver_bare_fallback_edges"],
        serde_json::json!(edges)
    );
}

fn assert_terminal_prerequisite_drop(cg: &CallGraph, reason: &str) {
    let call = site(cg);
    let outcome = cg.resolve_call_site_full(call);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(call.receiver_type, None, "{call:?}");
    assert!(call.receiver_materialized, "{call:?}");
    assert_bare_telemetry(cg, 0, 0, 0);
    assert_eq!(
        prism::navigation::queries::call_stats(cg)["go_receiver_prereq_drops"][reason],
        1
    );
    let manifest = prism::navigation::queries::interface_dispatch_manifest(cg);
    assert!(
        manifest["sites"]
            .as_array()
            .expect("manifest sites")
            .is_empty(),
        "{manifest:#}"
    );
}

fn ambiguous_basename_fixture(with_interface: bool) -> CallGraph {
    let mut sources = vec![
        (
            "one/q/types.go",
            "package q\n\
             type S struct{}\n\
             func (S) M() {}\n",
        ),
        ("two/q/types.go", "package q\ntype Other struct{}\n"),
        (
            "unrelated/types.go",
            "package unrelated\ntype Other interface { M() }\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import q \"example/one/q\"\n\
             func run(s q.S) { s.M() }\n",
        ),
    ];
    if with_interface {
        sources.push((
            "iface/types.go",
            "package iface\n\
             type S interface { M() }\n\
             type Wrong struct{}\n\
             func (Wrong) M() {}\n\
             func retain() { _ = Wrong{} }\n",
        ));
    }
    build_go(&sources)
}

#[test]
fn ambiguous_import_basename_with_interface_stops_at_prerequisite_membrane() {
    let cg = ambiguous_basename_fixture(true);
    assert_terminal_prerequisite_drop(&cg, "strict_import_unresolved");
}

#[test]
fn ambiguous_import_basename_without_interface_stops_at_prerequisite_membrane() {
    let cg = ambiguous_basename_fixture(false);
    assert_terminal_prerequisite_drop(&cg, "strict_import_unresolved");
}

fn duplicate_profile_fixture(identical: bool, with_interface: bool) -> CallGraph {
    let mut sources = vec![
        (
            "q/s_linux.go",
            "//go:build linux\n\n\
             package q\n\
             type S struct{}\n",
        ),
        (
            "q/s_darwin.go",
            if identical {
                "//go:build darwin\n\npackage q\ntype S struct{}\n"
            } else {
                "//go:build darwin\n\npackage q\ntype S struct{ Value int }\n"
            },
        ),
        (
            "q/method.go",
            "package q\n\
             func (S) M() {}\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import q \"example/q\"\n\
             func run(s q.S) { s.M() }\n",
        ),
        (
            "unrelated/types.go",
            "package unrelated\ntype Other interface { M() }\n",
        ),
    ];
    if with_interface {
        sources.push((
            "iface/types.go",
            "package iface\n\
             type S interface { M() }\n\
             type Wrong struct{}\n\
             func (Wrong) M() {}\n\
             func retain() { _ = Wrong{} }\n",
        ));
    }
    super::test_support::build_go(&sources)
}

#[test]
fn duplicate_profile_owner_with_interface_stops_at_prerequisite_membrane() {
    let cg = duplicate_profile_fixture(true, true);
    assert_terminal_prerequisite_drop(&cg, "declaration_unproven");
}

#[test]
fn duplicate_profile_owner_without_interface_stops_at_prerequisite_membrane() {
    let cg = duplicate_profile_fixture(false, false);
    assert_terminal_prerequisite_drop(&cg, "declaration_unproven");
}

#[test]
fn ambiguous_owner_embedded_interface_stops_at_prerequisite_membrane() {
    let cg = super::test_support::build_go(&[
        (
            "q/one.go",
            "package q\n\
             type I interface{ M() }\n\
             type S struct{ I }\n\
             type Good struct{}\n\
             func (Good) M() {}\n\
             func retain() { _ = Good{} }\n",
        ),
        ("q/two.go", "package q\ntype S struct{ I }\n"),
        (
            "app/use.go",
            "package app\n\
             import q \"example/q\"\n\
             func run(s q.S) { s.M() }\n",
        ),
    ]);
    assert_terminal_prerequisite_drop(&cg, "declaration_unproven");
}
