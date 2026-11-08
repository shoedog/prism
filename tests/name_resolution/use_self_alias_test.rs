//! #724: `use p::{self as m}` and `use p::{self}` at the scope-graph level.
//!
//! A self import binds the MODULE in the type namespace only (Rust Reference,
//! "self imports"), and `{self as m}` must not keep the literal `self` segment
//! in the path. RED rows fail while the populator emits `Pending(p::self)` in
//! both namespaces; KEEP rows pin ordinary imports and refusal. Each row kills
//! a mutant of `mutants/rust-use-spelling.json`.

use std::collections::BTreeMap;

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::engine::{resolve, resolve_path, ScopeGraph};
use prism::name_resolution::rust_policy::{RustPolicy, NS_TYPE, NS_VALUE};
use prism::name_resolution::rust_populator::{
    enclosing_scope, file_id, populate_rust, RustCrateConfig,
};
use prism::name_resolution::types::{
    Anchor, BindTarget, NamespaceId, RawPath, ResStatus, Resolution, ResolveQuery, ScopeId,
    SourceLoc, Target,
};

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

fn populate(fs: &BTreeMap<String, ParsedFile>, edition: u16) -> ScopeGraph {
    let mut cfg = RustCrateConfig::from_convention(fs);
    cfg.edition = edition;
    populate_rust(fs, &cfg, None)
}

fn from_in(g: &ScopeGraph, fs: &BTreeMap<String, ParsedFile>, file: &str) -> (ScopeId, SourceLoc) {
    let fid = file_id(fs, file).unwrap();
    (
        enclosing_scope(g, fid, 0).expect("scope"),
        SourceLoc { file: fid, byte: 0 },
    )
}

/// `(ns, target)` of every import binding named `name` whose target is `Pending`.
fn pending(g: &ScopeGraph, name: &str) -> Vec<(NamespaceId, RawPath, Anchor)> {
    let mut v = g
        .bindings
        .iter()
        .filter(|b| b.name == name)
        .filter_map(|b| match &b.target {
            BindTarget::Pending(p, a) => Some((b.ns, p.clone(), *a)),
            _ => None,
        })
        .collect::<Vec<_>>();
    v.sort_by_key(|t| t.0);
    v
}

fn path(segs: &[&str]) -> RawPath {
    RawPath(segs.iter().map(|s| s.to_string()).collect())
}

fn path_from(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    edition: u16,
    file: &str,
    segs: &[&str],
) -> Resolution {
    let (from, at) = from_in(g, fs, file);
    let pol = RustPolicy::new(g, edition);
    resolve_path(
        g,
        &path(segs),
        NS_VALUE,
        &Anchor::bare(),
        from,
        NS_TYPE,
        &at,
        &pol,
    )
}

fn bare_value(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    edition: u16,
    file: &str,
    name: &str,
) -> Resolution {
    let (from, at) = from_in(g, fs, file);
    let pol = RustPolicy::new(g, edition);
    resolve(
        g,
        &ResolveQuery {
            name: name.to_string(),
            ns: NS_VALUE,
            from,
            at,
            cfg: Default::default(),
            ctx: Default::default(),
        },
        &pol,
    )
}

fn assert_item(r: &Resolution) {
    assert_eq!(r.status, ResStatus::Resolved, "{r:?}");
    assert!(
        matches!(r.candidates[0].target, Target::Item { .. }),
        "{r:?}"
    );
}

fn assert_not_resolved(r: &Resolution) {
    assert!(
        !matches!(r.status, ResStatus::Resolved | ResStatus::ResolvedSet),
        "{r:?}"
    );
}

fn crate_fs(c: &str) -> BTreeMap<String, ParsedFile> {
    files(&[
        ("src/lib.rs", "mod a;\nmod c;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub fn f() {}\npub fn g() {}\n"),
        ("src/c.rs", c),
    ])
}

// ── RED rows ────────────────────────────────────────────────────────────────

#[test]
fn b1_alias_binds_the_prefix_module_in_the_type_namespace_only() {
    let fs = crate_fs("use crate::a::b::{self as m, g};\npub fn caller() {}\n");
    let g = populate(&fs, 2021);
    // No `self` segment, anchored at the crate root, type namespace only.
    assert_eq!(
        pending(&g, "m"),
        vec![(NS_TYPE, path(&["a", "b"]), Anchor::crate_root())]
    );
    assert_item(&path_from(&g, &fs, 2021, "src/c.rs", &["m", "f"]));
}

#[test]
fn b1_ordinary_named_sibling_keeps_both_namespaces() {
    let fs = crate_fs("use crate::a::b::{self as m, g};\npub fn caller() {}\n");
    let g = populate(&fs, 2021);
    let both = pending(&g, "g");
    assert_eq!(
        both,
        vec![
            (NS_TYPE, path(&["a", "b", "g"]), Anchor::crate_root()),
            (NS_VALUE, path(&["a", "b", "g"]), Anchor::crate_root()),
        ]
    );
}

#[test]
fn b2_relative_anchor_survives_the_strip() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\nmod c;\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
        (
            "src/a/c.rs",
            "use super::b::{self as m};\npub fn caller() {}\n",
        ),
    ]);
    let g = populate(&fs, 2021);
    assert_eq!(
        pending(&g, "m"),
        vec![(NS_TYPE, path(&["b"]), Anchor::super_n(1))]
    );
    assert_item(&path_from(&g, &fs, 2021, "src/a/c.rs", &["m", "f"]));
}

#[test]
fn b2n_nested_group_alias() {
    let fs = crate_fs("use crate::a::{b::{self as m}, b::g};\npub fn caller() {}\n");
    let g = populate(&fs, 2021);
    assert_eq!(
        pending(&g, "m"),
        vec![(NS_TYPE, path(&["a", "b"]), Anchor::crate_root())]
    );
    assert_item(&path_from(&g, &fs, 2021, "src/c.rs", &["m", "f"]));
}

#[test]
fn b7_leading_self_is_kept_in_an_edition_2015_crate() {
    let fs = files(&[
        ("src/lib.rs", "mod p;\nmod a;\n"),
        ("src/p.rs", "pub fn f() {}\n"),
        (
            "src/a.rs",
            "mod p;\nuse self::p::{self as m};\npub fn c() {}\n",
        ),
        ("src/a/p.rs", "pub fn f() {}\n"),
    ]);
    let g = populate(&fs, 2015);
    assert_eq!(
        pending(&g, "m"),
        vec![(NS_TYPE, path(&["p"]), Anchor::self_mod())]
    );
}

#[test]
fn b12_aliased_self_import_is_invisible_to_the_value_namespace() {
    // `fn m` in the value namespace; the alias `m` is the module `p`.
    let fs = files(&[
        (
            "src/lib.rs",
            "pub mod p;\npub fn p() {}\npub fn m() {}\nuse crate::p::{self as m};\n",
        ),
        ("src/p.rs", "pub fn other() {}\n"),
    ]);
    let g = populate(&fs, 2021);
    assert!(pending(&g, "m").iter().all(|(ns, _, _)| *ns == NS_TYPE));
    assert_item(&bare_value(&g, &fs, 2021, "src/lib.rs", "m"));
}

#[test]
fn b13_unaliased_self_import_is_invisible_to_the_value_namespace() {
    let fs = files(&[
        ("src/lib.rs", "pub mod p;\npub fn p() {}\nmod q;\n"),
        ("src/p.rs", "pub fn x() {}\n"),
        (
            "src/q.rs",
            "use crate::p::{self};\nfn p() {}\npub fn c() {}\n",
        ),
    ]);
    let g = populate(&fs, 2021);
    // The import binds `p` (the module) in the type namespace only.
    let imports = g
        .bindings
        .iter()
        .filter(|b| b.name == "p" && matches!(b.target, BindTarget::Pending(..)))
        .map(|b| b.ns)
        .collect::<Vec<_>>();
    assert_eq!(imports, vec![NS_TYPE]);
    // So the local `fn p` is the only value binding: it resolves.
    assert_item(&bare_value(&g, &fs, 2021, "src/q.rs", "p"));
}

// ── KEEP rows ───────────────────────────────────────────────────────────────

#[test]
fn b5_two_aliases_of_one_name_do_not_resolve() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\nmod c;\nmod d;\n"),
        ("src/a.rs", "pub fn f() {}\n"),
        ("src/c.rs", "pub fn f() {}\n"),
        (
            "src/d.rs",
            "use crate::a::{self as m};\nuse crate::c::{self as m};\npub fn caller() {}\n",
        ),
    ]);
    let g = populate(&fs, 2021);
    assert_not_resolved(&path_from(&g, &fs, 2021, "src/d.rs", &["m", "f"]));
}

#[test]
fn b6_alias_and_a_local_module_of_one_name_do_not_resolve() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\nmod d;\n"),
        ("src/a.rs", "pub fn f() {}\n"),
        (
            "src/d.rs",
            "use crate::a::{self as m};\nmod m {\n    pub fn f() {}\n}\npub fn caller() {}\n",
        ),
    ]);
    let g = populate(&fs, 2021);
    assert_not_resolved(&path_from(&g, &fs, 2021, "src/d.rs", &["m", "f"]));
}

#[test]
fn b14_ordinary_import_binds_both_namespaces() {
    let fs = files(&[
        ("src/lib.rs", "mod x;\nmod y;\n"),
        ("src/x.rs", "pub fn f() {}\n"),
        ("src/y.rs", "use crate::x::f;\npub fn caller() {}\n"),
    ]);
    let g = populate(&fs, 2021);
    let ns = pending(&g, "f")
        .into_iter()
        .map(|(n, _, _)| n)
        .collect::<Vec<_>>();
    assert_eq!(ns, vec![NS_TYPE, NS_VALUE]);
    assert_item(&bare_value(&g, &fs, 2021, "src/y.rs", "f"));
}

#[test]
fn b16_a_path_without_a_preceding_segment_keeps_a_lone_self() {
    // `use self::{self as m}`: the prefix is the `self` anchor, so only the TRAILING
    // marker is dropped and `m` names the current module.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "use self::{self as m};\npub fn f() {}\n"),
    ]);
    let g = populate(&fs, 2021);
    assert_eq!(
        pending(&g, "m"),
        vec![(NS_TYPE, path(&[]), Anchor::self_mod())]
    );
}
