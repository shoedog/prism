//! #765: a call through a `#[path]`-mounted module binds the file rustc loads.
//!
//! End to end through `CallGraph::build_with_scope_graph_inputs` and
//! `resolve_call_site_full`, one row per layout of the Rust Reference's `path`
//! attribute rule (see `tests/name_resolution/path_attribute_placement_test.rs` for the
//! rule and the graph-level rows). Every mount has a retained DECOY at the location the
//! pre-fix populator picked (the declaring file's `<stem>/` directory), with the
//! function on a different line than rustc's file, so a row asserts the binding by
//! function identity `(file, name, start_line)`, not by name.
//!
//! RED rows (E1 E1b E1c E4 E7b E8 E9 E12 and X1-X5) fail on the pre-fix populator (E9:
//! it bound the decoy `src/a/gone.rs` for a mount of a file that does not exist); KEEP
//! rows (E2 E3 E5 E6 E7 E9b E10 E11 E13) pin layouts that already agreed with rustc or
//! refuse. Each row kills a mutant of `mutants/rust-path-placement.json` or pins a
//! refusal.
use prism::{
    ast::ParsedFile,
    call_graph::{CallGraph, CallSite, ScopeGraphBuildInputs},
    languages::Language,
    name_resolution::rust_populator::RustCrateConfig,
    repo_loader::load_repo,
    resolution::ResolutionConfidence,
};
use std::collections::BTreeMap;
use tempfile::TempDir;

fn build(sources: &[(&str, &str)]) -> CallGraph {
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
    cfg.edition = 2021;
    inputs.cfg = cfg;
    let cg = CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs));
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    cg
}

/// Manifest-parsed fixture through `load_repo` (the production route).
fn build_repo(sources: &[(&str, &str)]) -> CallGraph {
    let dir = TempDir::new().unwrap();
    for (path, src) in sources {
        let abs = dir.path().join(path);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(abs, src).unwrap();
    }
    let repo = load_repo(dir.path()).unwrap();
    let cg =
        CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref());
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    cg
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

/// The call binds exactly one `Exact` target: function `name` at `start_line` of `target_file`.
fn binds(
    cg: &CallGraph,
    (file, caller, callee): (&str, &str, &str),
    (target_file, name, start_line): (&str, &str, usize),
) {
    let o = cg.resolve_call_site_full(site(cg, file, caller, callee));
    assert_eq!(o.resolved.len(), 1, "{file}:{caller} {callee}: {o:?}");
    let r = &o.resolved[0];
    assert_eq!(
        (
            r.target.file.as_str(),
            r.target.name.as_str(),
            r.target.start_line,
            r.confidence
        ),
        (target_file, name, start_line, ResolutionConfidence::Exact),
        "{o:?}"
    );
    assert_eq!(o.drop, None, "{o:?}");
}

/// The call has no in-repo target at all.
fn unbound(cg: &CallGraph, file: &str, caller: &str, callee: &str) {
    let o = cg.resolve_call_site_full(site(cg, file, caller, callee));
    assert!(o.resolved.is_empty(), "{file}:{caller} {callee}: {o:?}");
}

/// What rustc loads: the function sits on line 2.
const REAL: &str = "// rustc loads this file\npub fn f() -> bool {\n    true\n}\n";
/// The decoy at the pre-fix location: the function sits on line 1.
const DECOY: &str = "pub fn f() -> bool {\n    false\n}\n";
const REAL_AT: usize = 2;

/// `caller` in `src/c.rs` calling `path`, in a crate whose `lib.rs` is `lib`.
fn with_caller<'a>(
    lib: &'a str,
    mut files: Vec<(&'a str, &'a str)>,
    call: &'a str,
) -> Vec<(&'a str, &'a str)> {
    files.insert(0, ("src/lib.rs", lib));
    files.push(("src/c.rs", call));
    files
}

const CALL_ABF: &str = "pub fn caller() {\n    crate::a::b::f();\n}\n";

// ── RED rows ────────────────────────────────────────────────────────────────

#[test]
fn e1_non_mod_rs_declaring_file_binds_the_file_beside_it() {
    let cg = build(&with_caller(
        "mod a;\nmod c;\n",
        vec![
            ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
            ("src/b.rs", REAL),
            ("src/a/b.rs", DECOY),
        ],
        CALL_ABF,
    ));
    binds(
        &cg,
        ("src/c.rs", "caller", "crate::a::b::f"),
        ("src/b.rs", "f", REAL_AT),
    );
}

#[test]
fn e1b_import_through_the_mount_binds_rustcs_file() {
    let cg = build(&with_caller(
        "mod a;\nmod c;\n",
        vec![
            ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
            ("src/b.rs", REAL),
            ("src/a/b.rs", DECOY),
        ],
        "use crate::a::b::f;\npub fn caller() {\n    f();\n}\n",
    ));
    binds(&cg, ("src/c.rs", "caller", "f"), ("src/b.rs", "f", REAL_AT));
}

#[test]
fn e1c_call_from_the_declaring_file_itself() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "#[path = \"b.rs\"]\npub mod b;\npub fn caller() {\n    b::f();\n}\n",
        ),
        ("src/b.rs", REAL),
        ("src/a/b.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/a.rs", "caller", "b::f"),
        ("src/b.rs", "f", REAL_AT),
    );
}

#[test]
fn e3_crate_root_mount() {
    let cg = build(&[
        (
            "src/lib.rs",
            "#[path = \"b.rs\"]\npub mod b;\npub fn caller() {\n    b::f();\n}\n",
        ),
        ("src/b.rs", REAL),
        ("src/lib/b.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/lib.rs", "caller", "b::f"),
        ("src/b.rs", "f", REAL_AT),
    );
}

#[test]
fn e4_relative_subdirectory_from_a_non_mod_rs_file() {
    let cg = build(&with_caller(
        "mod a;\nmod c;\n",
        vec![
            ("src/a.rs", "#[path = \"impls/x.rs\"]\npub mod x;\n"),
            ("src/impls/x.rs", REAL),
            ("src/a/impls/x.rs", DECOY),
        ],
        "pub fn caller() {\n    crate::a::x::f();\n}\n",
    ));
    binds(
        &cg,
        ("src/c.rs", "caller", "crate::a::x::f"),
        ("src/impls/x.rs", "f", REAL_AT),
    );
}

#[test]
fn e7b_mount_in_a_nested_non_mod_rs_file() {
    let cg = build(&with_caller(
        "mod a;\nmod c;\n",
        vec![
            ("src/a.rs", "pub mod c;\n"),
            ("src/a/c.rs", "#[path = \"d.rs\"]\npub mod d;\n"),
            ("src/a/d.rs", REAL),
            ("src/a/c/d.rs", DECOY),
        ],
        "pub fn caller() {\n    crate::a::c::d::f();\n}\n",
    ));
    binds(
        &cg,
        ("src/c.rs", "caller", "crate::a::c::d::f"),
        ("src/a/d.rs", "f", REAL_AT),
    );
}

#[test]
fn e8_parent_directory_path_binds_the_shared_file() {
    let cg = build(&[
        (
            "src/lib.rs",
            "#[path = \"../shared/s.rs\"]\npub mod s;\npub fn caller() {\n    s::f();\n}\n",
        ),
        ("shared/s.rs", REAL),
        ("src/shared/s.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/lib.rs", "caller", "s::f"),
        ("shared/s.rs", "f", REAL_AT),
    );
}

#[test]
fn e12_inline_module_with_its_own_path_attribute() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "#[path = \"pp\"]\npub mod inner {\n    pub mod y;\n    #[path = \"z.rs\"]\n    pub mod z;\n}\n\
             pub fn caller() {\n    inner::y::f();\n    inner::z::f();\n}\n",
        ),
        ("src/pp/y.rs", REAL),
        ("src/pp/z.rs", REAL),
        ("src/a/pp/y.rs", DECOY),
        ("src/a/pp/z.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/a.rs", "caller", "inner::y::f"),
        ("src/pp/y.rs", "f", REAL_AT),
    );
    binds(
        &cg,
        ("src/a.rs", "caller", "inner::z::f"),
        ("src/pp/z.rs", "f", REAL_AT),
    );
}

// ── KEEP rows ───────────────────────────────────────────────────────────────

#[test]
fn e2_mod_rs_declaring_file() {
    let cg = build(&with_caller(
        "mod a;\nmod c;\n",
        vec![
            ("src/a/mod.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
            ("src/a/b.rs", REAL),
            ("src/b.rs", DECOY),
        ],
        CALL_ABF,
    ));
    binds(
        &cg,
        ("src/c.rs", "caller", "crate::a::b::f"),
        ("src/a/b.rs", "f", REAL_AT),
    );
}

#[test]
fn e5_inline_module_in_a_non_mod_rs_file() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "pub mod inner {\n    #[path = \"z.rs\"]\n    pub mod z;\n}\n\
             pub fn caller() {\n    inner::z::f();\n}\n",
        ),
        ("src/a/inner/z.rs", REAL),
        ("src/inner/z.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/a.rs", "caller", "inner::z::f"),
        ("src/a/inner/z.rs", "f", REAL_AT),
    );
}

#[test]
fn e6_inline_module_in_a_mod_rs_file() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a/mod.rs",
            "mod inner {\n    #[path = \"z.rs\"]\n    pub mod z;\n}\n\
             pub fn caller() {\n    inner::z::f();\n}\n",
        ),
        ("src/a/inner/z.rs", REAL),
        ("src/a/a/inner/z.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/a/mod.rs", "caller", "inner::z::f"),
        ("src/a/inner/z.rs", "f", REAL_AT),
    );
}

#[test]
fn e7_children_of_a_mounted_file_are_its_siblings() {
    let cg = build(&[
        (
            "src/lib.rs",
            "#[path = \"x/foo.rs\"]\npub mod foo;\npub fn caller() {\n    foo::c::f();\n    foo::bar::f();\n    foo::i::q::f();\n}\n",
        ),
        (
            "src/x/foo.rs",
            "pub mod c;\n#[path = \"bar.rs\"]\npub mod bar;\npub mod i {\n    #[path = \"q.rs\"]\n    pub mod q;\n}\n",
        ),
        ("src/x/c.rs", REAL),
        ("src/x/bar.rs", REAL),
        ("src/x/i/q.rs", REAL),
        ("src/x/foo/c.rs", DECOY),
        ("src/x/foo/bar.rs", DECOY),
        ("src/x/foo/i/q.rs", DECOY),
    ]);
    for (callee, file) in [
        ("foo::c::f", "src/x/c.rs"),
        ("foo::bar::f", "src/x/bar.rs"),
        ("foo::i::q::f", "src/x/i/q.rs"),
    ] {
        binds(&cg, ("src/lib.rs", "caller", callee), (file, "f", REAL_AT));
    }
}

#[test]
fn e9_missing_mount_file_binds_nothing() {
    // `gone.rs` is nowhere; the decoys at the stem directory and one level up must not bind.
    let cg = build(&with_caller(
        "mod a;\nmod c;\n",
        vec![
            ("src/a.rs", "#[path = \"gone.rs\"]\npub mod b;\n"),
            ("src/a/gone.rs", DECOY),
            ("src/a/b.rs", DECOY),
        ],
        CALL_ABF,
    ));
    unbound(&cg, "src/c.rs", "caller", "crate::a::b::f");
}

#[test]
fn e9b_absolute_path_binds_nothing() {
    let cg = build(&[
        (
            "src/lib.rs",
            "#[path = \"/etc/b.rs\"]\npub mod abs;\npub fn caller() {\n    abs::f();\n}\n",
        ),
        ("etc/b.rs", DECOY),
        ("src/etc/b.rs", DECOY),
    ]);
    unbound(&cg, "src/lib.rs", "caller", "abs::f");
}

#[test]
fn e10_plain_mod_in_a_non_mod_rs_file_uses_the_stem_directory() {
    let cg = build(&with_caller(
        "mod a;\nmod c;\n",
        vec![
            ("src/a.rs", "pub mod b;\n"),
            ("src/a/b.rs", REAL),
            ("src/b.rs", DECOY),
        ],
        CALL_ABF,
    ));
    binds(
        &cg,
        ("src/c.rs", "caller", "crate::a::b::f"),
        ("src/a/b.rs", "f", REAL_AT),
    );
}

#[test]
fn e11_cfg_attr_path_is_not_read() {
    // Inherited gap, pinned: `#[cfg_attr(.., path = "alt.rs")]` is not modeled, so the
    // default location `b.rs` binds (rustc would load `alt.rs` when the condition holds).
    let cg = build(&with_caller(
        "mod a;\nmod c;\n",
        vec![
            (
                "src/a.rs",
                "#[cfg_attr(all(), path = \"alt.rs\")]\npub mod b;\n",
            ),
            ("src/alt.rs", REAL),
            ("src/a/b.rs", DECOY),
        ],
        CALL_ABF,
    ));
    binds(
        &cg,
        ("src/c.rs", "caller", "crate::a::b::f"),
        ("src/a/b.rs", "f", 1),
    );
}

#[test]
fn e13_two_inline_levels_in_a_non_mod_rs_file() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        (
            "src/a.rs",
            "pub mod i {\n    pub mod j {\n        #[path = \"q.rs\"]\n        pub mod q;\n    }\n}\n\
             pub fn caller() {\n    i::j::q::f();\n}\n",
        ),
        ("src/a/i/j/q.rs", REAL),
        ("src/i/j/q.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/a.rs", "caller", "i::j::q::f"),
        ("src/a/i/j/q.rs", "f", REAL_AT),
    );
}

// ── Review folds (#765 PR D) ────────────────────────────────────────────────

#[test]
fn o1_double_loaded_file_binds_through_both_occurrences() {
    // sol W2, both declaration orders: `crate::b::f()` (plain) and `crate::a::b::f()` (mount).
    for lib in [
        "mod a;\nmod b;\npub fn g() {\n    crate::b::f();\n    crate::a::b::f();\n}\n",
        "mod b;\nmod a;\npub fn g() {\n    crate::b::f();\n    crate::a::b::f();\n}\n",
    ] {
        let cg = build(&[
            ("src/lib.rs", lib),
            ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
            ("src/b.rs", REAL),
        ]);
        for callee in ["crate::b::f", "crate::a::b::f"] {
            binds(&cg, ("src/lib.rs", "g", callee), ("src/b.rs", "f", REAL_AT));
        }
    }
}

#[test]
fn o2_each_occurrence_of_a_shared_file_has_its_own_children() {
    let cg = build(&[
        (
            "src/lib.rs",
            "mod a;\nmod b;\npub fn g() {\n    crate::a::b::child::h();\n    crate::b::child::h();\n}\n",
        ),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        ("src/b.rs", "pub mod child;\n"),
        ("src/child.rs", "pub fn h() {}\n"),
        ("src/b/child.rs", "pub fn h() {}\n"),
    ]);
    binds(
        &cg,
        ("src/lib.rs", "g", "crate::a::b::child::h"),
        ("src/child.rs", "h", 1),
    );
    binds(
        &cg,
        ("src/lib.rs", "g", "crate::b::child::h"),
        ("src/b/child.rs", "h", 1),
    );
}

#[test]
fn o3_a_site_inside_a_double_loaded_file_keeps_only_its_own_file() {
    // `super::g()` means `crate` in the plain occurrence and `a` in the mounted one: refused.
    // A call to a fn of the same file does not depend on the occurrence: still binds.
    let cg = build(&[
        ("src/lib.rs", "mod a;\nmod b;\npub fn g() {}\n"),
        ("src/a.rs", "#[path = \"b.rs\"]\npub mod b;\n"),
        (
            "src/b.rs",
            "pub fn own() {}\npub fn f() {\n    super::g();\n    own();\n}\n",
        ),
    ]);
    unbound(&cg, "src/b.rs", "f", "super::g");
    binds(&cg, ("src/b.rs", "f", "own"), ("src/b.rs", "own", 1));
}

#[test]
fn o5_a_tests_directory_below_src_holds_modules_not_roots() {
    // `src/m/tests/t.rs` is the module `m::tests::t` only (Cargo names `tests/<name>.rs` of
    // a package, not of `src/`): one occurrence, so an in-file site keeps its cross-file target.
    let cg = build(&[
        ("src/lib.rs", "mod m;\n"),
        ("src/m.rs", "pub fn g() {}\nmod tests;\n"),
        ("src/m/tests/mod.rs", "mod t;\n"),
        (
            "src/m/tests/t.rs",
            "use super::super::*;\npub fn run() {\n    g();\n}\n",
        ),
    ]);
    binds(&cg, ("src/m/tests/t.rs", "run", "g"), ("src/m.rs", "g", 1));
}

#[test]
fn o4_a_path_cycle_builds_without_looping_and_its_shell_refuses() {
    // `a.rs` mounts `b.rs`, `b.rs` mounts `a.rs`: the second `a` is a cyclic shell. A path
    // through it binds nothing, while the first occurrences bind.
    let cg = build(&[
        (
            "src/lib.rs",
            "mod a;\npub fn g() {\n    crate::a::b::f();\n    crate::a::b::a::h();\n    crate::a::h();\n}\n",
        ),
        (
            "src/a.rs",
            "#[path = \"b.rs\"]\npub mod b;\npub fn h() {}\n",
        ),
        ("src/b.rs", "#[path = \"a.rs\"]\npub mod a;\npub fn f() {}\n"),
    ]);
    binds(
        &cg,
        ("src/lib.rs", "g", "crate::a::b::f"),
        ("src/b.rs", "f", 3),
    );
    binds(
        &cg,
        ("src/lib.rs", "g", "crate::a::h"),
        ("src/a.rs", "h", 3),
    );
    unbound(&cg, "src/lib.rs", "g", "crate::a::b::a::h");
}

#[test]
fn o6_a_file_that_is_a_real_test_root_and_a_mounted_module_is_two_occurrences() {
    // R2-W2: `tests/probe.rs` is Cargo's integration-test crate AND the library's module
    // `probe`. `crate::helper()` means `tests/probe.rs:1` in one and `src/lib.rs` in the
    // other: the in-file site refuses; both externally addressed paths bind by identity.
    let cg = build(&[
        (
            "src/lib.rs",
            "#[path = \"../tests/probe.rs\"]\npub mod probe;\npub fn helper() -> u8 { 1 }\npub fn g() {\n    crate::probe::probe();\n    crate::probe::helper();\n}\n",
        ),
        (
            "tests/probe.rs",
            "pub fn helper() -> u8 { 2 }\npub fn probe() -> u8 { crate::helper() }\n",
        ),
    ]);
    unbound(&cg, "tests/probe.rs", "probe", "crate::helper");
    binds(
        &cg,
        ("src/lib.rs", "g", "crate::probe::probe"),
        ("tests/probe.rs", "probe", 2),
    );
    binds(
        &cg,
        ("src/lib.rs", "g", "crate::probe::helper"),
        ("tests/probe.rs", "helper", 1),
    );
}

#[test]
fn w1_inline_module_inside_a_fn_body_resolves_from_the_files_own_directory() {
    // Opus W1: `src/a/b.rs` is non-mod-rs, but inside a block rustc drops the pending
    // `<stem>/`: `mod i { #[path = "x.rs"] mod m; }` in a fn body is `src/a/i/x.rs`.
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a/mod.rs", "mod b;\n"),
        (
            "src/a/b.rs",
            "pub fn caller() {\n    mod i {\n        #[path = \"x.rs\"]\n        pub mod m;\n        pub fn h() {\n            m::f();\n        }\n    }\n    i::h();\n}\n",
        ),
        ("src/a/i/x.rs", REAL),
        ("src/a/b/i/x.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/a/b.rs", "h", "m::f"),
        ("src/a/i/x.rs", "f", REAL_AT),
    );
}

#[test]
fn w1_keep_a_file_level_inline_module_in_a_non_mod_rs_file_keeps_the_stem() {
    let cg = build(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/a/mod.rs", "mod b;\n"),
        (
            "src/a/b.rs",
            "mod i {\n    #[path = \"x.rs\"]\n    pub mod m;\n    pub fn h() {\n        m::f();\n    }\n}\n",
        ),
        ("src/a/b/i/x.rs", REAL),
        ("src/a/i/x.rs", DECOY),
    ]);
    binds(
        &cg,
        ("src/a/b.rs", "h", "m::f"),
        ("src/a/b/i/x.rs", "f", REAL_AT),
    );
}

#[test]
fn w3_a_mount_inside_an_unplaceable_inline_module_binds_nothing() {
    let cg = build(&[
        (
            "src/lib.rs",
            "#[path = \"/nonexistent-dir\"]\npub mod i {\n    #[path = \"../x.rs\"]\n    pub mod m;\n    pub fn h() {\n        m::f();\n    }\n}\n",
        ),
        ("x.rs", DECOY),
        ("src/x.rs", DECOY),
    ]);
    unbound(&cg, "src/lib.rs", "h", "m::f");
}

// ── The decoy routes three review rounds found behind guards (PR A R1, PR B W1, ──
// ── PR C W2 and the PR C Opus review R1-R3): each binds rustc's file by identity ──

const WS: &str = "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\n";
fn manifest(name: &str, deps: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{deps}")
}

/// Crate `a`: `p.rs` mounts `q.rs`; rustc loads `a/src/q.rs`, the decoy is `a/src/p/q.rs`.
/// The struct and its method sit on different lines in the two files.
const Q_REAL: &str =
    "// rustc loads this file\npub struct S;\nimpl S {\n    pub fn run(&self) {}\n}\n\
                      pub fn make() -> S {\n    S\n}\npub fn f() -> bool {\n    true\n}\n";
const Q_DECOY: &str = "pub struct S;\nimpl S {\n    pub fn run(&self) {}\n}\n\
                       pub fn make() -> S {\n    S\n}\npub fn f() -> bool {\n    false\n}\n";

fn crate_a(extra: &[(&'static str, &'static str)]) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = vec![
        ("Cargo.toml".into(), WS.into()),
        ("a/Cargo.toml".into(), manifest("a", "")),
        ("a/src/lib.rs".into(), "pub mod p;\n".into()),
        (
            "a/src/p.rs".into(),
            "#[path = \"q.rs\"]\npub mod q;\n".into(),
        ),
        ("a/src/q.rs".into(), Q_REAL.into()),
        ("a/src/p/q.rs".into(), Q_DECOY.into()),
        ("b/Cargo.toml".into(), manifest("b", "")),
        ("c/Cargo.toml".into(), manifest("c", "")),
    ];
    for (p, s) in extra {
        files.retain(|(q, _)| q != p);
        files.push(((*p).into(), (*s).into()));
    }
    files
}

fn repo(files: &[(String, String)]) -> CallGraph {
    let refs = files
        .iter()
        .map(|(p, s)| (p.as_str(), s.as_str()))
        .collect::<Vec<_>>();
    build_repo(&refs)
}

const B_DEP_A: &str = "[dependencies]\na = { path = \"../a\" }\n";

#[test]
fn x1_library_consumer_binds_rustcs_file() {
    let mut files = crate_a(&[("b/src/lib.rs", "pub fn lib_fn() {\n    a::p::q::f();\n}\n")]);
    files.retain(|(p, _)| p != "b/Cargo.toml");
    files.push(("b/Cargo.toml".into(), manifest("b", B_DEP_A)));
    let cg = repo(&files);
    binds(
        &cg,
        ("b/src/lib.rs", "lib_fn", "a::p::q::f"),
        ("a/src/q.rs", "f", 9),
    );
}

#[test]
fn x2_typed_parameter_receiver_from_the_packages_binary() {
    // R1: `fn go(s: a::p::q::S) { s.run(); }` from a binary of package `a`.
    let cg = repo(&crate_a(&[(
        "a/src/main.rs",
        "fn go(s: a::p::q::S) {\n    s.run();\n}\nfn main() {}\n",
    )]));
    binds(
        &cg,
        ("a/src/main.rs", "go", "run"),
        ("a/src/q.rs", "run", 4),
    );
}

#[test]
fn x3_return_typed_receiver_from_the_packages_binary() {
    // R2: `let s = a::p::q::make(); s.run();`.
    let cg = repo(&crate_a(&[(
        "a/src/main.rs",
        "fn go() {\n    let s = a::p::q::make();\n    s.run();\n}\nfn main() {}\n",
    )]));
    binds(
        &cg,
        ("a/src/main.rs", "go", "run"),
        ("a/src/q.rs", "run", 4),
    );
}

#[test]
fn x4_reexport_through_the_mounted_module() {
    // `pub use crate::p::q;` re-exports the mounted module; `a::q::f()` binds rustc's file.
    let mut files = crate_a(&[("a/src/lib.rs", "pub mod p;\npub use crate::p::q;\n")]);
    files.retain(|(p, _)| p != "b/Cargo.toml");
    files.push(("b/Cargo.toml".into(), manifest("b", B_DEP_A)));
    files.push((
        "b/src/lib.rs".into(),
        "pub fn caller() {\n    a::q::f();\n}\n".into(),
    ));
    let cg = repo(&files);
    binds(
        &cg,
        ("b/src/lib.rs", "caller", "a::q::f"),
        ("a/src/q.rs", "f", 9),
    );
}

#[test]
fn x5_dependency_reexporting_a_third_crates_mounted_module() {
    // R3: `c` holds the mount, `a` has `pub use c::p;`, `b` calls `a::p::q::f()`.
    let files: Vec<(String, String)> = vec![
        ("Cargo.toml".into(), WS.into()),
        ("c/Cargo.toml".into(), manifest("c", "")),
        ("c/src/lib.rs".into(), "pub mod p;\n".into()),
        (
            "c/src/p.rs".into(),
            "#[path = \"q.rs\"]\npub mod q;\n".into(),
        ),
        ("c/src/q.rs".into(), Q_REAL.into()),
        ("c/src/p/q.rs".into(), Q_DECOY.into()),
        (
            "a/Cargo.toml".into(),
            manifest("a", "[dependencies]\nc = { path = \"../c\" }\n"),
        ),
        ("a/src/lib.rs".into(), "pub use c::p;\n".into()),
        ("b/Cargo.toml".into(), manifest("b", B_DEP_A)),
        (
            "b/src/lib.rs".into(),
            "pub fn caller() {\n    a::p::q::f();\n}\n".into(),
        ),
    ];
    let cg = repo(&files);
    binds(
        &cg,
        ("b/src/lib.rs", "caller", "a::p::q::f"),
        ("c/src/q.rs", "f", 9),
    );
}
