use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::cpg::CodePropertyGraph;
use prism::data_flow::DataFlowGraph;
use prism::languages::Language;
use prism::navigation::queries;
use prism::resolution::{DropReason, GoOwnerIdentity};
use std::collections::{BTreeMap, BTreeSet};

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    super::test_support::build_go_with_module(sources, "example.com/root")
}

fn parse_go(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
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

fn site<'a>(cg: &'a CallGraph, caller_file: &str, caller: &str, method: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| {
            site.caller.file == caller_file
                && site.caller.name == caller
                && site.callee_name == method
        })
        .unwrap_or_else(|| panic!("missing {caller_file}:{caller}->{method}"))
}

fn owner(package_dir: &str, package_clause: &str, name: &str) -> GoOwnerIdentity {
    GoOwnerIdentity {
        package_dir: package_dir.to_string(),
        package_clause: package_clause.to_string(),
        name: name.to_string(),
    }
}

fn resolved_files(cg: &CallGraph, call: &CallSite) -> BTreeSet<String> {
    cg.resolve_call_site_full(call)
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
}

fn manifest_has_site(cg: &CallGraph, call: &CallSite) -> bool {
    queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .any(|entry| {
            entry["file"] == call.caller.file
                && entry["line"] == call.line
                && entry["method"] == call.callee_name
        })
}

#[test]
fn receiver_owner_carrying_uses_package_var_defining_file_alias() {
    let cg = build_go(&[
        (
            "api/types.go",
            "package api\ntype I interface{ M(); ApiOnly() }\ntype Real struct{}\nfunc (Real) M() {}\nfunc (Real) ApiOnly() {}\nfunc retain() { var _ I = Real{} }\n",
        ),
        (
            "decoy/types.go",
            "package decoy\ntype I interface{ M(); DecoyOnly() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\nfunc (Wrong) DecoyOnly() {}\nfunc retain() { var _ I = Wrong{} }\n",
        ),
        (
            "app/vars.go",
            "package app\nimport ext \"example.com/root/api\"\nvar Shared ext.I\n",
        ),
        (
            "app/use.go",
            "package app\nimport ext \"example.com/root/decoy\"\nfunc run() { Shared.M() }\n",
        ),
    ]);
    let call = site(&cg, "app/use.go", "run", "M");
    assert_eq!(
        call.receiver_owner_identity.as_ref(),
        Some(&owner("api", "api", "I")),
        "{call:?}"
    );
    assert_eq!(
        resolved_files(&cg, call),
        BTreeSet::from(["api/types.go".to_string()]),
        "{call:?}"
    );
    assert!(manifest_has_site(&cg, call), "{call:?}");
}

#[test]
fn receiver_owner_carrying_same_text_different_owners_conflicts() {
    let cg = build_go(&[
        (
            "api_a/types.go",
            "package api_a\ntype I interface{ M() }\ntype A struct{}\nfunc (A) M() {}\n",
        ),
        (
            "api_b/types.go",
            "package api_b\ntype I interface{ M() }\ntype B struct{}\nfunc (B) M() {}\n",
        ),
        (
            "app/a.go",
            "package app\nimport ext \"example.com/root/api_a\"\nvar Shared ext.I\n",
        ),
        (
            "app/b.go",
            "package app\nimport ext \"example.com/root/api_b\"\nvar Shared ext.I\n",
        ),
        (
            "app/use.go",
            "package app\nimport ext \"example.com/root/api_a\"\nfunc run() { Shared.M() }\n",
        ),
    ]);
    let call = site(&cg, "app/use.go", "run", "M");
    assert_eq!(call.receiver_type, None, "{call:?}");
    assert_eq!(call.receiver_owner_identity, None, "{call:?}");
    assert!(call.receiver_materialized, "{call:?}");
    assert!(resolved_files(&cg, call).is_empty(), "{call:?}");
    assert!(!manifest_has_site(&cg, call), "{call:?}");
    assert_eq!(cg.go_owner_identity_partition.drops, 1);
}

#[test]
fn receiver_owner_carrying_selects_exact_package_clause() {
    let cg = build_go(&[
        (
            "api/types.go",
            "package api\ntype I interface{ M(); TestOnly() }\ntype TestImpl struct{}\nfunc (TestImpl) M() {}\nfunc (TestImpl) TestOnly() {}\n",
        ),
        (
            "app/vars.go",
            "package app\ntype I interface{ M(); ProdOnly() }\ntype ProdImpl struct{}\nfunc (ProdImpl) M() {}\nfunc (ProdImpl) ProdOnly() {}\nvar Shared I\n",
        ),
        (
            "app/use.go",
            "package app\nfunc runProd() { Shared.M() }\n",
        ),
        (
            "app/vars_external_test.go",
            "package app_test\nimport ext \"example.com/root/api\"\nvar Shared ext.I\n",
        ),
        (
            "app/use_external_test.go",
            "package app_test\nfunc runTest() { Shared.M() }\n",
        ),
    ]);
    let prod = site(&cg, "app/use.go", "runProd", "M");
    let test = site(&cg, "app/use_external_test.go", "runTest", "M");
    assert_eq!(
        prod.receiver_owner_identity.as_ref(),
        Some(&owner("app", "app", "I")),
        "{prod:?}"
    );
    assert_eq!(
        test.receiver_owner_identity.as_ref(),
        Some(&owner("api", "api", "I")),
        "{test:?}"
    );
    assert_eq!(
        resolved_files(&cg, prod),
        BTreeSet::from(["app/vars.go".to_string()])
    );
    assert_eq!(
        resolved_files(&cg, test),
        BTreeSet::from(["api/types.go".to_string()])
    );
}

#[test]
fn receiver_owner_carrying_profileless_fact_is_materialized_drop() {
    let cg = build_go(&[
        (
            "api/types.go",
            "package api\ntype I interface{ M() }\ntype Real struct{}\nfunc (Real) M() {}\n",
        ),
        (
            "decoy/types.go",
            "package decoy\ntype I interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\n",
        ),
        (
            "app/broken.go",
            "import ext \"example.com/root/api\"\nvar Shared ext.I\n",
        ),
        (
            "app/use.go",
            "package app\nimport ext \"example.com/root/decoy\"\nfunc run() { Shared.M() }\n",
        ),
    ]);
    let call = site(&cg, "app/use.go", "run", "M");
    assert_eq!(call.receiver_type, None, "{call:?}");
    assert_eq!(call.receiver_owner_identity, None, "{call:?}");
    assert!(call.receiver_materialized, "{call:?}");
    let outcome = cg.resolve_call_site_full(call);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
}

#[test]
fn receiver_owner_carrying_incremental_owner_change_replaces_old_owner() {
    let root = tempfile::tempdir().expect("temporary Go module");
    std::fs::write(
        root.path().join("go.mod"),
        "module example.com/root\n\ngo 1.22\n",
    )
    .unwrap();
    let initial = parse_go(&[
        (
            "api_a/types.go",
            "package api_a\ntype I interface{ M(); AOnly() }\ntype A struct{}\nfunc (A) M() {}\nfunc (A) AOnly() {}\n",
        ),
        (
            "api_b/types.go",
            "package api_b\ntype I interface{ M(); BOnly() }\ntype B struct{}\nfunc (B) M() {}\nfunc (B) BOnly() {}\n",
        ),
        (
            "decoy/types.go",
            "package decoy\ntype I interface{ M(); DecoyOnly() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\nfunc (Wrong) DecoyOnly() {}\n",
        ),
        (
            "app/vars.go",
            "package app\nimport ext \"example.com/root/api_a\"\nvar Shared ext.I\n",
        ),
        (
            "app/use.go",
            "package app\nimport ext \"example.com/root/decoy\"\nfunc run() { Shared.M() }\n",
        ),
    ]);
    let initial_inputs = prism::repo_loader::scope_graph_build_inputs(root.path(), &initial);
    let initial_graph = CallGraph::build_with_scope_graph_inputs(&initial, Some(&initial_inputs));
    assert_eq!(
        site(&initial_graph, "app/use.go", "run", "M")
            .receiver_owner_identity
            .as_ref(),
        Some(&owner("api_a", "api_a", "I"))
    );

    let mut edited = initial.clone();
    edited.insert(
        "app/vars.go".to_string(),
        ParsedFile::parse(
            "app/vars.go",
            "package app\nimport ext \"example.com/root/api_b\"\nvar Shared ext.I\n",
            Language::Go,
        )
        .expect("parse defining-file owner edit"),
    );
    let edited_inputs = prism::repo_loader::scope_graph_build_inputs(root.path(), &edited);
    let changed = BTreeSet::from(["app/vars.go".to_string()]);
    let incremental = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
        initial_graph,
        DataFlowGraph::build(&BTreeMap::new()),
        &changed,
        &edited,
        None,
        Some(&edited_inputs),
    )
    .call_graph;
    let full = CallGraph::build_with_scope_graph_inputs(&edited, Some(&edited_inputs));
    assert_eq!(incremental.calls, full.calls);
    assert_eq!(incremental.go_package_vars, full.go_package_vars);
    assert_eq!(
        incremental.go_owner_identity_partition,
        full.go_owner_identity_partition
    );
    assert_eq!(
        queries::call_stats(&incremental),
        queries::call_stats(&full)
    );

    let call = site(&incremental, "app/use.go", "run", "M");
    assert_eq!(
        call.receiver_owner_identity.as_ref(),
        Some(&owner("api_b", "api_b", "I")),
        "{call:?}"
    );
    assert_eq!(
        resolved_files(&incremental, call),
        BTreeSet::from(["api_b/types.go".to_string()])
    );
    assert_eq!(
        incremental
            .calls
            .values()
            .flatten()
            .filter(|candidate| {
                candidate.caller.file == "app/use.go"
                    && candidate.caller.name == "run"
                    && candidate.callee_name == "M"
            })
            .count(),
        1,
        "owner replacement must not duplicate the occurrence"
    );
}

fn ended_scope_shadow_fixture() -> CallGraph {
    build_go(&[(
        "app/types.go",
        "package app\n\
         type I interface{ M(); IOnly() }\n\
         type Real struct{}\n\
         func (Real) M() {}\n\
         func (Real) IOnly() {}\n\
         type Inner struct{}\n\
         func (Inner) M() {}\n\
         func retain() { var _ I = Real{} }\n\
         func afterBlock(x I) {\n\
           { x := byte(1); _ = x }\n\
           x.M()\n\
         }\n\
         func afterIf(x I) {\n\
           if x := byte(1); x > 0 { _ = x }\n\
           x.M()\n\
         }\n\
         func afterVar(x I) {\n\
           { var x byte; _ = x }\n\
           x.M()\n\
         }\n\
         func activeShadow(x I) {\n\
           { x := Inner{}; x.M() }\n\
         }\n",
    )])
}

fn assert_ended_scope_owner(caller: &str) {
    let cg = ended_scope_shadow_fixture();
    let call = site(&cg, "app/types.go", caller, "M");
    assert!(!call.receiver_local_type_shadowed, "{caller}: {call:?}");
    assert_eq!(
        call.receiver_owner_identity.as_ref(),
        Some(&owner("app", "app", "I")),
        "{caller}: {call:?}"
    );
    assert_eq!(
        resolved_files(&cg, call),
        BTreeSet::from(["app/types.go".to_string()]),
        "{caller}: {call:?}"
    );
    assert!(manifest_has_site(&cg, call), "{caller}: {call:?}");
}

#[test]
fn receiver_owner_carrying_ignores_ended_block_short_declaration() {
    assert_ended_scope_owner("afterBlock");
}

#[test]
fn receiver_owner_carrying_ignores_ended_if_initializer_declaration() {
    assert_ended_scope_owner("afterIf");
}

#[test]
fn receiver_owner_carrying_ignores_ended_block_var_declaration() {
    assert_ended_scope_owner("afterVar");
}

#[test]
fn receiver_owner_carrying_keeps_an_active_inner_shadow_unproven() {
    let cg = ended_scope_shadow_fixture();
    let call = site(&cg, "app/types.go", "activeShadow", "M");
    assert!(call.receiver_local_type_shadowed, "{call:?}");
    assert_eq!(call.receiver_owner_identity, None, "{call:?}");
}

#[test]
fn ended_scope_owner_keeps_independent_implementer_with_tagged_name_collision() {
    let cg = build_go(&[
        (
            "api/decoder.go",
            "package api\n\
             type Adder interface{ Add(name, value string) }\n\
             func parse(b Adder) {\n\
               { b := byte(1); _ = b }\n\
               b.Add(\"name\", \"value\")\n\
             }\n",
        ),
        (
            "schema/good.go",
            "package schema\n\
             type Good struct{}\n\
             func (Good) Add(name, value string) {}\n\
             func retainGood() { _ = Good{} }\n",
        ),
        (
            "labels/labels_slice.go",
            "//go:build slicelabels\n\
             package labels\n\
             type ScratchBuilder struct{}\n\
             func (*ScratchBuilder) Add(name, value string) {}\n\
             func retainSlice() { _ = &ScratchBuilder{} }\n",
        ),
        (
            "labels/labels_dedupe.go",
            "//go:build dedupelabels\n\
             package labels\n\
             type ScratchBuilder struct{}\n\
             func (*ScratchBuilder) Add(name, value string) {}\n\
             func retainDedupe() { _ = &ScratchBuilder{} }\n",
        ),
        (
            "labels/labels_default.go",
            "//go:build !slicelabels && !dedupelabels\n\
             package labels\n\
             type ScratchBuilder struct{}\n\
             func (*ScratchBuilder) Add(name, value string) {}\n\
             func retainDefault() { _ = &ScratchBuilder{} }\n",
        ),
    ]);
    let call = site(&cg, "api/decoder.go", "parse", "Add");

    assert!(!call.receiver_local_type_shadowed, "{call:?}");
    assert_eq!(
        call.receiver_owner_identity.as_ref(),
        Some(&owner("api", "api", "Adder")),
        "{call:?}"
    );
    assert_eq!(
        resolved_files(&cg, call),
        BTreeSet::from(["schema/good.go".to_string()]),
        "an unrelated build-tag collision must not erase the exact implementer: {call:?}"
    );
    assert!(manifest_has_site(&cg, call), "{call:?}");
}
