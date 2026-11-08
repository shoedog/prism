//! #749: `crate_deps_by_root` for binary roots.
//!
//! Manifest-parsed fixtures through `load_repo` and `populate_rust`. Each row asserts
//! the complete dependency map keyed by root FILE (library roots, own-library entries
//! and the dependency entries of bin roots), so a mutant that keys one root too many or
//! too few fails. RED rows (D1 D2 D5 D6 D9) fail while `Builder::finish` keys library
//! roots only; KEEP rows pin the retained exclusions. Each row kills a mutant of
//! `mutants/rust-bin-root-deps.json`.
use std::collections::BTreeMap;

use prism::name_resolution::rust_populator::populate_rust;
use prism::repo_loader::load_repo;
use tempfile::TempDir;

type Keyed = BTreeMap<String, BTreeMap<String, String>>;

const WS: &str = "[workspace]\nmembers = [\"a\", \"b\"]\n";
const A_TOML: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const B_HEAD: &str = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const B_DEP: &str = "[dependencies]\na = { path = \"../a\" }\n";
const MAIN: &str = "fn main() {}\n";

fn pairs(rows: &[(&str, &[(&str, &str)])]) -> Keyed {
    rows.iter()
        .map(|(root, deps)| {
            (
                (*root).to_string(),
                deps.iter()
                    .map(|(n, t)| ((*n).to_string(), (*t).to_string()))
                    .collect(),
            )
        })
        .collect()
}

/// Workspace `a` (library) + `b` (manifest `b_toml`, files `b_files`): every root's
/// dependency map, by file path.
fn keyed(a_toml: &str, b_toml: &str, b_files: &[(&str, &str)]) -> Keyed {
    let dir = TempDir::new().unwrap();
    let mut all = vec![
        ("Cargo.toml", WS),
        ("a/Cargo.toml", a_toml),
        ("a/src/lib.rs", "pub fn f() {}\n"),
        ("b/Cargo.toml", b_toml),
    ];
    all.extend_from_slice(b_files);
    for (path, src) in all {
        let abs = dir.path().join(path);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(abs, src).unwrap();
    }
    let repo = load_repo(dir.path()).unwrap();
    let cfg = &repo.scope_graph_inputs.as_ref().unwrap().cfg;
    let graph = populate_rust(&repo.files, cfg, None);
    let path_of = |scope| {
        let file = graph.scope(scope).unwrap().extents[0].file;
        graph
            .file_paths
            .iter()
            .find(|(_, id)| **id == file)
            .map(|(p, _)| p.clone())
            .unwrap()
    };
    graph
        .crate_deps_by_root
        .iter()
        .map(|(root, deps)| {
            (
                path_of(*root),
                deps.iter().map(|(n, t)| (n.clone(), path_of(*t))).collect(),
            )
        })
        .collect()
}

fn b_toml(extra_head: &str, deps: &str) -> String {
    format!("{B_HEAD}{extra_head}{deps}")
}

const LIB: (&str, &str) = ("b/src/lib.rs", "pub fn lib_fn() {}\n");

#[test]
fn d1_every_bin_shape_is_keyed_with_the_dependency() {
    let got = keyed(
        A_TOML,
        &b_toml("", B_DEP),
        &[
            LIB,
            ("b/src/main.rs", MAIN),
            ("b/src/bin/tool.rs", MAIN),
            ("b/src/bin/multi/main.rs", MAIN),
            // Not auto-bins: nested files below `src/bin/<name>/` other than main.rs.
            ("b/src/bin/multi/helper.rs", "pub fn h() {}\n"),
        ],
    );
    let bin = [("a", "a/src/lib.rs"), ("b", "b/src/lib.rs")];
    assert_eq!(
        got,
        pairs(&[
            ("b/src/lib.rs", &[("a", "a/src/lib.rs")]),
            ("b/src/main.rs", &bin),
            ("b/src/bin/tool.rs", &bin),
            ("b/src/bin/multi/main.rs", &bin),
        ])
    );
}

#[test]
fn d2_renamed_dependency_is_keyed_under_its_in_source_name() {
    let got = keyed(
        A_TOML,
        &b_toml(
            "",
            "[dependencies]\nalpha = { package = \"a\", path = \"../a\" }\n",
        ),
        &[LIB, ("b/src/main.rs", MAIN)],
    );
    assert_eq!(
        got,
        pairs(&[
            ("b/src/lib.rs", &[("alpha", "a/src/lib.rs")]),
            (
                "b/src/main.rs",
                &[("alpha", "a/src/lib.rs"), ("b", "b/src/lib.rs")]
            ),
        ])
    );
}

#[test]
fn d5_hyphenated_dependency_name_is_normalized() {
    let a = "[package]\nname = \"a-core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    let got = keyed(
        a,
        &b_toml("", "[dependencies]\na-core = { path = \"../a\" }\n"),
        &[LIB, ("b/src/main.rs", MAIN)],
    );
    assert_eq!(
        got["b/src/main.rs"],
        pairs(&[("x", &[("a_core", "a/src/lib.rs"), ("b", "b/src/lib.rs")])])["x"]
    );
}

#[test]
fn d6_bin_only_member_is_keyed() {
    let got = keyed(A_TOML, &b_toml("", B_DEP), &[("b/src/main.rs", MAIN)]);
    assert_eq!(got, pairs(&[("b/src/main.rs", &[("a", "a/src/lib.rs")])]));
}

#[test]
fn d9_explicit_bin_path_is_keyed_and_replaces_auto_discovery() {
    let got = keyed(
        A_TOML,
        &b_toml("[[bin]]\nname = \"x\"\npath = \"tools/x.rs\"\n", B_DEP),
        &[
            LIB,
            ("b/tools/x.rs", MAIN),
            ("b/src/main.rs", MAIN),
            ("b/src/bin/tool.rs", MAIN),
        ],
    );
    assert_eq!(
        got,
        pairs(&[
            ("b/src/lib.rs", &[("a", "a/src/lib.rs")]),
            (
                "b/tools/x.rs",
                &[("a", "a/src/lib.rs"), ("b", "b/src/lib.rs")]
            ),
        ])
    );
}

// ── KEEP ────────────────────────────────────────────────────────────────────

#[test]
fn d3_no_dependency_entry_keys_only_the_own_library() {
    let got = keyed(A_TOML, &b_toml("", ""), &[LIB, ("b/src/main.rs", MAIN)]);
    assert_eq!(got, pairs(&[("b/src/main.rs", &[("b", "b/src/lib.rs")])]));
}

#[test]
fn d4_dev_dependency_and_build_script_are_never_keyed() {
    let got = keyed(
        A_TOML,
        &b_toml("", "[dev-dependencies]\na = { path = \"../a\" }\n[build-dependencies]\na = { path = \"../a\" }\n"),
        &[LIB, ("b/src/main.rs", MAIN), ("b/build.rs", MAIN)],
    );
    assert_eq!(got, pairs(&[("b/src/main.rs", &[("b", "b/src/lib.rs")])]));
}

#[test]
fn d4b_build_script_is_not_keyed_even_with_a_normal_dependency() {
    let got = keyed(
        A_TOML,
        &b_toml("", B_DEP),
        &[LIB, ("b/src/main.rs", MAIN), ("b/build.rs", MAIN)],
    );
    assert!(!got.contains_key("b/build.rs"), "{got:?}");
    assert!(got.contains_key("b/src/main.rs"), "{got:?}");
}

#[test]
fn d4c_tests_examples_and_benches_roots_are_not_keyed() {
    let got = keyed(
        A_TOML,
        &b_toml("", B_DEP),
        &[
            LIB,
            ("b/src/main.rs", MAIN),
            ("b/tests/it.rs", MAIN),
            ("b/examples/ex.rs", MAIN),
            ("b/benches/bn.rs", MAIN),
        ],
    );
    let mut roots: Vec<&str> = got.keys().map(String::as_str).collect();
    roots.sort_unstable();
    assert_eq!(roots, ["b/src/lib.rs", "b/src/main.rs"], "{got:?}");
}

#[test]
fn d7_edition_2015_package_keys_nothing_for_bins() {
    let a = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2015\"\n";
    let b = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2015\"\n[dependencies]\na = { path = \"../a\" }\n";
    let got = keyed(a, b, &[LIB, ("b/src/main.rs", MAIN)]);
    assert!(!got.contains_key("b/src/main.rs"), "{got:?}");
}

#[test]
fn d8_autobins_false_keys_no_auto_bin() {
    let got = keyed(
        A_TOML,
        &b_toml("autobins = false\n", B_DEP),
        &[LIB, ("b/src/main.rs", MAIN), ("b/src/bin/tool.rs", MAIN)],
    );
    assert_eq!(got, pairs(&[("b/src/lib.rs", &[("a", "a/src/lib.rs")])]));
}

#[test]
fn d10_bin_with_its_own_edition_is_skipped_per_target() {
    let got = keyed(
        A_TOML,
        &b_toml(
            "[[bin]]\nname = \"tool\"\npath = \"src/bin/tool.rs\"\nedition = \"2018\"\n\
             [[bin]]\nname = \"plain\"\npath = \"src/bin/plain.rs\"\n",
            B_DEP,
        ),
        &[
            LIB,
            ("b/src/bin/tool.rs", MAIN),
            ("b/src/bin/plain.rs", MAIN),
        ],
    );
    assert!(!got.contains_key("b/src/bin/tool.rs"), "{got:?}");
    assert_eq!(
        got["b/src/bin/plain.rs"],
        pairs(&[("x", &[("a", "a/src/lib.rs"), ("b", "b/src/lib.rs")])])["x"]
    );
}

#[test]
fn d11_dependency_named_like_the_own_library_is_dropped_alone() {
    // The own library is `x`; the dependency `x` (package a) would be a second `x`:
    // neither is inserted. The sibling dependency `a` still is.
    let got = keyed(
        A_TOML,
        &b_toml(
            "[lib]\nname = \"x\"\n",
            "[dependencies]\na = { path = \"../a\" }\nx = { package = \"a\", path = \"../a\" }\n",
        ),
        &[LIB, ("b/src/main.rs", MAIN)],
    );
    assert_eq!(
        got["b/src/main.rs"],
        pairs(&[("k", &[("a", "a/src/lib.rs")])])["k"]
    );
    assert!(
        got["b/src/lib.rs"].contains_key("x"),
        "the library root keeps its own keying (unchanged v1 rule): {got:?}"
    );
}

#[test]
fn d12_a_target_without_a_library_root_is_not_a_dependency_entry() {
    // `b` depends on `c`, a bin-only package: not `use`-nameable, so no entry.
    let c = "[package]\nname = \"c\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    let dir = TempDir::new().unwrap();
    for (path, src) in [
        ("Cargo.toml", "[workspace]\nmembers = [\"b\", \"c\"]\n"),
        ("c/Cargo.toml", c),
        ("c/src/main.rs", MAIN),
        ("b/Cargo.toml", "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[dependencies]\nc = { path = \"../c\" }\n"),
        ("b/src/main.rs", MAIN),
    ] {
        let abs = dir.path().join(path);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(abs, src).unwrap();
    }
    let repo = load_repo(dir.path()).unwrap();
    let graph = populate_rust(
        &repo.files,
        &repo.scope_graph_inputs.as_ref().unwrap().cfg,
        None,
    );
    assert!(
        graph.crate_deps_by_root.values().all(|m| m.is_empty()),
        "{:?}",
        graph.crate_deps_by_root
    );
}
