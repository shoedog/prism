//! #739: `build/` and `dist/` are build-output names only at the repository
//! root; `.git`, `target`, `node_modules` and `vendor` are skipped at every
//! depth. Every walk that applies the rule is pinned on one fixture.
use super::*;

/// Repository root with build output at the root, the same names nested as
/// real source packages, and the always-skipped names at both depths.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let rust = "pub fn f() {}\n";
    for (path, text) in [
        ("src/lib.rs", rust),
        // Root build output: skipped.
        ("build/a.rs", rust),
        ("build/go.mod", "module example/rootbuild\n"),
        ("dist/b.js", "export function b() {}\n"),
        // Nested source packages that happen to carry the names: indexed.
        ("src/x/build/c.rs", rust),
        ("src/x/build/go.mod", "module example/nestedbuild\n"),
        ("pkg/dist/d.go", "package dist\n\nfunc D() {}\n"),
        ("src/dist/e.js", "export function e() {}\n"),
        // Skipped at every depth.
        ("target/f.rs", rust),
        ("src/target/f2.rs", rust),
        ("vendor/g.rs", rust),
        ("src/vendor/g2.rs", rust),
        ("node_modules/h.js", "export function h() {}\n"),
        ("src/node_modules/h2.js", "export function h2() {}\n"),
        (".hidden/i.rs", rust),
        ("src/.hidden/i2.rs", rust),
    ] {
        let p = dir.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir
}

const INDEXED: &[&str] = &[
    "pkg/dist/d.go",
    "src/dist/e.js",
    "src/lib.rs",
    "src/x/build/c.rs",
];

const NEVER_INDEXED: &[&str] = &[
    "build/a.rs",
    "dist/b.js",
    "target/f.rs",
    "src/target/f2.rs",
    "vendor/g.rs",
    "src/vendor/g2.rs",
    "node_modules/h.js",
    "src/node_modules/h2.js",
    ".hidden/i.rs",
    "src/.hidden/i2.rs",
];

fn assert_listing(listing: &BTreeSet<String>, what: &str) {
    for p in INDEXED {
        assert!(
            listing.contains(*p),
            "{what}: {p} must be listed: {listing:?}"
        );
    }
    for p in NEVER_INDEXED {
        assert!(!listing.contains(*p), "{what}: {p} must not be listed");
    }
}

#[test]
fn load_repo_indexes_nested_build_and_dist_and_skips_root_ones() {
    let dir = fixture();
    let repo = load_repo(dir.path()).unwrap();
    let files: BTreeSet<String> = repo.files.keys().cloned().collect();
    assert_listing(&files, "load_repo files");
    let ignored: BTreeSet<&str> = repo
        .skipped
        .iter()
        .filter(|s| s.reason == SkipReason::Ignored)
        .map(|s| s.path.as_str())
        .collect();
    for d in [
        "build/",
        "dist/",
        "target/",
        "vendor/",
        "node_modules/",
        "src/target/",
        "src/vendor/",
        "src/node_modules/",
    ] {
        assert!(ignored.contains(d), "{d} must be Ignored: {ignored:?}");
    }
    for d in ["src/x/build/", "pkg/dist/", "src/dist/"] {
        assert!(!ignored.contains(d), "{d} must not be Ignored");
    }
    // The walk also feeds the manifest snapshot: nested go.mod seen, root one not.
    assert!(repo.manifest_hashes.contains_key("src/x/build/go.mod"));
    assert!(!repo.manifest_hashes.contains_key("build/go.mod"));
}

#[test]
fn manifest_snapshot_walk_sees_nested_build_manifests_only() {
    let dir = fixture();
    let snapshot = collect_manifest_snapshot(dir.path());
    let paths: BTreeSet<&str> = snapshot.entries().map(|(p, _)| p.as_str()).collect();
    assert!(paths.contains("src/x/build/go.mod"), "{paths:?}");
    assert!(!paths.contains("build/go.mod"), "{paths:?}");
}

#[test]
fn supported_source_listing_walk_follows_the_same_rule() {
    let dir = fixture();
    let listing = collect_supported_source_paths(dir.path()).unwrap();
    assert_listing(&listing, "source listing");
}

#[test]
fn freshness_walk_follows_the_same_rule() {
    let dir = fixture();
    let listing = collect_freshness_source_paths(dir.path()).unwrap();
    assert_listing(&listing, "freshness listing");
}

#[test]
fn freshness_listing_agrees_with_the_loaded_files() {
    let dir = fixture();
    let repo = load_repo(dir.path()).unwrap();
    let loaded: BTreeSet<String> = repo.files.keys().cloned().collect();
    assert_eq!(collect_freshness_source_paths(dir.path()).unwrap(), loaded);
}

#[test]
fn skip_dir_rule() {
    for name in BUILTIN_SKIP_DIRS {
        assert!(skip_dir(name, true) && skip_dir(name, false), "{name}");
    }
    for name in ["build", "dist"] {
        assert!(skip_dir(name, true), "{name} at the root");
        assert!(!skip_dir(name, false), "{name} nested");
    }
    for name in ["src", "builds", "Build", "distro"] {
        assert!(!skip_dir(name, true) && !skip_dir(name, false), "{name}");
    }
}
