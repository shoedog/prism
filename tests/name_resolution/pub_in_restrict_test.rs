//! #710: `pub(in <path>)` restrict paths are resolved after population.
//!
//! Graph level: `populate_rust` then the engine. Each row asserts the restrict
//! the populator recorded (`Vis::restrict`) and the engine's answer. RED rows
//! fail while the populator discards the path; KEEP rows (restrict `None`,
//! refusal) pin the "refuse what cannot be placed" property and each kills one
//! mutant of `mutants/rust-pub-in-restrict.json`.

use std::collections::BTreeMap;

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::engine::{resolve, resolve_path, ScopeGraph};
use prism::name_resolution::rust_policy::{
    RustPolicy, NS_TYPE, NS_VALUE, VIS_PRIV, VIS_PUB_CRATE, VIS_PUB_IN, VIS_PUB_SUPER,
};
use prism::name_resolution::rust_populator::{
    enclosing_scope, file_id, populate_rust, RustCrateConfig,
};
use prism::name_resolution::types::{
    Anchor, BindTarget, RawPath, ResStatus, Resolution, ResolveQuery, ScopeId, SourceLoc, Target,
};

const EDITION: u16 = 2021;

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
    cfg.edition = EDITION;
    populate_rust(fs, &cfg, None)
}

/// The module scope reached from the `src/lib.rs` Root by `chain` (`NS_TYPE` bindings).
fn module(g: &ScopeGraph, fs: &BTreeMap<String, ParsedFile>, chain: &[&str]) -> ScopeId {
    let fid = file_id(fs, "src/lib.rs").unwrap();
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

/// `restrict` of every `pub(in)` Value binding named `name`.
fn restricts(g: &ScopeGraph, name: &str) -> Vec<Option<ScopeId>> {
    let v = g
        .bindings
        .iter()
        .filter(|b| b.name == name && b.vis.kind == VIS_PUB_IN)
        .map(|b| b.vis.restrict)
        .collect::<Vec<_>>();
    assert!(!v.is_empty(), "no pub(in) binding {name}");
    v
}

fn from_in(g: &ScopeGraph, fs: &BTreeMap<String, ParsedFile>, file: &str) -> (ScopeId, SourceLoc) {
    let fid = file_id(fs, file).unwrap();
    // A byte after the file's first item header: inside the module scope itself.
    let byte = 0;
    (
        enclosing_scope(g, fid, byte).expect("scope"),
        SourceLoc { file: fid, byte },
    )
}

fn path_from(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    file: &str,
    segs: &[&str],
) -> Resolution {
    let (from, at) = from_in(g, fs, file);
    let pol = RustPolicy::new(g, EDITION);
    resolve_path(
        g,
        &RawPath(segs.iter().map(|s| s.to_string()).collect()),
        NS_VALUE,
        &Anchor::crate_root(),
        from,
        NS_TYPE,
        &at,
        &pol,
    )
}

fn bare_from(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    file: &str,
    name: &str,
) -> Resolution {
    let (from, at) = from_in(g, fs, file);
    let pol = RustPolicy::new(g, EDITION);
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

fn chain_files(vis: &str) -> BTreeMap<String, ParsedFile> {
    let c = format!("{vis} fn f() {{}}\npub fn g() {{ f(); }}\n");
    files(&[
        ("src/lib.rs", "mod a;\nmod z;\n"),
        ("src/a.rs", "pub mod b;\nmod d;\nmod x;\n"),
        ("src/a/b.rs", "pub mod c;\n"),
        ("src/a/b/c.rs", &c),
        (
            "src/a/d.rs",
            "use crate::a::b::c::f;\npub fn caller() { f(); }\n",
        ),
        ("src/a/x.rs", "pub fn caller() { crate::a::b::c::f(); }\n"),
        ("src/z.rs", "pub fn caller() { crate::a::b::c::f(); }\n"),
    ])
}

// ── RED rows ────────────────────────────────────────────────────────────────

#[test]
fn p1_restrict_resolves_to_the_named_ancestor() {
    let fs = chain_files("pub(in crate::a)");
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![Some(module(&g, &fs, &["a"]))]);
    // Bare `f` in d.rs through `use crate::a::b::c::f;`.
    assert_item(&bare_from(&g, &fs, "src/a/d.rs", "f"));
}

#[test]
fn p2_bare_call_in_defining_module() {
    let fs = chain_files("pub(in crate::a)");
    let g = populate(&fs);
    assert_item(&bare_from(&g, &fs, "src/a/b/c.rs", "f"));
}

#[test]
fn p3_full_path_inside_restrict_only() {
    let fs = chain_files("pub(in crate::a)");
    let g = populate(&fs);
    let segs = ["a", "b", "c", "f"];
    assert_item(&path_from(&g, &fs, "src/a/x.rs", &segs));
    assert_not_resolved(&path_from(&g, &fs, "src/z.rs", &segs));
}

#[test]
fn p4_super_super_is_the_grandparent() {
    let fs = chain_files("pub(in super::super)");
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![Some(module(&g, &fs, &["a"]))]);
}

#[test]
fn p5_restricted_mod_binding_gets_its_restrict() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub(in crate::a) mod m;\n"),
        ("src/a/m.rs", "pub fn g() {}\n"),
    ]);
    let g = populate(&fs);
    let m = g
        .bindings
        .iter()
        .find(|b| b.name == "m" && b.ns == NS_TYPE)
        .expect("binding m");
    assert_eq!(m.vis.kind, VIS_PUB_IN);
    assert_eq!(m.vis.restrict, Some(module(&g, &fs, &["a"])));
}

#[test]
fn p6_assoc_fn_gets_its_restrict() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\n"),
        (
            "src/a/b.rs",
            "pub struct T;\nimpl T {\n    pub(in crate::a) fn m() {}\n}\n",
        ),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "m"), vec![Some(module(&g, &fs, &["a"]))]);
}

fn glob_files(tail: &str) -> BTreeMap<String, ParsedFile> {
    let b = format!("pub(in crate::a::b) fn f() {{}}\n{tail}");
    files(&[
        ("src/lib.rs", "mod a;\nmod q;\nmod z;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", &b),
        ("src/q.rs", "pub fn f() {}\n"),
        (
            "src/z.rs",
            "use crate::a::b::*;\nuse crate::q::*;\npub fn outside() {}\n",
        ),
        (
            "src/a/b/w.rs",
            "use crate::a::b::*;\nuse crate::q::*;\npub fn inside() {}\n",
        ),
    ])
}

#[test]
fn p7_glob_member_outside_restrict_is_skipped() {
    let fs = glob_files("pub mod w;\n");
    let g = populate(&fs);
    let r = bare_from(&g, &fs, "src/z.rs", "f");
    assert_item(&r);
    // The winner is `q::f` (b's `f` is Hidden outside its restrict): same item id as the
    // binding of `f` in module `q`, not the one in `a::b`.
    let q = module(&g, &fs, &["q"]);
    let q_f = g
        .bindings
        .iter()
        .find(|b| b.scope == q && b.name == "f" && b.ns == NS_VALUE)
        .expect("q::f binding");
    let BindTarget::Resolved(q_target) = &q_f.target else {
        panic!("q::f is not resolved: {q_f:?}")
    };
    assert_eq!(&r.candidates[0].target, q_target, "{r:?}");
    let b = module(&g, &fs, &["a", "b"]);
    let b_f = g
        .bindings
        .iter()
        .find(|x| x.scope == b && x.name == "f")
        .expect("b::f binding");
    assert_ne!(&b_f.target, &BindTarget::Resolved(q_target.clone()));
}

#[test]
fn p8_mounted_module_is_a_module_not_a_directory() {
    let fs = files(&[
        ("src/lib.rs", "#[path = \"z/a.rs\"]\nmod a;\nmod z;\n"),
        ("src/z/a.rs", "#[path = \"impls/x.rs\"]\npub mod m;\n"),
        ("src/z/impls/x.rs", "pub(in crate::a) fn f() {}\n"),
        ("src/z.rs", "pub fn caller() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![Some(module(&g, &fs, &["a"]))]);
}

// ── KEEP rows ───────────────────────────────────────────────────────────────

#[test]
fn p4b_short_forms_keep_their_kinds() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\n"),
        (
            "src/a/b.rs",
            "pub(in super) fn s() {}\npub(in crate) fn c() {}\npub(in self) fn p() {}\n",
        ),
    ]);
    let g = populate(&fs);
    let kind = |n: &str| {
        let b = g
            .bindings
            .iter()
            .find(|b| b.name == n && b.ns == NS_VALUE)
            .unwrap();
        (b.vis.kind, b.vis.restrict)
    };
    assert_eq!(kind("s"), (VIS_PUB_SUPER, None));
    assert_eq!(kind("c"), (VIS_PUB_CRATE, None));
    assert_eq!(kind("p"), (VIS_PRIV, None));
}

#[test]
fn p9_chain_stops_at_the_named_module_not_the_first() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub mod c;\n"),
        ("src/a/b/c.rs", "pub(in crate::a::b) fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![Some(module(&g, &fs, &["a", "b"]))]);
}

#[test]
fn n3_non_ancestor_restrict_is_none() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\nmod other;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub(in crate::other) fn f() {}\n"),
        ("src/other.rs", "pub fn caller() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![None]);
}

#[test]
fn n4_unanchored_restrict_is_none() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub(in a::b) fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![None]);
}

#[test]
fn n5_inside_the_restrict_two_globs_are_refused() {
    let fs = glob_files("pub mod w;\n");
    let g = populate(&fs);
    assert_not_resolved(&bare_from(&g, &fs, "src/a/b/w.rs", "f"));
}

#[test]
fn p10_inside_the_restrict_two_globs_are_ambiguous_not_poisoned() {
    let fs = glob_files("pub mod w;\n");
    let g = populate(&fs);
    let r = bare_from(&g, &fs, "src/a/b/w.rs", "f");
    assert_eq!(r.status, ResStatus::Ambiguous, "{r:?}");
}

#[test]
fn n7_same_named_module_on_another_branch_is_none() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\nmod c;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub fn other() {}\n"),
        ("src/c.rs", "pub mod b;\n"),
        ("src/c/b.rs", "pub(in crate::a::b) fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![None]);
}

#[test]
fn n8_reexport_is_not_a_restrict_target() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\npub use crate::a::b as c;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub(in crate::c) fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![None]);
}

#[test]
fn restrict_longer_than_the_chain_is_none() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub(in crate::a::b::c) fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![None]);
}

#[test]
fn p5b_inline_mod_binding_gets_its_restrict() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "pub(in crate::a) mod m {\n    pub fn g() {}\n}\n",
        ),
    ]);
    let g = populate(&fs);
    let m = g
        .bindings
        .iter()
        .find(|b| b.name == "m" && b.ns == NS_TYPE)
        .expect("binding m");
    assert_eq!(m.vis.restrict, Some(module(&g, &fs, &["a"])));
}

#[test]
fn p11_every_item_kind_records_its_restrict() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            concat!(
                "pub mod b;\n",
                "pub(in crate::a) struct S;\n",
                "pub(in crate::a) enum E { V }\n",
                "pub(in crate::a) trait T {}\n",
                "pub(in crate::a) type A = u8;\n",
                "pub(in crate::a) const C: u8 = 0;\n",
                "pub(in crate::a) static ST: u8 = 0;\n",
                "impl S {\n",
                "    pub(in crate::a) const K: u8 = 0;\n",
                "    pub(in crate::a) fn m() {}\n",
                "}\n",
            ),
        ),
    ]);
    let g = populate(&fs);
    let a = Some(module(&g, &fs, &["a"]));
    for name in ["S", "E", "T", "A", "C", "ST", "V", "K", "m"] {
        assert!(
            g.bindings
                .iter()
                .any(|b| b.name == name && b.vis.kind == VIS_PUB_IN),
            "{name}"
        );
        assert!(
            g.bindings
                .iter()
                .filter(|b| b.name == name && b.vis.kind == VIS_PUB_IN)
                .all(|b| b.vis.restrict == a),
            "{name}"
        );
    }
}

#[test]
fn n9_super_above_the_crate_root_is_none() {
    // The item's module chain is [a, crate root]: two hops would leave the crate.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub(in super::super) fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![None]);
}

#[test]
fn n10_misplaced_path_mount_leaves_the_restrict_none() {
    // rustc loads `src/b.rs` for `#[path = "b.rs"]` in `src/a.rs`; the populator binds the
    // decoy `src/a/b.rs` (it resolves relative to `src/a/`), so the chain cannot be trusted.
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub(in crate::a) fn f() {}\n"),
        ("src/a/b.rs", "pub(in crate::a) fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![None]);
}

#[test]
fn p12_inline_relative_path_mount_keeps_its_restrict() {
    let fs = files(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "pub mod inl {\n    #[path = \"m.rs\"]\n    pub mod m;\n}\n",
        ),
        ("src/a/inl/m.rs", "pub(in crate::a) fn f() {}\n"),
    ]);
    let g = populate(&fs);
    assert_eq!(restricts(&g, "f"), vec![Some(module(&g, &fs, &["a"]))]);
}
