//! #765: which file a `#[path = "..."] mod m;` mount binds.
//!
//! Graph level: `populate_rust`, then the module scope a mount produced and the file
//! its extent names. rustc's rule (Rust Reference, "Modules", the `path` attribute;
//! confirmed against `rustc` for every layout below):
//!
//! - a non-inline mount is relative to the DECLARING FILE'S DIRECTORY, for a mod-rs
//!   file (`lib.rs`, `main.rs`, `mod.rs`) and a non-mod-rs file alike (`src/a.rs` +
//!   `#[path = "b.rs"]` loads `src/b.rs`, not `src/a/b.rs`);
//! - a mount inside an inline `mod` block is relative to that module's directory: the
//!   file's own directory for a mod-rs file, `<stem>/` for a non-mod-rs file, plus the
//!   inline module names;
//! - an inline `mod m { .. }` that itself carries `#[path = "p"]` makes `p` the
//!   directory its children (mounted or not) live in, relative to the declaring file's
//!   directory (not its `<stem>/`);
//! - a file loaded through `#[path]` is treated like a mod-rs file: its children
//!   (mounted or not) are siblings of it.
//!
//! RED rows (D1 D1b D1c D4 D7b D8 D8b D8c D12) fail on the pre-fix populator, which
//! resolved a non-inline mount relative to the declaring file's `<stem>/` directory,
//! did not normalize `.`/`..`, and put an inline `#[path]` module's children through a
//! stripped parent; KEEP rows (D2 D3 D5 D6 D7 D9 D9b D10 D10b D11 D13) pin what
//! already agreed with rustc. Each row kills a mutant of
//! `mutants/rust-path-placement.json` or pins a refusal.

use std::collections::BTreeMap;

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::engine::ScopeGraph;
use prism::name_resolution::rust_policy::NS_TYPE;
use prism::name_resolution::rust_populator::{
    enclosing_scope, file_id, populate_rust, RustCrateConfig,
};
use prism::name_resolution::types::{BindTarget, ScopeId, Target};

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs
        .iter()
        .map(|(p, s)| {
            (
                p.to_string(),
                ParsedFile::parse(p, s, Language::Rust).unwrap(),
            )
        })
        .collect()
}

fn populate(fs: &BTreeMap<String, ParsedFile>) -> ScopeGraph {
    let mut cfg = RustCrateConfig::from_convention(fs);
    cfg.edition = 2021;
    populate_rust(fs, &cfg, None)
}

/// The module scope reached from `root_file`'s Root scope by `chain`.
fn module(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    root_file: &str,
    chain: &[&str],
) -> ScopeId {
    let fid = file_id(fs, root_file).unwrap();
    let mut cur = enclosing_scope(g, fid, 0).expect("root");
    for seg in chain {
        cur = g
            .bindings
            .iter()
            .find_map(|b| match (&b.target, b.scope == cur, &b.name, b.ns) {
                (BindTarget::Resolved(Target::Scope(s)), true, n, NS_TYPE) if n == seg => Some(*s),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no module {seg} in {chain:?}"));
    }
    cur
}

/// The file a module scope was built from; `None` for a poisoned shell (a module whose
/// file the populator could not find: a zero-width extent in the declaring file).
fn file_of(g: &ScopeGraph, m: ScopeId) -> Option<String> {
    let scope = g.scope(m).expect("scope");
    let ext = &scope.extents[0];
    if ext.range.lo.byte == ext.range.hi.byte {
        return None;
    }
    g.file_paths
        .iter()
        .find(|(_, id)| **id == ext.file)
        .map(|(p, _)| p.clone())
}

/// The file `chain` binds from `src/lib.rs`.
fn bound(fs: &BTreeMap<String, ParsedFile>, chain: &[&str]) -> Option<String> {
    let g = populate(fs);
    file_of(&g, module(&g, fs, "src/lib.rs", chain))
}

fn some(p: &str) -> Option<String> {
    Some(p.to_string())
}

// ── RED rows ────────────────────────────────────────────────────────────────

#[test]
fn d1_non_mod_rs_declaring_file_mounts_beside_itself() {
    // rustc: `src/a.rs` is non-mod-rs; `#[path = "b.rs"]` is `src/b.rs`. `src/a/b.rs` is a decoy.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub fn f() {}\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "b"]), some("src/b.rs"));
}

#[test]
fn d1b_without_the_decoy_the_mount_is_still_found() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "b"]), some("src/b.rs"));
}

#[test]
fn d1c_a_file_only_at_the_stem_directory_is_not_rustcs() {
    // rustc fails with "file not found": no `src/a/b.rs` is loaded for this mount.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "b"]), None);
}

#[test]
fn d4_relative_subdirectory_from_a_non_mod_rs_file() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"impls/x.rs\"]\npub mod x;\n"),
        ("src/impls/x.rs", "pub fn f() {}\n"),
        ("src/a/impls/x.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "x"]), some("src/impls/x.rs"));
}

#[test]
fn d7b_mount_in_a_nested_non_mod_rs_file() {
    // `src/a.rs` declares `mod c;` (found at `src/a/c.rs`, non-mod-rs); its mount is
    // beside `src/a/c.rs`: `src/a/d.rs`, not `src/a/c/d.rs`.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod c;\n"),
        ("src/a/c.rs", "#[path = \"d.rs\"]\npub mod d;\n"),
        ("src/a/d.rs", "pub fn f() {}\n"),
        ("src/a/c/d.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "c", "d"]), some("src/a/d.rs"));
}

#[test]
fn d8_parent_directory_path() {
    // `src/lib.rs` + `../shared/s.rs` is `shared/s.rs`.
    let fs = files(&[
        ("src/lib.rs", "#[path = \"../shared/s.rs\"]\npub mod s;\n"),
        ("shared/s.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["s"]), some("shared/s.rs"));
}

#[test]
fn d8b_dot_segment_path() {
    let fs = files(&[
        ("src/lib.rs", "#[path = \"./b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["b"]), some("src/b.rs"));
}

#[test]
fn d8c_parent_path_from_a_non_mod_rs_file() {
    // `src/a.rs` + `../shared/s.rs` is `shared/s.rs` (the directory of `a.rs` is `src/`).
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"../shared/s.rs\"]\npub mod s;\n"),
        ("shared/s.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "s"]), some("shared/s.rs"));
}

#[test]
fn d12_inline_module_with_its_own_path_attribute() {
    // `#[path = "pp"] mod inner { .. }` in non-mod-rs `src/a.rs`: the children live in
    // `src/pp/` (relative to the file's directory), mounted or not.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "#[path = \"pp\"]\npub mod inner {\n    pub mod y;\n    #[path = \"z.rs\"]\n    pub mod z;\n}\n",
        ),
        ("src/pp/y.rs", "pub fn f() {}\n"),
        ("src/pp/z.rs", "pub fn f() {}\n"),
        ("src/a/pp/y.rs", "pub fn f() {}\n"),
        ("src/a/pp/z.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "inner", "y"]), some("src/pp/y.rs"));
    assert_eq!(bound(&fs, &["a", "inner", "z"]), some("src/pp/z.rs"));
}

// ── KEEP rows (already rustc's placement) ───────────────────────────────────

#[test]
fn d2_mod_rs_declaring_file_mounts_in_its_own_directory() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a/mod.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "b"]), some("src/a/b.rs"));
}

#[test]
fn d3_crate_roots_mount_in_their_directory() {
    let lib = files(&[
        ("src/lib.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub fn f() {}\n"),
        ("src/lib/b.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&lib, &["b"]), some("src/b.rs"));
    let main = files(&[
        ("src/main.rs", "#[path = \"b.rs\"]\nmod b;\nfn main() {}\n"),
        ("src/b.rs", "pub fn f() {}\n"),
        ("src/main/b.rs", "pub fn f() {}\n"),
    ]);
    let g = populate(&main);
    assert_eq!(
        file_of(&g, module(&g, &main, "src/main.rs", &["b"])),
        some("src/b.rs")
    );
}

#[test]
fn d5_inline_module_in_a_non_mod_rs_file_uses_the_stem_directory() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "pub mod inner {\n    #[path = \"z.rs\"]\n    pub mod z;\n}\n",
        ),
        ("src/a/inner/z.rs", "pub fn f() {}\n"),
        ("src/inner/z.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "inner", "z"]), some("src/a/inner/z.rs"));
}

#[test]
fn d6_inline_module_in_a_mod_rs_file_uses_its_own_directory() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a/mod.rs",
            "mod inner {\n    #[path = \"z.rs\"]\n    mod z;\n}\n",
        ),
        ("src/a/inner/z.rs", "pub fn f() {}\n"),
        ("src/a/a/inner/z.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "inner", "z"]), some("src/a/inner/z.rs"));
}

#[test]
fn d7_children_of_a_mounted_file_are_its_siblings() {
    // rustc treats a `#[path]`-loaded file like a mod-rs file: `mod c;`, a mount and an
    // inline-module mount in `src/x/foo.rs` resolve beside it, never under `src/x/foo/`.
    let fs = files(&[
        ("src/lib.rs", "#[path = \"x/foo.rs\"]\npub mod foo;\n"),
        (
            "src/x/foo.rs",
            "pub mod c;\n#[path = \"bar.rs\"]\npub mod bar;\npub mod i {\n    #[path = \"q.rs\"]\n    pub mod q;\n}\n",
        ),
        ("src/x/c.rs", "pub fn f() {}\n"),
        ("src/x/bar.rs", "pub fn f() {}\n"),
        ("src/x/i/q.rs", "pub fn f() {}\n"),
        ("src/x/foo/c.rs", "pub fn f() {}\n"),
        ("src/x/foo/bar.rs", "pub fn f() {}\n"),
        ("src/x/foo/i/q.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["foo"]), some("src/x/foo.rs"));
    assert_eq!(bound(&fs, &["foo", "c"]), some("src/x/c.rs"));
    assert_eq!(bound(&fs, &["foo", "bar"]), some("src/x/bar.rs"));
    assert_eq!(bound(&fs, &["foo", "i", "q"]), some("src/x/i/q.rs"));
}

#[test]
fn d9_mount_of_a_missing_file_binds_nothing() {
    // No panic; the module is a poisoned shell, never a decoy elsewhere.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"gone.rs\"]\npub mod b;\n"),
        ("src/gone2.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "b"]), None);
}

#[test]
fn d9b_absolute_and_root_escaping_paths_bind_nothing() {
    let fs = files(&[
        (
            "src/lib.rs",
            "#[path = \"/etc/b.rs\"]\npub mod abs;\n#[path = \"../../up.rs\"]\npub mod up;\n",
        ),
        ("etc/b.rs", "pub fn f() {}\n"),
        ("src/etc/b.rs", "pub fn f() {}\n"),
        ("up.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["abs"]), None);
    assert_eq!(bound(&fs, &["up"]), None);
}

#[test]
fn d10_plain_mod_in_a_non_mod_rs_file_still_uses_the_stem_directory() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
        ("src/b.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "b"]), some("src/a/b.rs"));
}

#[test]
fn d10b_plain_mod_in_an_inline_block_of_a_non_mod_rs_file() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "pub mod i {\n    pub mod j {\n        pub mod w;\n    }\n}\n",
        ),
        ("src/a/i/j/w.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "i", "j", "w"]), some("src/a/i/j/w.rs"));
}

#[test]
fn d11_cfg_attr_path_is_not_read() {
    // `#[cfg_attr(all(), path = "alt.rs")]` is not modeled (the attribute head is
    // `cfg_attr`): today's behaviour is the default location, `m.rs`, or a poisoned
    // shell when there is none. rustc would load `alt.rs`; this is an inherited gap.
    let with_default = files(&[
        (
            "src/lib.rs",
            "#[cfg_attr(all(), path = \"alt.rs\")]\npub mod m;\n",
        ),
        ("src/alt.rs", "pub fn f() {}\n"),
        ("src/m.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&with_default, &["m"]), some("src/m.rs"));
    let without = files(&[
        (
            "src/lib.rs",
            "#[cfg_attr(all(), path = \"alt.rs\")]\npub mod m;\n",
        ),
        ("src/alt.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&without, &["m"]), None);
}

#[test]
fn d13_two_inline_levels_in_a_non_mod_rs_file() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "pub mod i {\n    pub mod j {\n        #[path = \"q.rs\"]\n        pub mod q;\n    }\n}\n",
        ),
        ("src/a/i/j/q.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["a", "i", "j", "q"]), some("src/a/i/j/q.rs"));
}

// ── Review folds (#765 PR D): occurrences, blocks, unplaceable inline paths ──

/// Every chain, each bound from `src/lib.rs`, with the graph (for field checks).
fn bound_all(
    fs: &BTreeMap<String, ParsedFile>,
    chains: &[&[&str]],
) -> (ScopeGraph, Vec<Option<String>>) {
    let g = populate(fs);
    let out = chains
        .iter()
        .map(|c| file_of(&g, module(&g, fs, "src/lib.rs", c)))
        .collect();
    (g, out)
}

#[test]
fn o1_a_file_loaded_by_a_mount_and_a_plain_mod_is_two_populated_occurrences() {
    // sol W2: `src/b.rs` is `crate::a::b` (mount) and `crate::b` (plain). Both bind it,
    // whichever is declared first; the file is recorded as multiply populated.
    for lib in ["mod a;\nmod b;\n", "mod b;\nmod a;\n"] {
        let fs = files(&[
            ("src/lib.rs", lib),
            ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
            ("src/b.rs", "pub fn f() {}\n"),
        ]);
        let (g, got) = bound_all(&fs, &[&["a", "b"], &["b"]]);
        assert_eq!(got, vec![some("src/b.rs"), some("src/b.rs")], "{lib}");
        // Each occurrence is POPULATED: `f` is bound in both module scopes.
        for chain in [&["a", "b"][..], &["b"][..]] {
            let m = module(&g, &fs, "src/lib.rs", chain);
            assert!(
                g.bindings.iter().any(|b| b.scope == m && b.name == "f"),
                "{lib} {chain:?}"
            );
        }
        let b = file_id(&fs, "src/b.rs").unwrap();
        assert!(g.multi_occurrence_files.contains(&b), "{lib}");
    }
}

#[test]
fn o2_each_occurrence_searches_its_own_child_directory() {
    // Mounted: `src/b.rs` is mod-rs-like, `mod child;` is `src/child.rs`. Plain: `src/b/child.rs`.
    let fs = files(&[
        ("src/lib.rs", "mod a;\nmod b;\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub mod child;\n"),
        ("src/child.rs", "pub fn h() {}\n"),
        ("src/b/child.rs", "pub fn h() {}\n"),
    ]);
    let (_g, got) = bound_all(&fs, &[&["a", "b", "child"], &["b", "child"]]);
    assert_eq!(got, vec![some("src/child.rs"), some("src/b/child.rs")]);
}

#[test]
fn o3_a_path_cycle_terminates() {
    // `a.rs` mounts `b.rs`, `b.rs` mounts `a.rs`: the loader's active-stack check.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "#[path = \"a.rs\"]\npub mod a;\n"),
    ]);
    let (_g, got) = bound_all(&fs, &[&["a", "b"], &["a", "b", "a"]]);
    assert_eq!(got[0], some("src/b.rs"));
}

#[test]
fn o4_an_unshared_file_is_not_multiply_populated() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert!(g.multi_occurrence_files.is_empty());
}

#[test]
fn w3_a_mount_inside_an_unplaceable_inline_module_binds_nothing() {
    // `#[path = "/nonexistent-dir"]` is absolute: its children's directory is unplaceable and
    // `../x.rs` must not climb out of it to a repository file (rustc reads `/x.rs`).
    let fs = files(&[
        (
            "src/lib.rs",
            "#[path = \"/nonexistent-dir\"]\npub mod i {\n    #[path = \"../x.rs\"]\n    pub mod m;\n}\n",
        ),
        ("x.rs", "pub fn f() {}\n"),
        ("src/x.rs", "pub fn f() {}\n"),
    ]);
    assert_eq!(bound(&fs, &["i", "m"]), None);
}
