use prism::call_graph::CallGraph;
use prism::cpg::CodePropertyGraph;
use prism::navigation::{queries, NavigationIndex};
use prism::repo_loader::{load_repo, LoadedRepo};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

fn write_dispatch_fixture(root: &Path) {
    write(
        root,
        "api.go",
        "package root\n\
         type Context struct{}\n\
         type Doer interface { Act(Context) }\n\
         type Holder struct { Doer }\n\
         func invoke(h Holder, ctx Context) { h.Act(ctx) }\n",
    );
    write(
        root,
        "impl/impl.go",
        "package impl\n\
         import root \"example.com/root\"\n\
         type Impl struct{}\n\
         func (Impl) Act(root.Context) {}\n",
    );
}

fn write_nested_dispatch_fixture(root: &Path) {
    write(
        root,
        "nested/api.go",
        "package nested\n\
         type Context struct{}\n\
         type Doer interface { Act(Context) }\n\
         type Holder struct { Doer }\n\
         func invoke(h Holder, ctx Context) { h.Act(ctx) }\n",
    );
    write(
        root,
        "impl/impl.go",
        "package impl\n\
         import nested \"example.com/root/nested\"\n\
         type Impl struct{}\n\
         func (Impl) Act(nested.Context) {}\n",
    );
}

fn resolved_owners(repo: &LoadedRepo) -> BTreeSet<String> {
    let index = NavigationIndex::build(repo);
    let cg = index.call_graph();
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == "invoke" && site.callee_name == "Act")
        .expect("interface call site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|resolved| cg.method_owners.get(resolved.target).cloned())
        .collect()
}

fn resolved_owners_for(cg: &CallGraph, caller: &str, method: &str) -> BTreeSet<String> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == method)
        .expect("interface call site");
    cg.resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|resolved| cg.method_owners.get(resolved.target).cloned())
        .collect()
}

fn normalized_vec_map<K, V>(map: &BTreeMap<K, Vec<V>>) -> BTreeMap<K, BTreeSet<V>>
where
    K: Clone + Ord,
    V: Clone + Ord,
{
    map.iter()
        .map(|(key, values)| (key.clone(), values.iter().cloned().collect()))
        .collect()
}

fn assert_call_graph_parity(actual: &CallGraph, expected: &CallGraph) {
    assert_eq!(
        normalized_vec_map(&actual.functions),
        normalized_vec_map(&expected.functions)
    );
    assert_eq!(actual.calls, expected.calls);
    assert_eq!(
        normalized_vec_map(&actual.callers),
        normalized_vec_map(&expected.callers)
    );
    assert_eq!(
        normalized_vec_map(&actual.methods),
        normalized_vec_map(&expected.methods)
    );
    assert_eq!(actual.method_owners, expected.method_owners);
    assert_eq!(
        normalized_vec_map(&actual.interface_impls),
        normalized_vec_map(&expected.interface_impls)
    );
    assert_eq!(actual.method_arity, expected.method_arity);
    assert_eq!(actual.go_file_profiles, expected.go_file_profiles);
    assert_eq!(
        actual.go_interface_declarations,
        expected.go_interface_declarations
    );
    assert_eq!(
        actual.go_method_declarations,
        expected.go_method_declarations
    );
    assert_eq!(
        actual.go_interface_live_types,
        expected.go_interface_live_types
    );
    assert_eq!(queries::call_stats(actual), queries::call_stats(expected));
    assert_eq!(
        queries::interface_dispatch_manifest(actual),
        queries::interface_dispatch_manifest(expected)
    );
}

#[test]
fn loaded_manifest_snapshot_is_the_only_identity_source() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_dispatch_fixture(root);
    write(root, "go.mod", "module example.com/root\n");

    let loaded = load_repo(root).unwrap();
    write(root, "go.mod", "module example.com/changed\n");

    assert_eq!(
        resolved_owners(&loaded),
        BTreeSet::from(["Impl".to_string()])
    );
}

#[test]
fn unspaced_trailing_module_comment_preserves_proven_identity() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_dispatch_fixture(root);
    write(
        root,
        "go.mod",
        "module example.com/root// trailing comment\n",
    );

    let loaded = load_repo(root).unwrap();

    assert_eq!(
        resolved_owners(&loaded),
        BTreeSet::from(["Impl".to_string()])
    );
}

#[test]
fn gopkg_in_multielement_v2_module_preserves_proven_identity() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_dispatch_fixture(root);
    write(
        root,
        "impl/impl.go",
        "package impl\n\
         import root \"gopkg.in/user/foo.v2\"\n\
         type Impl struct{}\n\
         func (Impl) Act(root.Context) {}\n",
    );
    write(root, "go.mod", "module gopkg.in/user/foo.v2\n");

    let loaded = load_repo(root).unwrap();

    assert_eq!(
        resolved_owners(&loaded),
        BTreeSet::from(["Impl".to_string()])
    );
}

#[test]
fn carriage_return_does_not_end_a_go_mod_line_comment() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_dispatch_fixture(root);
    write(root, "go.mod", "// no module\rmodule example.com/root\n");

    let loaded = load_repo(root).unwrap();

    assert!(resolved_owners(&loaded).is_empty());
}

#[test]
fn carriage_return_between_module_and_path_preserves_proven_identity() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_dispatch_fixture(root);
    write(root, "go.mod", "module\r example.com/root\n");

    let loaded = load_repo(root).unwrap();

    assert_eq!(
        resolved_owners(&loaded),
        BTreeSet::from(["Impl".to_string()])
    );
}

#[cfg(unix)]
#[test]
fn symlinked_go_mod_is_refused_for_identity_even_when_its_target_is_valid() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_dispatch_fixture(root);
    write(root, "go-mod-target", "module example.com/root\n");
    std::os::unix::fs::symlink("go-mod-target", root.join("go.mod")).unwrap();

    let before = load_repo(root).unwrap();
    write(root, "go-mod-target", "module example.com/changed\n");
    let after = load_repo(root).unwrap();

    assert_eq!(before.manifest_hashes, after.manifest_hashes);
    assert!(resolved_owners(&before).is_empty());
    assert!(resolved_owners(&after).is_empty());
}

#[test]
fn malformed_nearest_go_mod_is_a_terminal_unproven_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_nested_dispatch_fixture(root);
    write(root, "go.mod", "module example.com/root\n");

    let inherited = load_repo(root).unwrap();
    write(root, "nested/go.mod", "module bad!path\n");
    let malformed_boundary = load_repo(root).unwrap();

    assert_eq!(
        resolved_owners(&inherited),
        BTreeSet::from(["Impl".to_string()])
    );
    assert!(resolved_owners(&malformed_boundary).is_empty());
}

#[cfg(unix)]
#[test]
fn symlinked_nearest_go_mod_is_a_terminal_unproven_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_nested_dispatch_fixture(root);
    write(root, "go.mod", "module example.com/root\n");
    write(root, "nested-go-mod-target", "module example.com/nested\n");
    std::os::unix::fs::symlink("../nested-go-mod-target", root.join("nested/go.mod")).unwrap();

    let loaded = load_repo(root).unwrap();

    assert!(resolved_owners(&loaded).is_empty());
}

#[test]
fn multi_module_full_incremental_and_cached_builds_are_identical() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "go.mod", "module example.com/root\n");
    write(root, "nested/go.mod", "module example.com/nested\n");
    write(root, "go.work", "go 1.22\nuse (\n.\n./nested\n)\n");
    write(
        root,
        "api.go",
        "package root\n\
         type Context struct{}\n\
         type Doer interface { Act(Context) }\n\
         type Holder struct { Doer }\n\
         func invokeRoot(h Holder, ctx Context) { h.Act(ctx) }\n",
    );
    write(
        root,
        "impl/impl.go",
        "package impl\n\
         import root \"example.com/root\"\n\
         type Impl struct{}\n\
         func (Impl) Act(root.Context) {}\n",
    );
    write(
        root,
        "nested/api.go",
        "package nested\n\
         type Context struct{}\n\
         type Worker interface { Work(Context) }\n\
         type Holder struct { Worker }\n\
         func invokeNested(h Holder, ctx Context) { h.Work(ctx) }\n",
    );
    write(
        root,
        "nested_impl/impl.go",
        "package nestedimpl\n\
         import nested \"example.com/nested\"\n\
         type NestedImpl struct{}\n\
         func (NestedImpl) Work(nested.Context) {}\n",
    );

    let initial_repo = load_repo(root).unwrap();
    let initial = NavigationIndex::build(&initial_repo);
    write(
        root,
        "impl/impl.go",
        "package impl\n\
         import root \"example.com/root\"\n\
         type Impl struct{}\n\
         func (Impl) Act(root.Context) {}\n\
         func helper() {}\n",
    );
    let updated_repo = load_repo(root).unwrap();
    let changed = BTreeSet::from(["impl/impl.go".to_string()]);
    let incremental = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
        initial.cpg().call_graph.clone(),
        initial.cpg().dfg.clone(),
        &changed,
        &updated_repo.files,
        None,
        updated_repo.scope_graph_inputs.as_ref(),
    );
    let full = NavigationIndex::build(&updated_repo);
    let cache = tempfile::tempdir().unwrap();
    let cached_miss = NavigationIndex::build_cached_under(&updated_repo, cache.path());
    let cached_hit = NavigationIndex::build_cached_under(&updated_repo, cache.path());

    let expected = full.call_graph();
    assert_call_graph_parity(&incremental.call_graph, expected);
    assert_call_graph_parity(cached_miss.call_graph(), expected);
    assert_call_graph_parity(cached_hit.call_graph(), expected);
    assert_eq!(
        resolved_owners_for(expected, "invokeRoot", "Act"),
        BTreeSet::from(["Impl".to_string()])
    );
    assert_eq!(
        resolved_owners_for(expected, "invokeNested", "Work"),
        BTreeSet::from(["NestedImpl".to_string()])
    );
}

#[test]
fn call_graph_consults_active_nested_module_identity_from_the_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "go.mod", "module example.com/root\n");
    write(root, "nested/go.mod", "module example.com/nested\n");
    write(root, "go.work", "go 1.22\nuse (\n.\n./nested\n)\n");
    write(
        root,
        "nested/api.go",
        "package nested\ntype Context struct{}\ntype Doer interface { Act(Context) }\ntype Holder struct { Doer }\nfunc invoke(h Holder, ctx Context) { h.Act(ctx) }\n",
    );
    write(
        root,
        "impl.go",
        "package root\nimport nested \"example.com/nested\"\ntype Impl struct{}\nfunc (Impl) Act(nested.Context) {}\n",
    );

    let loaded = load_repo(root).unwrap();
    assert_eq!(
        resolved_owners(&loaded),
        BTreeSet::from(["Impl".to_string()])
    );
}
