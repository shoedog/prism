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

    assert_eq!(
        resolved_exact_dispatch_identities(&cg, "invoke", "Act"),
        expected
    );
    assert_eq!(
        manifest_dispatch_identities(&cg, "lib/defs.go", "Act"),
        expected
    );
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

    assert_eq!(
        resolved_exact_dispatch_identities(&cg, "invoke", "Act"),
        expected
    );
    assert_eq!(
        manifest_dispatch_identities(&cg, "lib/defs.go", "Act"),
        expected
    );
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

    assert_eq!(
        resolved_exact_dispatch_identities(&cg, "invoke", "Use"),
        expected
    );
    assert_eq!(
        manifest_dispatch_identities(&cg, "lib/defs.go", "Use"),
        expected
    );
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

    assert_eq!(
        resolved_exact_dispatch_identities(&cg, "invoke", "Use"),
        expected
    );
    assert_eq!(
        manifest_dispatch_identities(&cg, "lib/defs.go", "Use"),
        expected
    );
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
        assert_eq!(
            resolved_exact_dispatch_identities(&cg, caller, method),
            expected
        );
        assert_eq!(
            manifest_dispatch_identities(&cg, "lib/defs.go", method),
            expected
        );
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

    assert_eq!(
        resolved_exact_dispatch_identities(&cg, "invoke", "Act"),
        expected
    );
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
    assert!(stats["go_alias_unresolved"]["cycle"].as_u64().unwrap_or(0) >= 1);
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

    assert_eq!(
        resolved_exact_dispatch_identities(&cg, "invoke", "Use"),
        expected
    );
    assert_eq!(
        manifest_dispatch_identities(&cg, "lib/defs.go", "Use"),
        expected
    );
}

#[test]
fn s4_block_local_alias_declarations_never_enter_the_package_index() {
    // SOL-W1: a block-local `type ID = int` inside an unrelated function body
    // must not become a package-level variant of the real Defined `ID`.
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype ID struct{}\ntype Doer interface { Act(ID) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, id ID) { h.Act(id) }\nfunc unrelated() { type ID = int; _ = ID(0) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(i ID) {}\n",
        ),
    ]);
    assert_eq!(
        resolved_method_owners(&cg, "invoke", "Act"),
        BTreeSet::from(["Impl".to_string()])
    );
    let stats = call_stats(&cg);
    assert_eq!(
        stats["go_alias_unresolved"]["defined_variant"]
            .as_u64()
            .unwrap_or(0),
        0
    );
}

#[test]
fn s4_cycle_guard_is_path_scoped_sibling_leaves_expand() {
    // SOL-W3 / fix-1: the first B's guard entry must be removed before the
    // second B expands — `func(B, B)` matches `func(int, int)`.
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype B = int\ntype F = func(B, B)\ntype M = map[B]B\ntype S = []B\ntype Doer interface { UseF(F); UseM(M); UseS(S); UseB(B) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.UseF(nil); h.UseM(nil); h.UseS(nil); h.UseB(0) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) UseF(f func(int, int)) {}\nfunc (Impl) UseM(m map[int]int) {}\nfunc (Impl) UseS(s []int) {}\nfunc (Impl) UseB(b int) {}\n",
        ),
    ]);

    for (caller, method) in [
        ("invoke", "UseF"),
        ("invoke", "UseM"),
        ("invoke", "UseS"),
        ("invoke", "UseB"),
    ] {
        let site = cg
            .calls
            .values()
            .flatten()
            .find(|s| s.caller.name == caller && s.callee_name == method)
            .expect("site");
        let owners: Vec<_> = cg
            .resolve_call_site_full(site)
            .resolved
            .iter()
            .filter_map(|r| cg.method_owners.get(r.target).cloned())
            .collect();
        assert_eq!(
            owners,
            vec!["Impl".to_string()],
            "{method} must keep its true Exact edge"
        );
    }
}

#[test]
fn s4_package_declaration_shadows_predeclared_byte_and_rune() {
    // SOL-W2 / fix-2: a visible package declaration named `byte` must be
    // resolved BEFORE predeclared normalization.
    let shadowing = build_go(&[
        ("base/b.go", "package base\ntype ID struct{}\n"),
        (
            "lib/defs.go",
            "package lib\nimport \"example.com/prism/base\"\ntype byte = base.ID\ntype Doer interface { Act(byte) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(base.ID{}) }\n",
        ),
        (
            "lib/impl_shadow.go",
            "package lib\nimport \"example.com/prism/base\"\ntype ImplShadow struct{}\nfunc (ImplShadow) Act(i base.ID) {}\n",
        ),
        (
            "lib/impl_uint8.go",
            "package lib\ntype ImplUint8 struct{}\nfunc (ImplUint8) Act(b uint8) {}\n",
        ),
    ]);
    // The alias expands to base.ID: matches the base.ID implementer, never uint8.
    let site = shadowing
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.name == "invoke" && s.callee_name == "Act")
        .expect("site");
    let owners: Vec<_> = shadowing
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|r| shadowing.method_owners.get(r.target).cloned())
        .collect();
    assert_eq!(owners, vec!["ImplShadow".to_string()]);

    // A DEFINED shadow (`type rune int32`) also disables normalization.
    let defined_shadow = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype rune int32\ntype Doer interface { Act(rune) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(rune(0)) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(r int32) {}\nfunc (Impl) ActR(real rune) {}\n",
        ),
    ]);
    let stats = call_stats(&defined_shadow);
    // `rune` here is a distinct defined type; it must NOT equal int32.
    let site = defined_shadow
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.name == "invoke" && s.callee_name == "Act")
        .expect("site");
    let owners: Vec<_> = defined_shadow
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|r| defined_shadow.method_owners.get(r.target).cloned())
        .collect();
    assert_eq!(owners, Vec::<String>::new());
    drop(stats);

    // Control: unshadowed byte ↔ uint8 stays Exact.
    let control = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype ByteAlias = byte\ntype Doer interface { Act(ByteAlias) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(uint8(0)) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(b uint8) {}\n",
        ),
    ]);
    let site = control
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.name == "invoke" && s.callee_name == "Act")
        .expect("site");
    let owners: Vec<_> = control
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|r| control.method_owners.get(r.target).cloned())
        .collect();
    assert_eq!(owners, vec!["Impl".to_string()]);
}

#[test]
fn s4_parameterized_alias_shape_recognition_is_not_error_text_dependent() {
    // SMELL-fix 4: a generic DEFINED type (`type G int`, no `=`) never
    // becomes an alias; a parameterized spec with an unextractable RHS
    // (`= =`) stays an ALIAS that fails closed instead of expanding.
    let defined_generic = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype G[T any] int\ntype Doer interface { Act(G[int]) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(G[int](0)) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(g G[int]) {}\n",
        ),
    ]);
    // Both sides spell G[int]: ordinary generic-defined comparison keeps Exact.
    assert_eq!(
        resolved_method_owners(&defined_generic, "invoke", "Act"),
        BTreeSet::from(["Impl".to_string()])
    );

    // Doubled `=` (ERROR node "= =", clean "=" token children): recognition
    // keys on the SHAPE (parameters + any `=`), not on ERROR text position,
    // and the extractable RHS still expands.
    let doubled_eq = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Pair[A any, B any] struct{ a A; b B }\ntype Twice[T any] = = Pair[T, T]\ntype Doer interface { Act(Twice[int]) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(Pair[int, int]{}) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(p Pair[int, int]) {}\n",
        ),
    ]);
    assert_eq!(
        resolved_method_owners(&doubled_eq, "invoke", "Act"),
        BTreeSet::from(["Impl".to_string()])
    );

    // A parameterized spec WITHOUT any `=` is a generic DEFINED type.
    let defined_generic_params = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Pair[A any, B any] struct{ a A; b B }\ntype Twice[T any] Pair[T, T]\ntype Doer interface { Act(Twice[int]) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(Pair[int, int]{}) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(p Pair[int, int]) {}\n",
        ),
    ]);
    assert!(resolved_method_owners(&defined_generic_params, "invoke", "Act").is_empty());
}

#[test]
fn s4ox_qualified_lookup_never_admits_the_external_test_clause_own_alias() {
    // sol-r2-1: `lib_test`'s own `type A = string` must not become a variant
    // of the IMPORTED lib.A; the valid int edge stays Exact.
    let cg = build_go(&[
        ("lib/prod.go", "package lib\ntype A = int\ntype B struct{}\nfunc (B) Act(a A) {}\n"),
        (
            "app/use.go",
            "package app\nimport l \"example.com/prism/lib\"\ntype Doer interface { Act(l.A) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(0) }\n",
        ),
        // The external-test package of lib re-declares A and imports lib.
        (
            "lib/ext_test.go",
            "package lib_test\nimport \"example.com/prism/lib\"\ntype A = string\ntype Probe struct{}\nfunc (Probe) Act(a A) {}\n",
        ),
    ]);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.name == "invoke" && s.callee_name == "Act")
        .expect("site");
    let owners: Vec<_> = cg
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|r| cg.method_owners.get(r.target).cloned())
        .collect();
    assert_eq!(owners, vec!["B".to_string()]);
}

#[test]
fn s4ox_predeclared_shadowing_requires_exactly_visible_declarations() {
    // terra-r2-1 / sol-r2-2: an INVISIBLE declaration of `byte`/`rune`
    // (mutually exclusive build tag, or a _test-only file) must NOT suppress
    // byte→uint8 / rune→int32 normalization for consumers that cannot see it.
    let tagged = build_go(&[
        (
            "lib/tagged_linux.go",
            "//go:build linux\npackage lib\ntype byte = int64\ntype rune = int64\n",
        ),
        (
            "lib/defs.go",
            "package lib\ntype Doer interface { Act(byte); Run(rune) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(uint8(0)); h.Run(int32(0)) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(b uint8) {}\nfunc (Impl) Run(r int32) {}\n",
        ),
    ]);
    for method in ["Act", "Run"] {
        let site = tagged
            .calls
            .values()
            .flatten()
            .find(|s| s.caller.name == "invoke" && s.callee_name == method)
            .expect("site");
        let owners: Vec<_> = tagged
            .resolve_call_site_full(site)
            .resolved
            .iter()
            .filter_map(|r| tagged.method_owners.get(r.target).cloned())
            .collect();
        assert_eq!(owners, vec!["Impl".to_string()], "{method}");
    }

    let test_only = build_go(&[
        ("lib/byte_test.go", "package lib\ntype byte = string\n"),
        (
            "lib/defs.go",
            "package lib\ntype Doer interface { Act(byte) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(uint8(0)) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Act(b uint8) {}\n",
        ),
    ]);
    let site = test_only
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.name == "invoke" && s.callee_name == "Act")
        .expect("site");
    let owners: Vec<_> = test_only
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|r| test_only.method_owners.get(r.target).cloned())
        .collect();
    assert_eq!(owners, vec!["Impl".to_string()]);
}

#[test]
fn s4ox_cycle_guard_keys_are_per_declaration_not_per_consumer() {
    // sol-r2-3: two packages may both declare local leaves named C; the guard
    // must key by DECLARATION identity, not consumer directory.
    let cg = build_go_with_module(
        &[
            (
                "p1/a.go",
                "package p1\nimport p2 \"example.com/prism/p2\"\ntype A = C\ntype C = p2.B\ntype Doer interface { Act(A) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(0) }\n",
            ),
            (
                "p2/b.go",
                "package p2\ntype B = C\ntype C = int\n",
            ),
            (
                "p2/impl.go",
                "package p2\ntype Impl struct{}\nfunc (Impl) Act(i int) {}\n",
            ),
        ],
        "example.com/prism",
    );
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.name == "invoke" && s.callee_name == "Act")
        .expect("site");
    let owners: Vec<_> = cg
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|r| cg.method_owners.get(r.target).cloned())
        .collect();
    assert_eq!(owners, vec!["Impl".to_string()]);
}

#[test]
fn s4ox_transitive_parameterized_aliases_instantiate() {
    // sol-r2-4: `Both = Twice[int]` expands through the canonical-string
    // walker and must instantiate Twice's parameter at that hop.
    let cg = build_go(&[
        (
            "lib/defs.go",
            "package lib\ntype Pair[A any, B any] struct{ a A; b B }\ntype Twice[T any] = Pair[T, T]\ntype Both = Twice[int]\ntype Doer interface { Use(Both); UseDirect(Twice[string]) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Use(Pair[int, int]{}); h.UseDirect(Pair[string, string]{}) }\n",
        ),
        (
            "lib/impl.go",
            "package lib\ntype Impl struct{}\nfunc (Impl) Use(p Pair[int, int]) {}\nfunc (Impl) UseDirect(p Pair[string, string]) {}\n",
        ),
    ]);
    for method in ["Use", "UseDirect"] {
        let site = cg
            .calls
            .values()
            .flatten()
            .find(|s| s.caller.name == "invoke" && s.callee_name == method)
            .expect("site");
        let owners: Vec<_> = cg
            .resolve_call_site_full(site)
            .resolved
            .iter()
            .filter_map(|r| cg.method_owners.get(r.target).cloned())
            .collect();
        assert_eq!(owners, vec!["Impl".to_string()], "{method}");
    }
}
