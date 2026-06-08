use prism::navigation::NavigationIndex;
use prism::repo_loader::load_repo;

#[test]
fn name_index_keeps_all_same_name_defs() {
    let dir = tempfile::tempdir().unwrap();
    // Two `new` fns in one Rust file across impl blocks (the func_index collision case).
    std::fs::write(
        dir.path().join("x.rs"),
        "struct A; struct B;\nimpl A { fn new() -> A { A } }\nimpl B { fn new() -> B { B } }\n",
    )
    .unwrap();
    let repo = std::sync::Arc::new(load_repo(dir.path()).unwrap());
    let idx = NavigationIndex::build(&repo);
    let defs = idx.name_index.get(&("x.rs".into(), "new".into())).unwrap();
    assert_eq!(
        defs.len(),
        2,
        "both `new` defs must be retained, not collapsed"
    );
}

#[test]
fn line_range_index_resolves_innermost() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.py"),
        "def outer():\n    def inner():\n        return 1\n    return inner()\n",
    )
    .unwrap();
    let repo = std::sync::Arc::new(load_repo(dir.path()).unwrap());
    let idx = NavigationIndex::build(&repo);
    let f = idx.enclosing_function("a.py", 3).unwrap(); // line 3 is inside `inner`
    assert_eq!(f.1, "inner");
}
