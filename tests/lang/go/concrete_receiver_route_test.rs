use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ReceiverRecovery, ResolutionConfidence, ResolutionKind};
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

fn assert_direct(
    cg: &CallGraph,
    caller: &str,
    recovery: ReceiverRecovery,
    expected_kind: ResolutionKind,
) {
    let site = site(cg, caller, "M");
    assert_eq!(site.receiver_recovery, Some(recovery));
    let outcome = cg.resolve_call_site_full(site);
    assert_eq!(outcome.drop, None, "{caller}: {outcome:?}");
    assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 1);
    assert_eq!(outcome.resolved.len(), 1, "{caller}: {outcome:?}");
    let resolved = &outcome.resolved[0];
    assert_eq!(resolved.target.file, "q/types.go", "{caller}: {outcome:?}");
    assert_eq!(resolved.confidence, ResolutionConfidence::Exact);
    assert_eq!(resolved.kind, expected_kind);
}

fn collision_fixture() -> CallGraph {
    build_go(&[
        (
            "p/interfaces.go",
            "package p\n\
             type A interface { M() }\n\
             type C interface { M() }\n\
             type Wrong struct{}\n\
             func (Wrong) M() {}\n\
             func retain() { _ = Wrong{} }\n",
        ),
        (
            "q/types.go",
            "package q\n\
             type A struct{}\n\
             func (A) M() {}\n\
             func NewA() A { return A{} }\n\
             type P struct{}\n\
             func (*P) M() {}\n\
             type C struct{}\n\
             func (*C) M() {}\n\
             func NewC() *C { return &C{} }\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import q \"example/q\"\n\
             func composite() { a := q.A{}; a.M() }\n\
             func factory() { a := q.NewA(); a.M() }\n\
             func pointer() { p := q.P{}; p.M() }\n\
             func typed(c *q.C) { c.M() }\n\
             func returned() { c := q.NewC(); c.M() }\n\
             func variable() { var c q.C; c.M() }\n\
             func asserted(x any) { x.(q.C).M() }\n",
        ),
    ])
}

#[test]
fn concrete_constructor_and_factory_receivers_select_only_the_declaring_file() {
    let cg = collision_fixture();

    assert_direct(
        &cg,
        "composite",
        ReceiverRecovery::ConstructorLocal,
        ResolutionKind::ConstructorLocal,
    );
    assert_direct(
        &cg,
        "factory",
        ReceiverRecovery::ReturnTyped,
        ResolutionKind::ReturnTyped,
    );
    assert_direct(
        &cg,
        "pointer",
        ReceiverRecovery::ConstructorLocal,
        ResolutionKind::ConstructorLocal,
    );
}

#[test]
fn concrete_typed_param_and_s1_return_typed_receivers_are_direct() {
    let cg = collision_fixture();

    assert_direct(
        &cg,
        "typed",
        ReceiverRecovery::TypedParam,
        ResolutionKind::TypedParam,
    );
    assert_direct(
        &cg,
        "returned",
        ReceiverRecovery::ReturnTyped,
        ResolutionKind::ReturnTyped,
    );
}

#[test]
fn concrete_var_and_type_assertion_receivers_are_direct() {
    let cg = collision_fixture();

    assert_direct(
        &cg,
        "variable",
        ReceiverRecovery::VarDecl,
        ResolutionKind::TypedParam,
    );
    assert_direct(
        &cg,
        "asserted",
        ReceiverRecovery::TypeAssertion,
        ResolutionKind::TypedParam,
    );
}

#[test]
fn syntactic_return_without_a_proven_owner_keeps_the_pre_drop() {
    let cg = collision_fixture();
    let mut syntactic_only = site(&cg, "returned", "M").clone();
    assert_eq!(
        syntactic_only.receiver_recovery,
        Some(ReceiverRecovery::ReturnTyped)
    );
    syntactic_only.receiver_owner_identity = None;

    let outcome = cg.resolve_call_site_full(&syntactic_only);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(
        outcome.drop,
        Some(prism::resolution::DropReason::ExternalReceiver)
    );
}

#[test]
fn direct_receiver_fixture_contains_a_discriminating_wrong_interface_target() {
    let cg = collision_fixture();
    let wrong_targets: BTreeSet<_> = cg
        .methods
        .get(&("Wrong".to_string(), "M".to_string()))
        .expect("wrong interface implementer method")
        .iter()
        .map(|target| target.file.as_str())
        .collect();

    assert_eq!(wrong_targets, BTreeSet::from(["p/interfaces.go"]));
}
