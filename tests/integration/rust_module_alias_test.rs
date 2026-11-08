//! #724: `use p::{self as m}` module aliases bind (type namespace only) and
//! macro-argument `crate`/`self`/`super` prefixes reach the authoritative route.
//!
//! End to end through `CallGraph::build_with_scope_graph_inputs` and
//! `resolve_call_site_full`. RED rows (B1 B2 B2n B3 B4 B7 B9 B10 B12 B13 B15a)
//! fail while `{self as m}` binds the literal path `p::self`, a self import binds
//! the value namespace, and the macro-argument extractor drops the leading
//! keyword; KEEP rows (B5 B6 B8 B14 B15b) pin what must not change. Each row
//! kills a mutant of `mutants/rust-use-spelling.json`.
use prism::{
    ast::ParsedFile,
    call_graph::{CallGraph, CallSite, ScopeGraphBuildInputs},
    languages::Language,
    name_resolution::rust_populator::RustCrateConfig,
    resolution::{DropReason, ResolutionConfidence, ResolutionKind},
};
use std::collections::BTreeMap;

fn build_with(sources: &[(&str, &str)], edition: u16) -> CallGraph {
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
    inputs.cfg = cfg;
    let cg = CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs));
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    cg
}

fn build(sources: &[(&str, &str)]) -> CallGraph {
    build_with(sources, 2021)
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

/// The call is bound: exactly one Exact target in `target_file` with `kind`.
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
    assert!(o.resolved.is_empty(), "{file}:{caller} {callee}: {o:?}");
    assert_eq!(o.drop, Some(DropReason::UnknownName), "{o:?}");
}

/// No in-repo edge at all (any drop reason, or an external target).
fn no_in_repo_edge(cg: &CallGraph, file: &str, caller: &str, callee: &str) {
    let o = cg.resolve_call_site_full(site(cg, file, caller, callee));
    assert!(
        o.resolved
            .iter()
            .all(|r| r.confidence != ResolutionConfidence::Exact),
        "{file}:{caller} {callee}: {o:?}"
    );
}

// ── alias rows (RED) ────────────────────────────────────────────────────────

#[test]
fn b1_crate_path_self_alias_binds_and_named_sibling_is_unchanged() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod c;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub fn f() {}\npub fn g() {}\n"),
        (
            "src/c.rs",
            "use crate::a::b::{self as m, g};\npub fn caller() {\n    m::f();\n    g();\n}\n",
        ),
    ]);
    exact(
        &cg,
        "src/c.rs",
        "caller",
        "m::f",
        "src/a/b.rs",
        ResolutionKind::QualifiedOwner,
    );
    // KEEP half: the ordinary named import beside the alias binds the value.
    exact(
        &cg,
        "src/c.rs",
        "caller",
        "g",
        "src/a/b.rs",
        ResolutionKind::FreeSingle,
    );
}

#[test]
fn b2_relative_super_self_alias_binds() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\nmod c;\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
        (
            "src/a/c.rs",
            "use super::b::{self as m};\npub fn caller() {\n    m::f();\n}\n",
        ),
    ]);
    exact(
        &cg,
        "src/a/c.rs",
        "caller",
        "m::f",
        "src/a/b.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b2n_nested_group_self_alias_binds() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod c;\n"),
        ("src/a.rs", "pub mod b;\npub fn d() {}\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
        (
            "src/c.rs",
            "use crate::a::{b::{self as m}, d};\npub fn caller() {\n    m::f();\n    d();\n}\n",
        ),
    ]);
    exact(
        &cg,
        "src/c.rs",
        "caller",
        "m::f",
        "src/a/b.rs",
        ResolutionKind::QualifiedOwner,
    );
    exact(
        &cg,
        "src/c.rs",
        "caller",
        "d",
        "src/a.rs",
        ResolutionKind::FreeSingle,
    );
}

#[test]
fn b3_child_tests_module_reaches_the_alias_through_super_glob() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod c;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
        (
            "src/c.rs",
            "use crate::a::b::{self as m};\n\
             pub fn top() {}\n\
             mod tests {\n    use super::*;\n    fn t() {\n        m::f();\n    }\n}\n",
        ),
    ]);
    exact(
        &cg,
        "src/c.rs",
        "t",
        "m::f",
        "src/a/b.rs",
        ResolutionKind::QualifiedOwner,
    );
}

// B4: a qualified path starts its lookup at the enclosing MODULE (`RustPolicy::anchor`,
// `AnchorKind::Bare`), so it never sees a block-scope import or module (#768). That is
// inherited for plain aliases (pinned below as wrong); the `{self as m}` form this PR
// admits refuses instead of binding the module-level alias past a block-scope shadow.
fn shadow_fixture(d: &str) -> CallGraph {
    build(&[
        ("src/lib.rs", "mod a;\nmod c;\nmod d;\n"),
        ("src/a.rs", "pub mod b;\n"),
        ("src/a/b.rs", "pub fn f() {}\n"),
        ("src/c.rs", "pub fn f() {}\n"),
        ("src/d.rs", d),
    ])
}

#[test]
fn b4_shadowed_self_alias_refuses() {
    // rustc binds `c::f` (the block `use` shadows). Base dropped the site (the alias was
    // unresolvable); head must not bind the module-level alias's `a/b.rs::f`.
    let cg = shadow_fixture(
        "use crate::a::b::{self as m};\n\
         pub fn caller() {\n    use crate::c::{self as m};\n    m::f();\n}\n",
    );
    refused(&cg, "src/d.rs", "caller", "m::f");
}

#[test]
fn b4_shadow_by_a_block_local_module_refuses() {
    let cg = shadow_fixture(
        "use crate::a::b::{self as m};\n\
         pub fn caller() {\n    mod m {\n        pub fn f() {}\n    }\n    m::f();\n}\n",
    );
    refused(&cg, "src/d.rs", "caller", "m::f");
}

#[test]
fn b4_unshadowed_sibling_still_binds_the_module_alias() {
    // The refusal is per site: another function of the same module has no shadow.
    let cg = shadow_fixture(
        "use crate::a::b::{self as m};\n\
         pub fn caller() {\n    use crate::c::{self as m};\n    m::f();\n}\n\
         pub fn plain() {\n    m::f();\n}\n",
    );
    exact(
        &cg,
        "src/d.rs",
        "plain",
        "m::f",
        "src/a/b.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b4c_a_block_local_self_alias_alone_is_not_seen_by_a_qualified_path() {
    // Control: the block-local alias is the only `m`. The qualified path starts at the
    // module and does not see it, so the engine cannot resolve it and the site refuses
    // (base and head alike; rustc would bind `c::f`).
    let cg = shadow_fixture("pub fn caller() {\n    use crate::c::{self as m};\n    m::f();\n}\n");
    refused(&cg, "src/d.rs", "caller", "m::f");
}

#[test]
fn inherited_wrong_plain_alias_block_shadow_768() {
    // Inherited, not introduced here (#768): with ORDINARY aliases the module-level one wins
    // on base and head alike (rustc binds `c::f`). Pinned so a fix of #768 flips it knowingly.
    let cg = shadow_fixture(
        "use crate::a::b as m;\n\
         pub fn caller() {\n    use crate::c as m;\n    m::f();\n}\n",
    );
    exact(
        &cg,
        "src/d.rs",
        "caller",
        "m::f",
        "src/a/b.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b7_edition_2015_leading_self_is_kept() {
    // `use self::p::{self as m}` in module `a` names `a::p`; stripping the
    // leading `self` would make the path crate-relative and bind the root `p`.
    let cg = build_with(
        &[
            ("src/lib.rs", "mod p;\nmod a;\n"),
            ("src/p.rs", "pub fn f() {}\n"),
            (
                "src/a.rs",
                "mod p;\nuse self::p::{self as m};\npub fn c() {\n    m::f();\n}\n",
            ),
            ("src/a/p.rs", "pub fn f() {}\n"),
        ],
        2015,
    );
    exact(
        &cg,
        "src/a.rs",
        "c",
        "m::f",
        "src/a/p.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b12_self_import_binds_the_type_namespace_only() {
    // `mod p {}` `fn p() {}` `fn m() {}`; a block-local `use crate::p::{self as m}`
    // imports the MODULE as `m` (type namespace); `m()` is still `fn m`.
    let cg = build(&[
        (
            "src/lib.rs",
            "pub mod p;\npub fn p() {}\npub fn m() {}\n\
             pub fn caller() {\n    use crate::p::{self as m};\n    m();\n}\n",
        ),
        ("src/p.rs", "pub fn other() {}\n"),
    ]);
    exact(
        &cg,
        "src/lib.rs",
        "caller",
        "m",
        "src/lib.rs",
        ResolutionKind::LocalDef,
    );
    // `fn p` is in the same file: the target NAME discriminates the two functions.
    let o = cg.resolve_call_site_full(site(&cg, "src/lib.rs", "caller", "m"));
    assert_eq!(o.resolved[0].target.name, "m", "{o:?}");
}

#[test]
fn b13_unaliased_self_import_does_not_conflict_with_a_local_fn() {
    let cg = build(&[
        ("src/lib.rs", "pub mod p;\npub fn p() {}\nmod q;\n"),
        ("src/p.rs", "pub fn x() {}\n"),
        (
            "src/q.rs",
            "use crate::p::{self};\nfn p() {}\npub fn c() {\n    p();\n}\n",
        ),
    ]);
    exact(
        &cg,
        "src/q.rs",
        "c",
        "p",
        "src/q.rs",
        ResolutionKind::LocalDef,
    );
}

// ── alias rows (KEEP) ───────────────────────────────────────────────────────

#[test]
fn b5_two_aliases_of_one_name_bind_nothing() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod c;\nmod d;\n"),
        ("src/a.rs", "pub fn f() {}\n"),
        ("src/c.rs", "pub fn f() {}\n"),
        (
            "src/d.rs",
            "use crate::a::{self as m};\nuse crate::c::{self as m};\npub fn caller() {\n    m::f();\n}\n",
        ),
    ]);
    refused(&cg, "src/d.rs", "caller", "m::f");
}

#[test]
fn b6_alias_beside_a_local_module_of_the_same_name_binds_nothing() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod d;\n"),
        ("src/a.rs", "pub fn f() {}\n"),
        (
            "src/d.rs",
            "use crate::a::{self as m};\nmod m {\n    pub fn f() {}\n}\npub fn caller() {\n    m::f();\n}\n",
        ),
    ]);
    refused(&cg, "src/d.rs", "caller", "m::f");
}

#[test]
fn b8_external_module_alias_makes_no_in_repo_edge() {
    let cg = build(&[
        ("src/lib.rs", "mod x;\nmod y;\n"),
        ("src/x.rs", "pub fn to_string() {}\n"),
        (
            "src/y.rs",
            "use serde::{self as s};\npub fn caller() {\n    s::to_string();\n}\n",
        ),
    ]);
    no_in_repo_edge(&cg, "src/y.rs", "caller", "s::to_string");
}

#[test]
fn b14_ordinary_named_import_still_binds_the_value() {
    let cg = build(&[
        ("src/lib.rs", "mod x;\nmod y;\n"),
        ("src/x.rs", "pub fn f() {}\n"),
        (
            "src/y.rs",
            "use crate::x::f;\npub fn caller() {\n    f();\n}\n",
        ),
    ]);
    exact(
        &cg,
        "src/y.rs",
        "caller",
        "f",
        "src/x.rs",
        ResolutionKind::FreeSingle,
    );
}

// ── macro-argument anchor rows ──────────────────────────────────────────────

#[test]
fn b9_super_super_in_a_macro_binds_through_the_anchored_path() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub mod b;\npub fn f(_x: u8) {}\n"),
        ("src/a/b.rs", "mod c;\n"),
        (
            "src/a/b/c.rs",
            "pub fn t() {\n    assert!(super::super::f(1));\n}\n",
        ),
    ]);
    // The recorded spelling is base's; only the authoritative route reads the anchor.
    exact(
        &cg,
        "src/a/b/c.rs",
        "t",
        "f",
        "src/a.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b9_crate_path_in_a_macro_binds_through_the_anchored_path() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod z;\n"),
        ("src/a.rs", "pub fn f() {}\n"),
        ("src/z.rs", "pub fn t() {\n    assert!(crate::a::f());\n}\n"),
    ]);
    exact(
        &cg,
        "src/z.rs",
        "t",
        "a::f",
        "src/a.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b9_self_path_in_a_macro_binds_through_the_anchored_path() {
    let cg = build(&[
        ("src/lib.rs", "mod m;\n"),
        (
            "src/m.rs",
            "pub fn f() {}\npub fn t() {\n    assert!(self::f());\n}\n",
        ),
    ]);
    // Base resolves the stripped `f` as `LocalDef`; the anchored path is qualified.
    exact(
        &cg,
        "src/m.rs",
        "t",
        "f",
        "src/m.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b10_super_prefix_binds_the_parent_item_not_a_same_named_child() {
    // WRONG on base: the stripped `a::f` is read from the caller's module and
    // binds the CHILD `mod a`; `super::a::f` is the parent's.
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod m;\n"),
        ("src/a.rs", "pub fn f() {}\n"),
        (
            "src/m.rs",
            "mod a;\npub fn t() {\n    assert!(super::a::f());\n}\n",
        ),
        ("src/m/a.rs", "pub fn f() {}\n"),
    ]);
    exact(
        &cg,
        "src/m.rs",
        "t",
        "a::f",
        "src/a.rs",
        ResolutionKind::QualifiedOwner,
    );
}

const B15_FILES: [(&str, &str); 3] = [
    ("src/lib.rs", "mod m;\n"),
    (
        "src/m.rs",
        "pub fn unique_fn() {}\npub fn modeled() {\n    assert!(self::unique_fn());\n}\n",
    ),
    // Not reachable from `lib.rs` through `mod`: an unmodeled file, legacy ladder.
    (
        "src/orphan.rs",
        "pub fn orphan() {\n    assert!(self::unique_fn());\n}\n",
    ),
];

#[test]
fn b15a_modeled_self_prefix_is_anchored() {
    let cg = build(&B15_FILES);
    // RED: base resolves the stripped `unique_fn` as `LocalDef`.
    exact(
        &cg,
        "src/m.rs",
        "modeled",
        "unique_fn",
        "src/m.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b15b_orphan_self_prefix_keeps_the_legacy_outcome() {
    let cg = build(&B15_FILES);
    // KEEP: the legacy route sees base's spelling (`unique_fn`, cross-file FreeSingle);
    // writing the prefix into `callee_name` would send it down the `self::` same-file case.
    exact(
        &cg,
        "src/orphan.rs",
        "orphan",
        "unique_fn",
        "src/m.rs",
        ResolutionKind::FreeSingle,
    );
}

// ── #[path] placement (review fold, #765; fixed at the root) ────────────────
//
// In `src/a.rs`, `#[path = "b.rs"] mod b;` rustc loads `src/b.rs`. The pre-fix populator
// bound the retained decoy `src/a/b.rs` (it resolved relative to `src/a/`), and this PR's
// guards refused such targets; the placement now follows rustc, so the anchored
// macro-argument path and the `{self as m}` alias bind rustc's file.

fn mount_files(call_file: &str) -> Vec<(&str, &str)> {
    vec![
        ("src/lib.rs", "mod a;\nmod c;\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub fn f() -> bool { true }\n"), // what rustc loads
        ("src/a/b.rs", "pub fn f() -> bool { false }\n"), // the decoy
        ("src/c.rs", call_file),
    ]
}

fn mod_rs_mount_files(call_file: &str) -> Vec<(&str, &str)> {
    vec![
        ("src/lib.rs", "mod a;\nmod c;\n"),
        // Directly in a mod-rs file both calculations give `src/a/b.rs`.
        ("src/a/mod.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/a/b.rs", "pub fn f() -> bool { true }\n"),
        ("src/c.rs", call_file),
    ]
}

const MACRO_CALL: &str = "pub fn caller() {\n    assert!(crate::a::b::f());\n}\n";
const ALIAS_CALL: &str = "use crate::a::b::{self as m};\npub fn caller() {\n    m::f();\n}\n";

#[test]
fn b16_anchored_macro_path_binds_rustcs_file_for_a_non_mod_rs_mount() {
    let cg = build(&mount_files(MACRO_CALL));
    exact(
        &cg,
        "src/c.rs",
        "caller",
        "a::b::f",
        "src/b.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b17_alias_binds_rustcs_file_for_a_non_mod_rs_mount() {
    let cg = build(&mount_files(ALIAS_CALL));
    exact(
        &cg,
        "src/c.rs",
        "caller",
        "m::f",
        "src/b.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b16c_anchored_macro_path_binds_a_correctly_placed_mount() {
    let cg = build(&mod_rs_mount_files(MACRO_CALL));
    exact(
        &cg,
        "src/c.rs",
        "caller",
        "a::b::f",
        "src/a/b.rs",
        ResolutionKind::QualifiedOwner,
    );
}

#[test]
fn b17c_alias_binds_a_correctly_placed_mount() {
    let cg = build(&mod_rs_mount_files(ALIAS_CALL));
    exact(
        &cg,
        "src/c.rs",
        "caller",
        "m::f",
        "src/a/b.rs",
        ResolutionKind::QualifiedOwner,
    );
}

// ── W2 / S1 ─────────────────────────────────────────────────────────────────

#[test]
fn b18_self_super_prefix_is_anchored_and_refuses_like_a_source_site() {
    // `self::super::g()` names the parent's `g`; the recorded `g` alone would bind the
    // caller's own `g`. The authoritative route reads the whole prefix, and a Source-spelled
    // `self::super::g()` refuses (its `super` is read as a name), so the macro site does too.
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a.rs", "pub fn g() {}\nmod c;\n"),
        (
            "src/a/c.rs",
            "pub fn g() {}\npub fn macro_site() {\n    assert!(self::super::g());\n}\n\
             pub fn source_site() {\n    self::super::g();\n}\n",
        ),
    ]);
    refused(&cg, "src/a/c.rs", "source_site", "self::super::g");
    refused(&cg, "src/a/c.rs", "macro_site", "g");
}

#[test]
fn s1_anchored_site_is_not_judged_by_its_unanchored_spelling_for_externals() {
    // `extern crate log;` makes the bare spelling `log::L::new` an external path; the
    // written path is `crate::log::L::new`, the in-repo module, and keeps its edge.
    let cg = build(&[
        ("src/lib.rs", "mod log;\nmod z;\n"),
        (
            "src/log.rs",
            "pub struct L;\nimpl L {\n    pub fn new() {}\n}\n",
        ),
        (
            "src/z.rs",
            "extern crate log;\npub fn caller() {\n    assert!(crate::log::L::new());\n}\n",
        ),
    ]);
    exact(
        &cg,
        "src/z.rs",
        "caller",
        "log::L::new",
        "src/log.rs",
        ResolutionKind::QualifiedOwner,
    );
}
