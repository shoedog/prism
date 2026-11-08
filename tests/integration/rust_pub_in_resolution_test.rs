//! #710: callers inside a `pub(in <path>)` restrict bind to the item.
//!
//! End to end through `CallGraph::build_with_scope_graph_inputs` and
//! `resolve_call_site_full`. RED rows (P1-P5, P8, P7) fail while the populator
//! discards the restrict path; KEEP rows (P4b, P6, N1-N8) pin what must not
//! change (and each kills one mutant of `mutants/rust-pub-in-restrict.json`).
use prism::{
    ast::ParsedFile,
    call_graph::{CallGraph, CallSite, ScopeGraphBuildInputs},
    languages::Language,
    name_resolution::rust_populator::RustCrateConfig,
    resolution::{DropReason, ResolutionConfidence, ResolutionKind},
};
use std::collections::BTreeMap;

fn build_with(
    sources: &[(&str, &str)],
    edition: u16,
    members: &[&str],
    deps: BTreeMap<String, BTreeMap<String, String>>,
) -> CallGraph {
    let files = sources
        .iter()
        .map(|(p, s)| {
            (
                (*p).to_string(),
                ParsedFile::parse(p, s, Language::Rust).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    let mut cfg = RustCrateConfig::from_convention(&files);
    cfg.edition = edition;
    cfg.workspace_members = members.iter().map(|m| m.to_string()).collect();
    cfg.member_in_repo_deps = deps;
    inputs.cfg = cfg;
    let cg = CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs));
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    cg
}

fn build(sources: &[(&str, &str)]) -> CallGraph {
    build_with(sources, 2021, &[], BTreeMap::new())
}

fn site<'a>(cg: &'a CallGraph, file: &str, caller: &str, callee: &str) -> &'a CallSite {
    let sites = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.file == file && s.caller.name == caller && s.callee_name == callee)
        .collect::<Vec<_>>();
    assert_eq!(sites.len(), 1, "{file}:{caller} {callee}: {sites:?}");
    sites[0]
}

/// The call is bound: exactly one Exact target `(file, name)` with `kind`.
fn exact(
    cg: &CallGraph,
    file: &str,
    caller: &str,
    callee: &str,
    target_file: &str,
    kind: ResolutionKind,
) {
    let o = cg.resolve_call_site_full(site(cg, file, caller, callee));
    assert_eq!(o.resolved.len(), 1, "{file}:{caller} {callee}: {o:?}");
    let r = &o.resolved[0];
    assert_eq!(
        (r.target.file.as_str(), r.confidence, r.kind),
        (target_file, ResolutionConfidence::Exact, kind),
        "{o:?}"
    );
    assert_eq!(o.drop, None, "{o:?}");
}

/// The call is refused: no target, dropped `UnknownName`.
fn refused(cg: &CallGraph, file: &str, caller: &str, callee: &str) {
    let o = cg.resolve_call_site_full(site(cg, file, caller, callee));
    assert_eq!(
        o.resolved
            .iter()
            .map(|r| (&r.target.file, &r.target.name))
            .collect::<Vec<_>>(),
        vec![],
        "{file}:{caller} {callee}: {o:?}"
    );
    assert_eq!(o.drop, Some(DropReason::UnknownName), "{o:?}");
}

/// Crate shape of P1/P2/P3/N1/N2: `a/b/c.rs` holds the restricted item.
fn chain(item_vis: &str) -> Vec<(String, String)> {
    vec![
        ("src/lib.rs".into(), "mod a;\nmod z;\n".into()),
        ("src/a.rs".into(), "pub mod b;\nmod d;\nmod x;\n".into()),
        ("src/a/b.rs".into(), "pub mod c;\n".into()),
        (
            "src/a/b/c.rs".into(),
            format!("{item_vis} fn f() {{}}\npub fn g() {{\n    f();\n}}\n"),
        ),
        (
            "src/a/d.rs".into(),
            "use crate::a::b::c::f;\npub fn via_use() {\n    f();\n}\n".into(),
        ),
        (
            "src/a/x.rs".into(),
            "pub fn via_path() {\n    crate::a::b::c::f();\n}\n".into(),
        ),
        (
            "src/z.rs".into(),
            "pub fn outside() {\n    crate::a::b::c::f();\n}\n".into(),
        ),
    ]
}

fn build_owned(files: &[(String, String)]) -> CallGraph {
    let refs = files
        .iter()
        .map(|(p, s)| (p.as_str(), s.as_str()))
        .collect::<Vec<_>>();
    build(&refs)
}

// ── RED rows ────────────────────────────────────────────────────────────────

#[test]
fn p1_file_module_restrict_binds() {
    let cg = build_owned(&chain("pub(in crate::a)"));
    exact(
        &cg,
        "src/a/d.rs",
        "via_use",
        "f",
        "src/a/b/c.rs",
        ResolutionKind::FreeSingle,
    );
}

#[test]
fn p2_bare_call_in_defining_module_binds() {
    let cg = build_owned(&chain("pub(in crate::a)"));
    exact(
        &cg,
        "src/a/b/c.rs",
        "g",
        "f",
        "src/a/b/c.rs",
        ResolutionKind::LocalDef,
    );
}

#[test]
fn p3_full_path_inside_restrict_binds() {
    let cg = build_owned(&chain("pub(in crate::a)"));
    exact(
        &cg,
        "src/a/x.rs",
        "via_path",
        "crate::a::b::c::f",
        "src/a/b/c.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn p4_super_super_restrict_binds_from_a_cousin() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod z;\n"),
        ("src/a.rs", "pub mod b;\npub mod e;\n"),
        ("src/a/b.rs", "pub mod c;\n"),
        ("src/a/b/c.rs", "pub(in super::super) fn f() {}\n"),
        ("src/a/e.rs", "pub mod g;\n"),
        (
            "src/a/e/g.rs",
            "pub fn cousin() {\n    crate::a::b::c::f();\n}\n",
        ),
        (
            "src/z.rs",
            "pub fn outside() {\n    crate::a::b::c::f();\n}\n",
        ),
    ]);
    exact(
        &cg,
        "src/a/e/g.rs",
        "cousin",
        "crate::a::b::c::f",
        "src/a/b/c.rs",
        ResolutionKind::QualifiedOwner,
    );
    // `super::super` is `crate::a`, not the crate root: a crate-level caller is refused.
    refused(&cg, "src/z.rs", "outside", "crate::a::b::c::f");
}

#[test]
fn p5_restricted_mod_item_binds_inside_restrict() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod z;\n"),
        ("src/a.rs", "pub(in crate::a) mod m;\nmod d;\n"),
        ("src/a/m.rs", "pub fn g() {}\n"),
        (
            "src/a/d.rs",
            "pub fn inside() {\n    crate::a::m::g();\n}\n",
        ),
        ("src/z.rs", "pub fn outside() {\n    crate::a::m::g();\n}\n"),
    ]);
    exact(
        &cg,
        "src/a/d.rs",
        "inside",
        "crate::a::m::g",
        "src/a/m.rs",
        ResolutionKind::QualifiedOwner,
    );
    refused(&cg, "src/z.rs", "outside", "crate::a::m::g");
}

fn glob_crate(tail: &str) -> Vec<(&'static str, String)> {
    vec![
        ("src/lib.rs", "mod a;\nmod q;\nmod z;\n".into()),
        ("src/a.rs", "pub mod b;\n".into()),
        (
            "src/a/b.rs",
            format!("pub(in crate::a::b) fn f() {{}}\n{tail}"),
        ),
        ("src/q.rs", "pub fn f() {}\n".into()),
        (
            "src/z.rs",
            "use crate::a::b::*;\nuse crate::q::*;\npub fn outside() {\n    f();\n}\n".into(),
        ),
    ]
}

#[test]
fn p7_glob_member_outside_restrict_is_skipped() {
    let files = glob_crate("");
    let refs = files
        .iter()
        .map(|(p, s)| (*p, s.as_str()))
        .collect::<Vec<_>>();
    let cg = build(&refs);
    exact(
        &cg,
        "src/z.rs",
        "outside",
        "f",
        "src/q.rs",
        ResolutionKind::FreeSingle,
    );
}

#[test]
fn p8_mounted_file_module_identity_decides() {
    // `src/z/a.rs` is mounted as `crate::a` and mounts `impls/x.rs` (physically
    // under `src/z/`, the directory of module `z`): the restrict is module
    // identity, never the directory.
    let cg = build(&[
        ("src/lib.rs", "#[path = \"z/a.rs\"]\nmod a;\nmod z;\n"),
        (
            "src/z/a.rs",
            "#[path = \"impls/x.rs\"]\npub mod m;\nmod d;\n",
        ),
        ("src/z/impls/x.rs", "pub(in crate::a) fn f() {}\n"),
        (
            "src/z/d.rs",
            "pub fn inside() {\n    crate::a::m::f();\n}\n",
        ),
        ("src/z.rs", "pub fn outside() {\n    crate::a::m::f();\n}\n"),
    ]);
    exact(
        &cg,
        "src/z/d.rs",
        "inside",
        "crate::a::m::f",
        "src/z/impls/x.rs",
        ResolutionKind::QualifiedOwner,
    );
    // N8b (KEEP): `crate::z` owns the directory but is outside the restrict.
    refused(&cg, "src/z.rs", "outside", "crate::a::m::f");
}

// ── KEEP rows ───────────────────────────────────────────────────────────────

#[test]
fn p4b_in_super_crate_self_behave_as_their_short_forms() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod z;\n"),
        ("src/a.rs", "pub mod b;\nmod d;\n"),
        (
            "src/a/b.rs",
            "pub(in super) fn s() {}\npub(in crate) fn c() {}\npub(in self) fn p() {}\npub fn own() {\n    p();\n}\n",
        ),
        (
            "src/a/d.rs",
            "pub fn sibling() {\n    crate::a::b::s();\n    crate::a::b::p();\n}\n",
        ),
        (
            "src/z.rs",
            "pub fn outside() {\n    crate::a::b::s();\n    crate::a::b::c();\n}\n",
        ),
    ]);
    let o = |file, caller, callee| cg.resolve_call_site_full(site(&cg, file, caller, callee));
    // pub(in super) == pub(super): visible in the parent's subtree, not outside it.
    assert_eq!(
        o("src/a/d.rs", "sibling", "crate::a::b::s").resolved.len(),
        1
    );
    assert!(o("src/z.rs", "outside", "crate::a::b::s")
        .resolved
        .is_empty());
    // pub(in crate) == pub(crate).
    assert_eq!(o("src/z.rs", "outside", "crate::a::b::c").resolved.len(), 1);
    // pub(in self) == private: visible in its own module, not from a sibling.
    assert_eq!(o("src/a/b.rs", "own", "p").resolved.len(), 1);
    assert!(o("src/a/d.rs", "sibling", "crate::a::b::p")
        .resolved
        .is_empty());
}

#[test]
fn p6_assoc_fn_keeps_its_outcome() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod z;\n"),
        ("src/a.rs", "pub mod b;\nmod d;\n"),
        (
            "src/a/b.rs",
            "pub struct T;\nimpl T {\n    pub(in crate::a) fn m() {}\n}\n",
        ),
        (
            "src/a/d.rs",
            "use crate::a::b::T;\npub fn inside() {\n    T::m();\n}\n",
        ),
    ]);
    let o = cg.resolve_call_site_full(site(&cg, "src/a/d.rs", "inside", "T::m"));
    assert_eq!(o.resolved.len(), 1, "{o:?}");
    assert_eq!(o.resolved[0].target.file, "src/a/b.rs");
    assert_eq!(
        (o.resolved[0].confidence, o.resolved[0].kind),
        (ResolutionConfidence::Exact, ResolutionKind::QualifiedOwner),
        "{o:?}"
    );
}

fn n1_crate() -> CallGraph {
    // Restrict `crate::a::b`, item in `crate::a::b::c`.
    build(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "pub mod b;\npub fn parent() {\n    crate::a::b::c::f();\n}\n",
        ),
        (
            "src/a/b.rs",
            "pub mod c;\npub fn inside() {\n    crate::a::b::c::f();\n}\n",
        ),
        ("src/a/b/c.rs", "pub(in crate::a::b) fn f() {}\n"),
    ])
}

#[test]
fn n1_parent_of_the_restrict_is_refused() {
    refused(&n1_crate(), "src/a.rs", "parent", "crate::a::b::c::f");
}

#[test]
fn p9_restrict_stops_at_the_named_module_not_the_first_below_the_anchor() {
    exact(
        &n1_crate(),
        "src/a/b.rs",
        "inside",
        "crate::a::b::c::f",
        "src/a/b/c.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn n2_sibling_crate_module_by_full_path_is_refused() {
    let cg = build_owned(&chain("pub(in crate::a)"));
    refused(&cg, "src/z.rs", "outside", "crate::a::b::c::f");
}

#[test]
fn n3_restrict_that_is_not_an_ancestor_is_refused_even_from_it() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod other;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub(in crate::other) fn f() {}\n"),
        (
            "src/other.rs",
            "pub fn caller() {\n    crate::a::b::f();\n}\n",
        ),
    ]);
    refused(&cg, "src/other.rs", "caller", "crate::a::b::f");
}

#[test]
fn n4_unanchored_restrict_is_refused() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\n"),
        (
            "src/a/b.rs",
            "pub(in a::b) fn f() {}\npub fn caller() {\n    f();\n}\n",
        ),
    ]);
    refused(&cg, "src/a/b.rs", "caller", "f");
}

#[test]
fn n5_inside_the_restrict_two_globs_stay_ambiguous() {
    let mut files = glob_crate("pub mod w;\n");
    files.push((
        "src/a/b/w.rs",
        "use crate::a::b::*;\nuse crate::q::*;\npub fn inside() {\n    f();\n}\n".into(),
    ));
    let refs = files
        .iter()
        .map(|(p, s)| (*p, s.as_str()))
        .collect::<Vec<_>>();
    let cg = build(&refs);
    let o = cg.resolve_call_site_full(site(&cg, "src/a/b/w.rs", "inside", "f"));
    assert!(o.resolved.is_empty(), "{o:?}");
    assert!(o.drop.is_some(), "{o:?}");
}

#[test]
fn n6_workspace_crate_roots_differ() {
    let deps = BTreeMap::from([(
        "crates/a".to_string(),
        BTreeMap::from([("b".to_string(), "crates/b".to_string())]),
    )]);
    let cg = build_with(
        &[
            ("crates/a/src/lib.rs", "pub mod x;\n"),
            (
                "crates/a/src/x.rs",
                "pub fn caller() {\n    b::x::f();\n}\n",
            ),
            ("crates/b/src/lib.rs", "pub mod x;\n"),
            ("crates/b/src/x.rs", "pub(in crate::x) fn f() {}\n"),
        ],
        2021,
        &["crates/a", "crates/b"],
        deps,
    );
    refused(&cg, "crates/a/src/x.rs", "caller", "b::x::f");
}

#[test]
fn n7_same_named_module_on_another_branch_is_not_the_restrict() {
    // `crate::a::b` exists, but the item lives in `crate::c::b`.
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod c;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub fn other() {}\n"),
        ("src/c.rs", "pub mod b;\n"),
        (
            "src/c/b.rs",
            "pub(in crate::a::b) fn f() {}\npub fn caller() {\n    f();\n}\n",
        ),
    ]);
    refused(&cg, "src/c/b.rs", "caller", "f");
}

#[test]
fn n8_restrict_through_a_reexport_is_refused() {
    // `crate::c` is a re-export of `crate::a::b`, not a module on b's ancestor chain.
    let cg = build(&[
        ("src/lib.rs", "mod a;\npub use crate::a::b as c;\n"),
        ("src/a.rs", "pub mod b;\npub mod x;\n"),
        (
            "src/a/b.rs",
            "pub(in crate::c) fn f() {}\npub fn inside() {\n    f();\n}\n",
        ),
        (
            "src/a/x.rs",
            "pub fn sibling() {\n    crate::a::b::f();\n}\n",
        ),
    ]);
    refused(&cg, "src/a/b.rs", "inside", "f");
    refused(&cg, "src/a/x.rs", "sibling", "crate::a::b::f");
}
