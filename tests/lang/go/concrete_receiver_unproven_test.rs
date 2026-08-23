use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{DropReason, ResolutionConfidence, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

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

fn resolved_files(cg: &CallGraph) -> BTreeSet<String> {
    cg.resolve_call_site_full(site(cg))
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
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

fn manifest_site(cg: &CallGraph) -> serde_json::Value {
    let manifest = prism::navigation::queries::interface_dispatch_manifest(cg);
    let sites = manifest["sites"].as_array().expect("manifest sites");
    assert_eq!(sites.len(), 1, "{manifest:#}");
    sites[0].clone()
}

fn manifest_target_files(cg: &CallGraph) -> BTreeSet<String> {
    manifest_site(cg)["implementer_identities"]
        .as_array()
        .expect("implementer identities")
        .iter()
        .map(|identity| {
            identity["file"]
                .as_str()
                .expect("implementer file")
                .to_string()
        })
        .collect()
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
fn ambiguous_import_basename_keeps_the_legacy_bare_interface_hit() {
    let cg = ambiguous_basename_fixture(true);
    let outcome = cg.resolve_call_site_full(site(&cg));

    assert_eq!(
        resolved_files(&cg),
        BTreeSet::from(["iface/types.go".to_string()]),
        "{outcome:?}"
    );
    assert!(outcome.resolved.iter().all(|resolved| {
        resolved.confidence == ResolutionConfidence::Exact
            && resolved.kind == ResolutionKind::InterfaceDispatch
    }));
    assert_bare_telemetry(&cg, 1, 1, 1);
    assert_eq!(manifest_site(&cg)["dispatch_route"], "interface_dispatch");
    assert_eq!(manifest_target_files(&cg), resolved_files(&cg));
}

#[test]
fn ambiguous_import_basename_keeps_the_legacy_no_interface_drop() {
    let cg = ambiguous_basename_fixture(false);
    let outcome = cg.resolve_call_site_full(site(&cg));

    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_bare_telemetry(&cg, 1, 0, 0);
    let manifest = manifest_site(&cg);
    assert_eq!(manifest["dispatch_route"], "unproven_drop");
    assert_eq!(manifest["implementer_identities"], serde_json::json!([]));
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
    build_go(&sources)
}

#[test]
fn duplicate_profile_owner_keeps_the_legacy_bare_interface_hit() {
    let cg = duplicate_profile_fixture(true, true);
    let outcome = cg.resolve_call_site_full(site(&cg));

    assert_eq!(
        resolved_files(&cg),
        BTreeSet::from(["iface/types.go".to_string()]),
        "{outcome:?}"
    );
    assert!(outcome
        .resolved
        .iter()
        .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch));
    assert_bare_telemetry(&cg, 1, 1, 1);
    assert_eq!(manifest_site(&cg)["dispatch_route"], "interface_dispatch");
    assert_eq!(manifest_target_files(&cg), resolved_files(&cg));
}

#[test]
fn duplicate_profile_owner_keeps_the_legacy_no_interface_drop() {
    let cg = duplicate_profile_fixture(false, false);
    let outcome = cg.resolve_call_site_full(site(&cg));

    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_bare_telemetry(&cg, 1, 0, 0);
    let manifest = manifest_site(&cg);
    assert_eq!(manifest["dispatch_route"], "unproven_drop");
    assert_eq!(manifest["implementer_identities"], serde_json::json!([]));
}
