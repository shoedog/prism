use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
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
    let repo = tempfile::tempdir().expect("temporary Go module root");
    std::fs::write(
        repo.path().join("go.mod"),
        "module example.com/root\n\ngo 1.24\n",
    )
    .expect("write go.mod fixture");
    let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn resolver_target_files(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .expect("interface dispatch site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .map(|resolved| {
            assert_eq!(resolved.confidence, ResolutionConfidence::Exact);
            assert_eq!(resolved.kind, ResolutionKind::InterfaceDispatch);
            resolved.target.file.clone()
        })
        .collect()
}

fn manifest_target_files(cg: &CallGraph, caller_file: &str, method: &str) -> BTreeSet<String> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|site| site["file"] == caller_file && site["method"] == method)
        .expect("manifest interface dispatch site")["implementer_identities"]
        .as_array()
        .expect("manifest implementer identities")
        .iter()
        .map(|identity| identity["file"].as_str().expect("target file").to_string())
        .collect()
}

fn assert_target_files(
    cg: &CallGraph,
    caller_file: &str,
    caller: &str,
    method: &str,
    expected: &[&str],
) {
    let expected: BTreeSet<String> = expected.iter().map(|path| (*path).to_string()).collect();
    assert_eq!(resolver_target_files(cg, caller, method), expected);
    assert_eq!(manifest_target_files(cg, caller_file, method), expected);
}

fn assert_unresolved_reason(cg: &CallGraph, reason: &str) {
    let stats = prism::navigation::queries::call_stats(cg);
    assert!(
        stats["go_alias_unresolved"][reason]
            .as_u64()
            .is_some_and(|count| count > 0),
        "missing go_alias_unresolved/{reason}: {stats}"
    );
    assert!(
        stats["interface_gaps"]["AliasUnresolved"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "missing interface_gaps/AliasUnresolved: {stats}"
    );
}

#[test]
fn parameterized_alias_binds_each_occurrence_capture_safely() {
    let cg = build_go(&[
        (
            "api/api.go",
            "package api\ntype Pair[A, B any] struct{ First A; Second B }\ntype Twice[T any] = Pair[T, T]\ntype Doer interface{ Use(Twice[int]) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v Twice[int]){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\nimport api \"example.com/root/api\"\ntype Impl struct{}\nfunc (Impl) Use(api.Pair[int, int]){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["worker/impl.go"]);
}

#[test]
fn parameterized_alias_wrong_arity_fails_closed() {
    let cg = build_go(&[
        (
            "api/api.go",
            "package api\ntype Pair[A, B any] struct{ First A; Second B }\ntype Twice[T any] = Pair[T, T]\ntype Doer interface{ Use(Twice[int, string]) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder){ h.Use(Twice[int, string]{}) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\nimport api \"example.com/root/api\"\ntype Impl struct{}\nfunc (Impl) Use(api.Pair[int, int]){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &[]);
    assert_unresolved_reason(&cg, "arity");
}

#[test]
fn unsupported_parameterized_alias_constraint_is_unresolvable() {
    let cg = build_go(&[
        (
            "api/api.go",
            "package api\ntype Pair[A, B any] struct{ First A; Second B }\ntype Constrained[T ~int] = Pair[T, T]\ntype Doer interface{ Use(Constrained[int]) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v Constrained[int]){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\nimport api \"example.com/root/api\"\ntype Impl struct{}\nfunc (Impl) Use(api.Pair[int, int]){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &[]);
    assert_unresolved_reason(&cg, "unresolvable");
}

#[test]
fn nested_aliases_expand_transitively() {
    let cg = build_go(&[
        (
            "base/base.go",
            "package base\ntype ID struct{}\ntype A = ID\ntype B = A\ntype Doer interface{ Use(B) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v B){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\nimport base \"example.com/root/base\"\ntype Impl struct{}\nfunc (Impl) Use(base.ID){}\n",
        ),
    ]);
    assert_target_files(&cg, "base/base.go", "invoke", "Use", &["worker/impl.go"]);
}

#[test]
fn generic_instantiation_wrapping_an_alias_keeps_shape() {
    let cg = build_go(&[
        ("base/base.go", "package base\ntype ID struct{}\n"),
        (
            "api/api.go",
            "package api\nimport base \"example.com/root/base\"\ntype Box[T any] struct{ Value T }\ntype A = base.ID\ntype Doer interface{ Use(Box[A]) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v Box[A]){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\nimport api \"example.com/root/api\"\nimport base \"example.com/root/base\"\ntype Impl struct{}\nfunc (Impl) Use(api.Box[base.ID]){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["worker/impl.go"]);
}

#[test]
fn agreeing_build_profile_alias_variants_expand() {
    let cg = build_go(&[
        ("base/base.go", "package base\ntype ID struct{}\n"),
        (
            "api/alias_linux.go",
            "package api\nimport base \"example.com/root/base\"\ntype A = base.ID\n",
        ),
        (
            "api/alias_windows.go",
            "package api\nimport base \"example.com/root/base\"\ntype A = base.ID\n",
        ),
        (
            "api/api.go",
            "package api\ntype Doer interface{ Use(A) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v A){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\nimport base \"example.com/root/base\"\ntype Impl struct{}\nfunc (Impl) Use(base.ID){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &["worker/impl.go"]);
    assert!(
        prism::navigation::queries::call_stats(&cg)["go_alias_expanded"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
}

#[test]
fn alias_and_defined_build_variants_fail_closed() {
    let cg = build_go(&[
        (
            "api/alias_linux.go",
            "package api\ntype A = int\n",
        ),
        (
            "api/alias_windows.go",
            "package api\ntype A int\n",
        ),
        (
            "api/api.go",
            "package api\ntype Doer interface{ Use(A) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v A){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\ntype Impl struct{}\nfunc (Impl) Use(int){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &[]);
    assert_unresolved_reason(&cg, "defined_variant");
}

#[test]
fn uncertain_alias_profile_fails_closed() {
    let cg = build_go(&[
        (
            "api/alias.go",
            "//go:build (\n\npackage api\ntype A = int\n",
        ),
        (
            "api/api.go",
            "package api\ntype Doer interface{ Use(A) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v A){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\ntype Impl struct{}\nfunc (Impl) Use(int){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &[]);
    assert_unresolved_reason(&cg, "profile_uncertain");
}

#[test]
fn alias_cycle_fails_closed() {
    let cg = build_go(&[
        (
            "api/api.go",
            "package api\ntype A = B\ntype B = A\ntype Doer interface{ Use(A) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v A){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\ntype Impl struct{}\nfunc (Impl) Use(int){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &[]);
    assert_unresolved_reason(&cg, "cycle");
}

#[test]
fn unresolvable_alias_target_fails_closed() {
    let cg = build_go(&[
        (
            "api/api.go",
            "package api\ntype A = missing.ID\ntype Doer interface{ Use(A) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v A){ h.Use(v) }\n",
        ),
        (
            "worker/impl.go",
            "package worker\ntype Impl struct{}\nfunc (Impl) Use(int){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &[]);
    assert_unresolved_reason(&cg, "unresolvable");
}

#[test]
fn external_test_clause_alias_is_invisible_to_production() {
    let cg = build_go(&[
        ("base/base.go", "package base\ntype ID struct{}\n"),
        (
            "api/api.go",
            "package api\ntype A struct{}\ntype Doer interface{ Use(A) }\ntype Holder struct{ Doer }\nfunc invoke(h Holder, v A){ h.Use(v) }\n",
        ),
        (
            "api/alias_test.go",
            "package api_test\nimport base \"example.com/root/base\"\ntype A = base.ID\n",
        ),
        (
            "worker/impl.go",
            "package worker\nimport base \"example.com/root/base\"\ntype Impl struct{}\nfunc (Impl) Use(base.ID){}\n",
        ),
    ]);
    assert_target_files(&cg, "api/api.go", "invoke", "Use", &[]);
}
