use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{DropReason, ResolutionKind};
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

fn site<'a>(cg: &'a CallGraph, caller: &str, method: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .unwrap_or_else(|| panic!("missing {caller}->{method}"))
}

fn assert_manifest_route(cg: &CallGraph, expected: &str) {
    let manifest = prism::navigation::queries::interface_dispatch_manifest(cg);
    let sites = manifest["sites"].as_array().expect("manifest sites");
    assert_eq!(sites.len(), 1, "{manifest:#}");
    assert_eq!(sites[0]["dispatch_route"], serde_json::json!(expected));
}

fn resolver_target_files(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    cg.resolve_call_site_full(site(cg, caller, method))
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
}

fn manifest_target_files(cg: &CallGraph) -> BTreeSet<String> {
    let manifest = prism::navigation::queries::interface_dispatch_manifest(cg);
    let sites = manifest["sites"].as_array().expect("manifest sites");
    assert_eq!(sites.len(), 1, "{manifest:#}");
    sites[0]["implementer_identities"]
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

#[test]
fn manifest_pins_concrete_direct_with_resolver_telemetry() {
    let cg = build_go(&[
        (
            "q/types.go",
            "package q\ntype A struct{}\nfunc (A) M() {}\n",
        ),
        (
            "p/types.go",
            "package p\ntype A interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\n",
        ),
        (
            "app/use.go",
            "package app\nimport q \"example/q\"\nfunc run(a q.A) { a.M() }\n",
        ),
    ]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, None);
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 1);
    assert_eq!(outcome.resolved[0].target.file, "q/types.go");
    assert_manifest_route(&cg, "concrete_direct");
}

#[test]
fn manifest_pins_promoted_snapshot_hit_with_resolver_telemetry() {
    let cg = build_go(&[
        (
            "q/types.go",
            "package q\ntype B struct{}\nfunc (B) M() {}\ntype S struct{ B }\n",
        ),
        ("p/types.go", "package p\ntype Marker interface{ M() }\n"),
        (
            "app/use.go",
            "package app\nimport q \"example/q\"\nfunc run(s q.S) { s.M() }\n",
        ),
    ]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.file, "q/types.go");
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::EmbeddedPromotion);
    assert_eq!(outcome.telemetry.go_promoted_snapshot_hits, 1);
    assert_eq!(outcome.telemetry.go_concrete_receiver_promoted_deferred, 0);
    assert_manifest_route(&cg, "concrete_promoted_snapshot");
}

#[test]
fn manifest_pins_existing_promoted_edge_with_resolver_telemetry() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type Marker interface{ M() }\n\
         type B struct{}\n\
         func (B) M() {}\n\
         type S struct{ B }\n\
         func run(s S) { s.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.file, "main.go");
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::EmbeddedPromotion);
    assert_eq!(outcome.telemetry.go_concrete_receiver_promoted_existing, 1);
    assert_manifest_route(&cg, "concrete_promoted");
}

#[test]
fn manifest_pins_interface_dispatch_with_resolver_targets() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type I interface{ M() }\n\
         type Good struct{}\n\
         func (Good) M() {}\n\
         func retain() { _ = Good{} }\n\
         func run(i I) { i.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, None);
    assert!(outcome
        .resolved
        .iter()
        .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch));
    assert_manifest_route(&cg, "interface_dispatch");
    assert_eq!(
        manifest_target_files(&cg),
        resolver_target_files(&cg, "run", "M")
    );
}

#[test]
fn manifest_pins_embedded_interface_dispatch_with_resolver_targets() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type I interface{ M() }\n\
         type S struct{ I }\n\
         type Good struct{}\n\
         func (Good) M() {}\n\
         func retain() { _ = Good{} }\n\
         func run(s S) { s.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, None);
    assert!(outcome
        .resolved
        .iter()
        .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch));
    assert_manifest_route(&cg, "embedded_interface_dispatch");
    assert_eq!(
        manifest_target_files(&cg),
        resolver_target_files(&cg, "run", "M")
    );
}

#[test]
fn pointer_embedded_interface_never_supplies_an_s4_selector() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type I interface{ M() }\n\
         type S struct{ *I }\n\
         type Wrong struct{}\n\
         func (Wrong) M() {}\n\
         func retain() { _ = Wrong{} }\n\
         func run(s S) { s.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ConcreteReceiverNoSelector));
    assert_eq!(outcome.telemetry.go_concrete_receiver_no_selector_drop, 1);
    assert_manifest_route(&cg, "concrete_no_selector_drop");
    assert!(manifest_target_files(&cg).is_empty());
}

#[test]
fn shallower_embedded_interface_wins_over_deeper_concrete_supplier() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type I interface{ M() }\n\
         type C struct{}\n\
         func (C) M() {}\n\
         type Inner struct{ C }\n\
         type S struct{ I; Inner }\n\
         type Good struct{}\n\
         func (Good) M() {}\n\
         func retain() { _ = Good{} }\n\
         func run(s S) { s.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert!(!outcome.resolved.is_empty(), "{outcome:?}");
    assert!(outcome
        .resolved
        .iter()
        .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch));
    assert_manifest_route(&cg, "embedded_interface_dispatch");
}

#[test]
fn shallower_embedded_concrete_wins_over_deeper_interface_supplier() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type I interface{ M() }\n\
         type C struct{}\n\
         func (C) M() {}\n\
         type Inner struct{ I }\n\
         type S struct{ C; Inner }\n\
         func run(s S) { s.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::EmbeddedPromotion);
    assert_manifest_route(&cg, "concrete_promoted");
}

#[test]
fn equal_depth_embedded_interface_and_concrete_is_ambiguous() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type I interface{ M() }\n\
         type C struct{}\n\
         func (C) M() {}\n\
         type S struct{ I; C }\n\
         func run(s S) { s.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ConcreteReceiverNoSelector));
    assert_eq!(outcome.telemetry.go_concrete_receiver_no_selector_drop, 1);
    assert_manifest_route(&cg, "concrete_no_selector_drop");
}

#[test]
fn manifest_pins_func_value_field_with_resolver_target() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type Runner interface{ Run() }\n\
         type Command struct{ Run func() }\n\
         func worker() {}\n\
         func New() Command { return Command{Run: worker} }\n\
         func invoke() { c := New(); c.Run() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "invoke", "Run"));

    assert_eq!(outcome.drop, None);
    assert_eq!(outcome.resolved.len(), 1);
    assert_eq!(outcome.resolved[0].target.name, "worker");
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::FuncValueField);
    assert_manifest_route(&cg, "func_value_field");
}

#[test]
fn named_func_value_field_keeps_p5_and_manifest_route() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type Runner interface{ Run() }\n\
         type H func()\n\
         type Command struct{ Run H }\n\
         func worker() {}\n\
         func New() Command { return Command{Run: worker} }\n\
         func invoke() { c := New(); c.Run() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "invoke", "Run"));

    assert_eq!(outcome.drop, None, "{outcome:?}");
    assert_eq!(outcome.resolved.len(), 1, "{outcome:?}");
    assert_eq!(outcome.resolved[0].target.name, "worker");
    assert_eq!(outcome.resolved[0].kind, ResolutionKind::FuncValueField);
    assert_manifest_route(&cg, "func_value_field");
}

#[test]
fn pointer_to_named_func_field_is_not_callable_p5() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type Runner interface{ Run() }\n\
         type H func()\n\
         type Command struct{ Run *H }\n\
         func worker() {}\n\
         func New() Command { return Command{} }\n\
         func invoke() { c := New(); c.Run() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "invoke", "Run"));

    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ConcreteReceiverNoSelector));
    assert_manifest_route(&cg, "concrete_no_selector_drop");
}

#[test]
fn manifest_pins_concrete_no_selector_with_resolver_telemetry() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type Marker interface{ M() }\n\
         type S struct{}\n\
         func run(s S) { s.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, Some(DropReason::ConcreteReceiverNoSelector));
    assert_eq!(outcome.telemetry.go_concrete_receiver_no_selector_drop, 1);
    assert_manifest_route(&cg, "concrete_no_selector_drop");
}

#[test]
fn manifest_pins_unproven_drop_with_resolver_telemetry() {
    let cg = build_go(&[
        (
            "one/q/types.go",
            "package q\ntype S struct{}\nfunc (S) M() {}\n",
        ),
        ("two/q/types.go", "package q\ntype Other struct{}\n"),
        (
            "iface/types.go",
            "package iface\ntype Other interface{ M() }\n",
        ),
        (
            "app/use.go",
            "package app\nimport q \"example/one/q\"\nfunc run(s q.S) { s.M() }\n",
        ),
    ]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        1
    );
    assert_eq!(outcome.telemetry.go_unproven_receiver_bare_fallback_hits, 0);
    assert_manifest_route(&cg, "unproven_drop");
}
