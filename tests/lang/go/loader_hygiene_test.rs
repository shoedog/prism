use prism::navigation::NavigationIndex;
use prism::repo_loader::{load_repo, LoadedRepo};
use std::collections::BTreeSet;
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
