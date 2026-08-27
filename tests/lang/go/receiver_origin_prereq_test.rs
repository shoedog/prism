use prism::call_graph::{CallGraph, CallSite};
use prism::navigation::types::SymbolRef;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use prism::resolution::DropReason;
use std::collections::BTreeSet;
use std::sync::Arc;

struct GoFixture {
    _root: tempfile::TempDir,
    session: NavigationSession,
}

impl GoFixture {
    fn call_graph(&self) -> &CallGraph {
        self.session.index.call_graph()
    }
}

fn write(root: &std::path::Path, path: &str, contents: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture parent")).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn build_go(sources: &[(&str, &str)]) -> GoFixture {
    let root = tempfile::tempdir().expect("temporary Go module");
    write(
        root.path(),
        "go.mod",
        "module example.com/root\n\ngo 1.22\n",
    );
    for (path, contents) in sources {
        write(root.path(), path, contents);
    }
    let repo = Arc::new(load_repo(root.path()).expect("load Go fixture through repo_loader"));
    let index = Arc::new(NavigationIndex::build(&repo));
    GoFixture {
        _root: root,
        session: NavigationSession { repo, index },
    }
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

fn resolved_files(cg: &CallGraph, call: &CallSite) -> BTreeSet<String> {
    cg.resolve_call_site_full(call)
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
}

fn nav_files(
    fixture: &GoFixture,
    caller_file: &str,
    caller: &str,
    callee: &str,
) -> BTreeSet<String> {
    queries::callees(&fixture.session, Some(caller), Some(caller_file), None, 1)
        .expect("callees query")
        .items
        .iter()
        .filter_map(|item| match item.symbol.as_ref() {
            Some(SymbolRef::Function { file, name, .. }) if name == callee => Some(file.clone()),
            _ => None,
        })
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

fn assert_terminal_prerequisite_drop(
    fixture: &GoFixture,
    caller_file: &str,
    caller: &str,
    method: &str,
    wrong_file: &str,
    expected_reason: Option<&str>,
) {
    let cg = fixture.call_graph();
    let call = site(cg, caller_file, caller, method);
    let outcome = cg.resolve_call_site_full(call);
    let files: BTreeSet<_> = outcome
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect();
    assert!(
        !files.contains(wrong_file),
        "receiver prerequisite admitted exact wrong target {wrong_file}: {call:?} {outcome:?}"
    );
    assert!(files.is_empty(), "terminal prerequisite drop: {outcome:?}");
    assert!(!manifest_has_site(cg, call), "{call:?}");
    assert_eq!(call.receiver_type, None, "{call:?}");
    assert!(call.receiver_materialized, "{call:?}");
    assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    if let Some(reason) = expected_reason {
        let stats = queries::call_stats(cg);
        assert_eq!(
            stats["go_receiver_prereq_drops"][reason], 1,
            "{reason}: {} {call:?}",
            stats["go_receiver_prereq_drops"]
        );
    }
    assert!(
        !nav_files(fixture, caller_file, caller, method).contains(wrong_file),
        "sidecar/navigation retained exact wrong target {wrong_file}"
    );
}

fn assert_exact_retained(
    fixture: &GoFixture,
    caller_file: &str,
    caller: &str,
    method: &str,
    target_file: &str,
) {
    let cg = fixture.call_graph();
    let call = site(cg, caller_file, caller, method);
    assert_eq!(
        resolved_files(cg, call),
        BTreeSet::from([target_file.to_string()]),
        "{call:?}"
    );
    assert!(
        nav_files(fixture, caller_file, caller, method).contains(target_file),
        "navigation omitted {target_file}"
    );
}

#[test]
fn receiver_origin_prereq_predeclared_error_does_not_bind_unrelated_source_type() {
    let fixture = build_go(&[
        (
            "q/error.go",
            "package q\ntype error interface{ Error() string }\ntype Wrong struct{}\nfunc (Wrong) Error() string { return \"wrong\" }\nfunc retain() { _ = Wrong{} }\n",
        ),
        (
            "app/use.go",
            "package app\nfunc run(e error) string { return e.Error() }\n",
        ),
    ]);
    assert_terminal_prerequisite_drop(
        &fixture,
        "app/use.go",
        "run",
        "Error",
        "q/error.go",
        Some("declaration_unproven"),
    );
}

#[test]
fn receiver_origin_prereq_external_import_cannot_fall_back_to_directory_basename() {
    let fixture = build_go(&[
        (
            "api/types.go",
            "package api\ntype I interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\nfunc retain() { _ = Wrong{} }\n",
        ),
        (
            "app/use.go",
            "package app\nimport api \"outside.example/api\"\nfunc run(v api.I) { v.M() }\n",
        ),
    ]);
    assert_terminal_prerequisite_drop(
        &fixture,
        "app/use.go",
        "run",
        "M",
        "api/types.go",
        Some("strict_import_unresolved"),
    );
}

#[test]
fn receiver_origin_prereq_external_factory_callee_is_exact_before_return_lookup() {
    let fixture = build_go(&[
        (
            "api/factory.go",
            "package api\ntype I interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\nfunc New() I { return Wrong{} }\n",
        ),
        (
            "app/use.go",
            "package app\nimport api \"outside.example/api\"\nfunc run() { r := api.New(); r.M() }\n",
        ),
    ]);
    assert_terminal_prerequisite_drop(&fixture, "app/use.go", "run", "M", "api/factory.go", None);
}

#[test]
fn receiver_origin_prereq_dot_import_bare_name_requires_local_declaration() {
    let fixture = build_go(&[
        (
            "q/types.go",
            "package q\ntype I interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\nfunc retain() { _ = Wrong{} }\n",
        ),
        (
            "app/use.go",
            "package app\nimport . \"outside.example/api\"\nfunc run(v I) { v.M() }\n",
        ),
    ]);
    assert_terminal_prerequisite_drop(
        &fixture,
        "app/use.go",
        "run",
        "M",
        "q/types.go",
        Some("dot_import_bare_unproven"),
    );
}

#[test]
fn receiver_origin_prereq_function_type_parameter_is_not_a_package_type() {
    let fixture = build_go(&[(
        "app/use.go",
        "package app\ntype T struct{}\nfunc (T) M() {}\nfunc run[T interface{ M() }](v T) { v.M() }\n",
    )]);
    assert_terminal_prerequisite_drop(
        &fixture,
        "app/use.go",
        "run",
        "M",
        "app/use.go",
        Some("type_parameter"),
    );
}

#[test]
fn receiver_origin_prereq_method_receiver_type_parameter_is_not_a_package_type() {
    let fixture = build_go(&[(
        "app/use.go",
        "package app\ntype T struct{}\nfunc (T) M() {}\ntype Store[U interface{ M() }] struct{}\nfunc (s Store[T]) run(v T) { v.M() }\n",
    )]);
    assert_terminal_prerequisite_drop(
        &fixture,
        "app/use.go",
        "run",
        "M",
        "app/use.go",
        Some("type_parameter"),
    );
}

#[test]
fn receiver_origin_prereq_local_type_declaration_is_terminal() {
    let fixture = build_go(&[
        (
            "q/types.go",
            "package q\ntype Iterator interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\nfunc retain() { _ = Wrong{} }\n",
        ),
        (
            "app/use.go",
            "package app\ntype Iterator interface{ M() }\ntype Good struct{}\nfunc (Good) M() {}\nfunc retain() { _ = Good{} }\nfunc run(v Iterator) { type Iterator interface{ M() }; v.M() }\n",
        ),
    ]);
    assert_terminal_prerequisite_drop(
        &fixture,
        "app/use.go",
        "run",
        "M",
        "q/types.go",
        Some("local_type_declaration"),
    );
}

#[test]
fn receiver_origin_prereq_dot_import_preserves_declared_local_receiver() {
    let fixture = build_go(&[(
        "app/use.go",
        "package app\nimport . \"outside.example/api\"\ntype Local struct{}\nfunc (Local) M() {}\nfunc run(v Local) { v.M() }\n",
    )]);
    assert_exact_retained(&fixture, "app/use.go", "run", "M", "app/use.go");
}

#[test]
fn receiver_origin_prereq_non_generic_bare_type_preserves_declared_local_receiver() {
    let fixture = build_go(&[(
        "app/use.go",
        "package app\ntype T struct{}\nfunc (T) M() {}\nfunc run(v T) { v.M() }\n",
    )]);
    assert_exact_retained(&fixture, "app/use.go", "run", "M", "app/use.go");
}

#[test]
fn receiver_origin_prereq_value_rebinding_preserves_same_scope_direct_reuse() {
    let fixture = build_go(&[(
        "app/use.go",
        "package app\ntype C struct{}\nfunc (C) M() {}\nfunc Reset(v C) (C, error) { return v, nil }\nfunc run(v C) { v, err := Reset(v); _ = err; v.M() }\n",
    )]);
    assert_exact_retained(&fixture, "app/use.go", "run", "M", "app/use.go");
}

#[test]
fn receiver_origin_prereq_qualified_exact_module_identity_is_retained() {
    let fixture = build_go(&[
        (
            "api/types.go",
            "package api\ntype A struct{}\nfunc (A) M() {}\n",
        ),
        (
            "app/use.go",
            "package app\nimport api \"example.com/root/api\"\nfunc run(v api.A) { v.M() }\n",
        ),
    ]);
    assert_exact_retained(&fixture, "app/use.go", "run", "M", "api/types.go");
}

#[test]
fn receiver_origin_prereq_missing_declaration_profile_is_not_proof() {
    let fixture = build_go(&[
        (
            "api/types.go",
            "type I interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\n",
        ),
        (
            "app/use.go",
            "package app\nimport api \"example.com/root/api\"\nfunc run(v api.I) { v.M() }\n",
        ),
    ]);
    assert_terminal_prerequisite_drop(
        &fixture,
        "app/use.go",
        "run",
        "M",
        "api/types.go",
        Some("strict_import_unresolved"),
    );
}

#[test]
fn receiver_origin_prereq_slice1_cross_file_package_var_alias_sentinel() {
    let fixture = build_go(&[
        (
            "decoy/types.go",
            "package decoy\ntype I interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\nfunc retain() { _ = Wrong{} }\n",
        ),
        (
            "app/a.go",
            "package app\nimport ext \"outside.example/api\"\nvar V ext.I\n",
        ),
        (
            "app/b.go",
            "package app\nimport ext \"example.com/root/decoy\"\nfunc run() { V.M() }\n",
        ),
    ]);
    assert_terminal_prerequisite_drop(&fixture, "app/b.go", "run", "M", "decoy/types.go", None);
}
