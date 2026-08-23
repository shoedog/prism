//! Slice 4 (roadmap #14 §5): alias-aware `Local↔Local` by path.
//!
//! Every alias expansion substitutes the ENTIRE canonical RHS type expression
//! before `Local`/`Qualified` tokens; expansion is allowed only when every
//! EXACTLY visible declaration variant is an `Alias` and all canonicalize
//! identically. Fail closed (`AliasUnresolved`) otherwise. Tests assert
//! resolver AND manifest identities, not just owner names.

use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
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

fn build_go_with_module(sources: &[(&str, &str)], module_path: &str) -> CallGraph {
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
        format!("module {module_path}\n\ngo 1.22\n"),
    )
    .expect("write go.mod fixture");
    let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn resolved_method_owners(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .map(|site| {
            cg.resolve_call_site_full(site)
                .resolved
                .iter()
                .filter_map(|resolved| cg.method_owners.get(resolved.target).cloned())
                .collect()
        })
        .unwrap_or_default()
}

type DispatchTargetIdentity = (String, String, String, String);

fn resolved_exact_dispatch_identities(
    cg: &CallGraph,
    caller: &str,
    method: &str,
) -> BTreeSet<DispatchTargetIdentity> {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .map(|site| {
            cg.resolve_call_site_full(site)
                .resolved
                .iter()
                .map(|resolved| {
                    assert_eq!(
                        resolved.confidence,
                        prism::resolution::ResolutionConfidence::Exact
                    );
                    assert_eq!(
                        resolved.kind,
                        prism::resolution::ResolutionKind::InterfaceDispatch
                    );
                    let target = resolved.target.clone();
                    (
                        target
                            .file
                            .rsplit_once('/')
                            .map(|(directory, _)| directory)
                            .unwrap_or_default()
                            .to_string(),
                        cg.go_file_profiles[&target.file].package_clause.clone(),
                        cg.method_owners[&target].clone(),
                        target.file.clone(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn manifest_dispatch_identities(
    cg: &CallGraph,
    caller_file: &str,
    method: &str,
) -> BTreeSet<DispatchTargetIdentity> {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|site| site["file"] == caller_file && site["method"] == method)
        .map(|site| {
            site["implementer_identities"]
                .as_array()
                .expect("manifest implementer identities")
                .iter()
                .map(|identity| {
                    (
                        identity["package_dir"].as_str().unwrap().to_string(),
                        identity["package_clause"].as_str().unwrap().to_string(),
                        identity["name"].as_str().unwrap().to_string(),
                        identity["file"].as_str().unwrap().to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn call_stats(cg: &CallGraph) -> serde_json::Value {
    prism::navigation::queries::call_stats(cg)
}

#[test]
fn s4_alias_to_same_package_local_type_expands_to_the_rhs() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype B struct{}\ntype A = B\ntype Doer interface { Act(A) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(B{}) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(B) {}\n",
        ),
    ]);
    let expected = BTreeSet::from([(
        "lib".to_string(),
        "lib".to_string(),
        "Impl".to_string(),
        "lib/impl.go".to_string(),
    )]);

    assert_eq!(resolved_exact_dispatch_identities(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_dispatch_identities(&cg, "lib/defs.go", "Act"), expected);
}

#[test]
fn s4_alias_to_qualified_type_expands_through_the_import_map() {
    let cg = build_go(&[
        ("base/b.go", "package base\ntype T struct{}\n"),
        (
            "lib/defs.go",
            "package lib\nimport \"example.com/prism/base\"\ntype A = base.T\ntype Doer interface { Act(A) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(base.T{}) }\n",
        ),
        (
            "app/impl.go",
            "package app\nimport \"example.com/prism/base\"\ntype Impl struct{}\nfunc (Impl) Act(base.T) {}\n",
        ),
    ]);
    let expected = BTreeSet::from([(
        "app".to_string(),
        "app".to_string(),
        "Impl".to_string(),
        "app/impl.go".to_string(),
    )]);

    assert_eq!(resolved_exact_dispatch_identities(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_dispatch_identities(&cg, "lib/defs.go", "Act"), expected);
}

#[test]
fn s4_alias_to_instantiated_generic_keeps_exact_against_direct_instantiation() {
    let cg = build_go(&[
        ("base/b.go", "package base\ntype List[T any] struct{ items []T }\n"),
        (
            "lib/defs.go",
            "package lib\nimport \"example.com/prism/base\"\ntype L = base.List[int]\ntype Doer interface { Use(L) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Use(base.List[int]{}) }\n",
        ),
        (
            "app/impl.go",
            "package app\nimport \"example.com/prism/base\"\ntype Impl struct{}\nfunc (Impl) Use(base.List[int]) {}\n",
        ),
    ]);
    let expected = BTreeSet::from([(
        "app".to_string(),
        "app".to_string(),
        "Impl".to_string(),
        "app/impl.go".to_string(),
    )]);

    assert_eq!(resolved_exact_dispatch_identities(&cg, "invoke", "Use"), expected);
    assert_eq!(manifest_dispatch_identities(&cg, "lib/defs.go", "Use"), expected);
}

#[test]
fn s4_parameterized_alias_expands_with_arity_checked_binding() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Pair[A any, B any] struct{ a A; b B }\ntype Twice[T any] = Pair[T, T]\ntype Doer interface { Use(Twice[int]) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Use(Pair[int, int]{}) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Use(p Pair[int, int]) {}\n",
        ),
    ]);
    let expected = BTreeSet::from([(
        "lib".to_string(),
        "lib".to_string(),
        "Impl".to_string(),
        "lib/impl.go".to_string(),
    )]);

    assert_eq!(resolved_exact_dispatch_identities(&cg, "invoke", "Use"), expected);
    assert_eq!(manifest_dispatch_identities(&cg, "lib/defs.go", "Use"), expected);
}

#[test]
fn s4_parameterized_alias_wrong_arity_fails_closed() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Pair[A any, B any] struct{ a A; b B }\ntype Twice[T any] = Pair[T, T]\ntype Doer interface { Bad(Twice[int, string]) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Bad(Pair[int, string]{}) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Bad(p Pair[int, string]) {}\n",
        ),
    ]);

    // Wrong arity -> AliasUnresolved -> the interface signature is gapped and
    // no Exact edge may be minted.
    assert!(resolved_exact_dispatch_identities(&cg, "invoke", "Bad").is_empty());
    assert!(manifest_dispatch_identities(&cg, "lib/defs.go", "Bad").is_empty());
    let stats = call_stats(&cg);
    assert!(stats["go_alias_unresolved"]["arity"].as_u64().unwrap_or(0) >= 1);
}

#[test]
fn s4_byte_and_rune_aliases_normalize_to_uint8_and_int32() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype ByteAlias = byte\ntype RuneAlias = rune\ntype Doer interface { UseB(ByteAlias); UseR(RuneAlias) }\ntype Holder struct { Doer }\nfunc invokeB(h Holder) { h.UseB(uint8(0)) }\nfunc invokeR(h Holder) { h.UseR(int32(0)) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) UseB(b uint8) {}\nfunc (Impl) UseR(r int32) {}\n",
        ),
    ]);

    for (caller, method) in [("invokeB", "UseB"), ("invokeR", "UseR")] {
        let expected = BTreeSet::from([(
            "lib".to_string(),
            "lib".to_string(),
            "Impl".to_string(),
            "lib/impl.go".to_string(),
        )]);
        assert_eq!(resolved_exact_dispatch_identities(&cg, caller, method), expected);
        assert_eq!(manifest_dispatch_identities(&cg, "lib/defs.go", method), expected);
    }
}

#[test]
fn s4_alias_to_composite_predeclared_type_expands() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Mapping = map[string]int\ntype Doer interface { Use(Mapping) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Use(map[string]int{}) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Use(m map[string]int) {}\n",
        ),
    ]);
    let expected = BTreeSet::from(["Impl".to_string()]);

    assert_eq!(resolved_method_owners(&cg, "invoke", "Use"), expected);
}

#[test]
fn s4_aliases_in_two_packages_to_one_base_type_keep_exact() {
    let cg = build_go(&[
        ("base/b.go", "package base\ntype T struct{}\n"),
        (
            "x/x.go",
            "package x\nimport \"example.com/prism/base\"\ntype XAlias = base.T\ntype Doer interface { Act(XAlias) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(base.T{}) }\n",
        ),
        (
            "y/y.go",
            "package y\nimport \"example.com/prism/base\"\ntype YAlias = base.T\ntype Impl struct{}\nfunc (Impl) Act(t base.T) {}\n",
        ),
    ]);
    let expected = BTreeSet::from([(
        "y".to_string(),
        "y".to_string(),
        "Impl".to_string(),
        "y/y.go".to_string(),
    )]);

    assert_eq!(resolved_exact_dispatch_identities(&cg, "invoke", "Act"), expected);
    assert_eq!(manifest_dispatch_identities(&cg, "x/x.go", "Act"), expected);
}

#[test]
fn s4_disagreeing_build_profile_variants_fail_closed() {
    let cg = build_go(&[
        (
            "lib/a_linux.go",
            "//go:build linux\npackage lib\ntype A = int\n",
        ),
        (
            "lib/a_windows.go",
            "//go:build windows\npackage lib\ntype A int\n",
        ),
        (
            "lib/defs.go",
            "package lib\ntype Doer interface { Act(A) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(0) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(a int) {}\n",
        ),
    ]);

    // One visible variant is an Alias, the other Defined -> AliasUnresolved.
    assert!(resolved_exact_dispatch_identities(&cg, "invoke", "Act").is_empty());
    assert!(manifest_dispatch_identities(&cg, "lib/defs.go", "Act").is_empty());
    let stats = call_stats(&cg);
    assert!(
        stats["go_alias_unresolved"]["defined_variant"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn s4_agreeing_build_profile_variants_expand() {
    let cg = build_go(&[
        (
            "lib/a_linux.go",
            "//go:build linux\npackage lib\ntype A = int\n",
        ),
        (
            "lib/a_windows.go",
            "//go:build windows\npackage lib\ntype A = int\n",
        ),
        (
            "lib/defs.go",
            "package lib\ntype Doer interface { Act(A) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(0) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(a int) {}\n",
        ),
    ]);

    assert_eq!(
        resolved_method_owners(&cg, "invoke", "Act"),
        BTreeSet::from(["Impl".to_string()])
    );
}

#[test]
fn s4_distinct_defined_types_with_the_same_name_in_two_proven_packages_no_longer_match() {
    // Renamed from the P10 fixture s4_unqualified_named_types_keep_the_existing_bare_name_rule:
    // with proven paths on both sides, Local↔Local now compares PATHS, so the
    // same-named defined types in two different packages must NOT match.
    let cg = build_go_with_module(
        &[
            (
                "lib/defs.go",
                "package lib\ntype ID struct{}\ntype Doer interface { Act(ID) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(ID{}) }\n",
            ),
            (
                "other/impl.go",
                "package other\ntype ID struct{}\ntype Impl struct{}\nfunc (Impl) Act(ID) {}\n",
            ),
        ],
        "example.com/root",
    );

    assert!(resolved_method_owners(&cg, "invoke", "Act").is_empty());
    assert!(manifest_dispatch_identities(&cg, "lib/defs.go", "Act").is_empty());
}

#[test]
fn s4_bare_bare_without_gomod_still_keeps_the_name_rule() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype ID struct{}\ntype Doer interface { Act(ID) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(ID{}) }\n",
        ),
        (
            "other/impl.go",
            "package other\ntype ID struct{}\ntype Impl struct{}\nfunc (Impl) Act(ID) {}\n",
        ),
    ]);

    assert_eq!(
        resolved_method_owners(&cg, "invoke", "Act"),
        BTreeSet::from(["Impl".to_string()])
    );
}

#[test]
fn s4_alias_cycle_fails_closed() {
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype A = B\ntype B = A\ntype Doer interface { Act(A) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(0) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(a int) {}\n",
        ),
    ]);

    assert!(resolved_exact_dispatch_identities(&cg, "invoke", "Act").is_empty());
    let stats = call_stats(&cg);
    assert!(
        stats["go_alias_unresolved"]["cycle"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn s4_test_clause_alias_is_invisible_to_production_consumers() {
    let cg = build_go(&[
        (
            "lib/prod.go",
            "package lib\ntype Doer interface { Act(A) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(0) }\n",
        ),
        // Declared in a _test file with the ordinary clause: production files
        // in the package must NOT see this alias.
        (
            "lib/ext_test.go",
            "package lib\ntype A = int\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(a int) {}\n",
        ),
    ]);

    assert!(resolved_exact_dispatch_identities(&cg, "invoke", "Act").is_empty());
}

#[test]
fn s4_generic_instantiation_wrapping_an_alias_keeps_shape() {
    let cg = build_go(&[
        ("base/b.go", "package base\ntype List[T any] struct{ items []T }\n"),
        (
            "lib/defs.go",
            "package lib\nimport \"example.com/prism/base\"\ntype Elem = int\ntype Doer interface { Use(base.List[Elem]) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Use(base.List[int]{}) }\n",
        ),
        (
            "app/impl.go",
            "package app\nimport \"example.com/prism/base\"\ntype Impl struct{}\nfunc (Impl) Use(l base.List[int]) {}\n",
        ),
    ]);
    let expected = BTreeSet::from([(
        "app".to_string(),
        "app".to_string(),
        "Impl".to_string(),
        "app/impl.go".to_string(),
    )]);

    assert_eq!(resolved_exact_dispatch_identities(&cg, "invoke", "Use"), expected);
    assert_eq!(manifest_dispatch_identities(&cg, "lib/defs.go", "Use"), expected);
}
