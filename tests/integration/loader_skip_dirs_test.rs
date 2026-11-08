//! #739: `build/` and `dist/` are skipped only at the repository root; the
//! loader indexes a nested source package that carries the name, and
//! `.git`/`target`/`node_modules`/`vendor` stay skipped at every depth.
use prism::repo_loader::{collect_freshness_source_paths, load_repo, SkipReason};
use std::collections::BTreeSet;

fn write(root: &std::path::Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
}

#[test]
fn nested_build_and_dist_sources_are_loaded_root_ones_are_not() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            ("src/lib.rs", "pub fn f() {}\n"),
            ("build/a.rs", "pub fn a() {}\n"),
            ("dist/b.js", "export function b() {}\n"),
            ("src/x/build/c.rs", "pub fn c() {}\n"),
            ("pkg/dist/d.go", "package dist\n\nfunc D() {}\n"),
            ("src/node_modules/e.js", "export function e() {}\n"),
            ("src/target/f.rs", "pub fn f2() {}\n"),
            ("src/vendor/g.rs", "pub fn g() {}\n"),
        ],
    );
    let repo = load_repo(dir.path()).unwrap();
    let files: BTreeSet<&str> = repo.files.keys().map(String::as_str).collect();
    assert_eq!(
        files,
        BTreeSet::from(["pkg/dist/d.go", "src/lib.rs", "src/x/build/c.rs"])
    );
    for root_dir in [
        "build/",
        "dist/",
        "src/node_modules/",
        "src/target/",
        "src/vendor/",
    ] {
        assert!(
            repo.skipped
                .iter()
                .any(|s| s.path == root_dir && s.reason == SkipReason::Ignored),
            "{root_dir} must be Ignored"
        );
    }
    let loaded: BTreeSet<String> = repo.files.keys().cloned().collect();
    assert_eq!(collect_freshness_source_paths(dir.path()).unwrap(), loaded);
}
