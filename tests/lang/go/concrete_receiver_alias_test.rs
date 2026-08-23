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

fn site<'a>(cg: &'a CallGraph, caller: &str, method: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .unwrap_or_else(|| panic!("missing {caller}->{method} call site"))
}

fn assert_only_file(cg: &CallGraph, caller: &str, method: &str, expected: &str) {
    let outcome = cg.resolve_call_site_full(site(cg, caller, method));
    let files: BTreeSet<_> = outcome
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.as_str())
        .collect();
    assert_eq!(files, BTreeSet::from([expected]), "{caller}: {outcome:?}");
    assert!(outcome.resolved.iter().all(|resolved| {
        resolved.confidence == ResolutionConfidence::Exact
            && resolved.kind != ResolutionKind::EmbeddedPromotion
    }));
}

fn manifest_routes(cg: &CallGraph) -> BTreeSet<String> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .map(|site| {
            site["dispatch_route"]
                .as_str()
                .expect("dispatch route")
                .to_string()
        })
        .collect()
}

fn manifest_target_files(cg: &CallGraph) -> BTreeSet<String> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .flat_map(|site| {
            site["implementer_identities"]
                .as_array()
                .expect("implementer identities")
        })
        .map(|identity| {
            identity["file"]
                .as_str()
                .expect("implementer file")
                .to_string()
        })
        .collect()
}

#[test]
fn defined_interface_and_alias_to_interface_keep_interface_dispatch() {
    let cg = build_go(&[
        (
            "p/types.go",
            "package p\n\
             type I interface { M(string) }\n\
             type D I\n\
             type A = I\n\
             type Good struct{}\n\
             func (Good) M(string) {}\n\
             func retain() { _ = Good{} }\n",
        ),
        (
            "decoy/types.go",
            "package decoy\n\
             type D interface { M(int) }\n\
             type A interface { M(int) }\n\
             type Wrong struct{}\n\
             func (Wrong) M(int) {}\n\
             func retain() { _ = Wrong{} }\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import p \"example/p\"\n\
             func defined(d p.D) { d.M(\"ok\") }\n\
             func alias(a p.A) { a.M(\"ok\") }\n",
        ),
    ]);

    assert_only_file(&cg, "defined", "M", "p/types.go");
    assert_only_file(&cg, "alias", "M", "p/types.go");
    for caller in ["defined", "alias"] {
        assert!(cg
            .resolve_call_site_full(site(&cg, caller, "M"))
            .resolved
            .iter()
            .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch));
    }
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["interface_dispatch".into()])
    );
    assert_eq!(
        manifest_target_files(&cg),
        BTreeSet::from(["p/types.go".into()])
    );
}

#[test]
fn literal_interface_alias_feeds_satisfaction_and_dispatch() {
    let cg = build_go(&[
        (
            "p/types.go",
            "package p\n\
             type A = interface { M() }\n\
             type Good struct{}\n\
             func (Good) M() {}\n\
             func retain() { _ = Good{} }\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import p \"example/p\"\n\
             func run(a p.A) { a.M() }\n",
        ),
    ]);

    assert_only_file(&cg, "run", "M", "p/types.go");
    assert!(cg.interface_impls.contains_key(&("A".into(), "M".into())));
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["interface_dispatch".into()])
    );
    assert_eq!(
        manifest_target_files(&cg),
        BTreeSet::from(["p/types.go".into()])
    );
}

#[test]
fn concrete_alias_uses_its_declaration_file_import_environment() {
    let cg = build_go(&[
        (
            "q/types.go",
            "package q\n\
             type S struct{}\n\
             func (S) M() {}\n",
        ),
        (
            "p/types.go",
            "package p\n\
             import dep \"example/q\"\n\
             type A = dep.S\n",
        ),
        (
            "p/run.go",
            "package p\n\
             func run(a A) { a.M() }\n",
        ),
        (
            "decoy/types.go",
            "package decoy\n\
             type A interface { M() }\n\
             type Wrong struct{}\n\
             func (Wrong) M() {}\n",
        ),
    ]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_only_file(&cg, "run", "M", "q/types.go");
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 1);
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["concrete_direct".into()])
    );
    assert!(manifest_target_files(&cg).is_empty());
}

#[test]
fn pointer_and_transitive_concrete_aliases_route_to_the_canonical_owner() {
    let cg = build_go(&[
        (
            "q/types.go",
            "package q\n\
             type S struct{}\n\
             func (*S) M() {}\n",
        ),
        (
            "p/types.go",
            "package p\n\
             import q \"example/q\"\n\
             type B = *q.S\n\
             type A = B\n",
        ),
        (
            "p/run.go",
            "package p\n\
             func run(a A) { a.M() }\n",
        ),
        (
            "decoy/types.go",
            "package decoy\n\
             type A interface { M() }\n\
             type Wrong struct{}\n\
             func (Wrong) M() {}\n",
        ),
    ]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_only_file(&cg, "run", "M", "q/types.go");
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 1);
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["concrete_direct".into()])
    );
    assert!(manifest_target_files(&cg).is_empty());
}

#[test]
fn pointer_to_interface_alias_keeps_interface_dispatch() {
    let cg = build_go(&[
        (
            "p/types.go",
            "package p\n\
             type I interface{ M() }\n\
             type P = *I\n\
             type Good struct{}\n\
             func (Good) M() {}\n\
             func retain() { _ = Good{} }\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import p \"example/p\"\n\
             func run(value p.P) { value.M() }\n",
        ),
    ]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_only_file(&cg, "run", "M", "p/types.go");
    assert!(outcome
        .resolved
        .iter()
        .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch));
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["interface_dispatch".into()])
    );
    assert_eq!(
        manifest_target_files(&cg),
        BTreeSet::from(["p/types.go".into()])
    );
}

#[test]
fn pointer_alias_with_unresolved_pointee_fails_closed_to_r3() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type Marker interface{ M() }\n\
         type P = *Missing\n\
         func run(value P) { value.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 0);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        1
    );
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["unproven_drop".into()])
    );
}

#[test]
fn defined_non_interface_uses_its_own_method_set() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type Token string\n\
         type Marker interface{ M() }\n\
         func (Token) M() {}\n\
         func run(t Token) { t.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_only_file(&cg, "run", "M", "main.go");
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 1);
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["concrete_direct".into()])
    );
    assert!(manifest_target_files(&cg).is_empty());
}

#[test]
fn alias_cycle_fails_closed_without_a_direct_edge() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type A = B\n\
         type B = A\n\
         type Marker interface{ M() }\n\
         func run(a A) { a.M() }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 0);
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["unproven_drop".into()])
    );
    assert!(manifest_target_files(&cg).is_empty());
}

#[test]
fn local_interface_shadows_package_concrete_alias_at_call() {
    let cg = build_go(&[
        (
            "q/types.go",
            "package q\ntype S struct{}\nfunc (S) M() {}\n",
        ),
        (
            "p/types.go",
            "package p\n\
             import q \"example/q\"\n\
             type A = q.S\n\
             type Marker interface{ M() }\n",
        ),
        (
            "p/run.go",
            "package p\n\
             func run() {\n\
               type A interface{ M() }\n\
               var a A\n\
               a.M()\n\
             }\n",
        ),
    ]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 0);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        1
    );
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["unproven_drop".into()])
    );
}

#[test]
fn local_concrete_shadows_package_interface_at_call() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
         type A interface{ M() }\n\
         type Good struct{}\n\
         func (Good) M() {}\n\
         func retain() { _ = Good{} }\n\
         func run() {\n\
           type A struct{}\n\
           var a A\n\
           a.M()\n\
         }\n",
    )]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_only_file(&cg, "run", "M", "main.go");
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 0);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        1
    );
    assert_eq!(outcome.telemetry.go_unproven_receiver_bare_fallback_hits, 1);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_edges,
        1
    );
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["interface_dispatch".into()])
    );
}

#[test]
fn local_type_in_different_function_does_not_shadow_package_alias() {
    let cg = build_go(&[
        (
            "q/types.go",
            "package q\ntype S struct{}\nfunc (S) M() {}\n",
        ),
        (
            "p/types.go",
            "package p\n\
             import q \"example/q\"\n\
             type A = q.S\n\
             type Marker interface{ M() }\n",
        ),
        (
            "p/run.go",
            "package p\n\
             func shadow() { type A interface{ M() }; var _ A }\n\
             func run(a A) { a.M() }\n",
        ),
    ]);
    let outcome = cg.resolve_call_site_full(site(&cg, "run", "M"));

    assert_only_file(&cg, "run", "M", "q/types.go");
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 1);
    assert_eq!(
        manifest_routes(&cg),
        BTreeSet::from(["concrete_direct".into()])
    );
}
