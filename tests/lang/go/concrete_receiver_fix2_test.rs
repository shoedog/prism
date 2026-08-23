use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{DropReason, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

fn parsed_files(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect()
}

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    CallGraph::build(&parsed_files(sources))
}

fn site<'a>(cg: &'a CallGraph, caller: &str, method: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .unwrap_or_else(|| panic!("missing {caller}->{method}"))
}

fn resolved_files(cg: &CallGraph, call: &CallSite) -> BTreeSet<String> {
    cg.resolve_call_site_full(call)
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
}

fn manifest_route(cg: &CallGraph, caller_file: &str, line: usize) -> String {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|entry| entry["file"] == caller_file && entry["line"] == line)
        .unwrap_or_else(|| panic!("missing manifest site {caller_file}:{line}"))["dispatch_route"]
        .as_str()
        .expect("dispatch route")
        .to_string()
}

fn rebinding_fixture() -> CallGraph {
    build_go(&[
        (
            "p/types.go",
            "package p\n\
             type I interface{ M() }\n\
             type Good struct{}\n\
             func (Good) M() {}\n\
             func retain() { _ = Good{} }\n",
        ),
        (
            "q/types.go",
            "package q\n\
             type Outer struct{}\n\
             func (Outer) M() {}\n\
             type Narrow struct{}\n\
             func (Narrow) M() {}\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import (p \"example/p\"; q \"example/q\")\n\
             func typeSwitchConcrete(x q.Outer) {\n\
               switch x := any(x).(type) {\n\
               case q.Narrow:\n\
                 x.M()\n\
               }\n\
             }\n\
             func typeSwitchInterface(x p.I) {\n\
               switch x := x.(type) {\n\
               case q.Narrow:\n\
                 x.M()\n\
               }\n\
             }\n\
             func shortRebind(x q.Outer) {\n\
               {\n\
                 x := q.Narrow{}\n\
                 x.M()\n\
               }\n\
             }\n\
             func rangeRebind(x q.Outer) {\n\
               for _, x := range []q.Narrow{{}} {\n\
                 x.M()\n\
               }\n\
             }\n\
             func singleConcrete(x q.Outer) { x.M() }\n\
             func singleInterface(x p.I) { x.M() }\n",
        ),
    ])
}

#[test]
fn value_rebindings_fail_closed_to_r3_without_outer_concrete_direct_edges() {
    let cg = rebinding_fixture();
    for (caller, line) in [
        ("typeSwitchConcrete", 6),
        ("shortRebind", 18),
        ("rangeRebind", 23),
    ] {
        let call = site(&cg, caller, "M");
        assert!(call.receiver_local_type_shadowed, "{caller}: {call:?}");
        let outcome = cg.resolve_call_site_full(call);
        assert!(outcome.resolved.is_empty(), "{caller}: {outcome:?}");
        assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
        assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 0);
        assert_eq!(manifest_route(&cg, "app/use.go", line), "unproven_drop");
    }
}

#[test]
fn type_switch_interface_rebinding_uses_the_legacy_r3_fallback() {
    let cg = rebinding_fixture();
    let call = site(&cg, "typeSwitchInterface", "M");
    assert!(call.receiver_local_type_shadowed, "{call:?}");
    let outcome = cg.resolve_call_site_full(call);

    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_sites,
        1
    );
    assert_eq!(outcome.telemetry.go_unproven_receiver_bare_fallback_hits, 1);
    assert_eq!(
        outcome.telemetry.go_unproven_receiver_bare_fallback_edges,
        2
    );
    assert_eq!(
        resolved_files(&cg, call),
        BTreeSet::from(["p/types.go".into(), "q/types.go".into()])
    );
    assert_eq!(manifest_route(&cg, "app/use.go", 12), "interface_dispatch");
}

#[test]
fn single_receiver_bindings_keep_r1_and_r2_routes() {
    let cg = rebinding_fixture();
    let concrete = site(&cg, "singleConcrete", "M");
    let concrete_outcome = cg.resolve_call_site_full(concrete);
    assert_eq!(
        resolved_files(&cg, concrete),
        BTreeSet::from(["q/types.go".into()])
    );
    assert_eq!(concrete_outcome.telemetry.go_concrete_receiver_direct, 1);
    assert_eq!(manifest_route(&cg, "app/use.go", 26), "concrete_direct");

    let interface = site(&cg, "singleInterface", "M");
    let interface_outcome = cg.resolve_call_site_full(interface);
    assert_eq!(
        resolved_files(&cg, interface),
        BTreeSet::from(["p/types.go".into(), "q/types.go".into()])
    );
    assert!(interface_outcome
        .resolved
        .iter()
        .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch));
    assert_eq!(manifest_route(&cg, "app/use.go", 27), "interface_dispatch");
}
