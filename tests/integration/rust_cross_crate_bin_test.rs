//! #749: a binary root of a workspace member sees the package's in-repo
//! `[dependencies]` under their in-source names, so `a::m::f()` from
//! `b/src/main.rs` resolves like the same path from `b/src/lib.rs`.
//!
//! Manifest-parsed fixtures through `load_repo` and the navigation index (the
//! production route), asserting the full `resolve_call_site_full` outcome. RED rows
//! (C1 C1b C5 C6 C7b C9 C12 C17c) fail while `Builder::finish` keys library roots only; KEEP
//! rows pin what must not change (a row whose outcome is inherited is pinned on
//! base and head alike, and says so). Each row kills a mutant of
//! `mutants/rust-bin-root-deps.json`.
use prism::{
    call_graph::{CallGraph, CallSite},
    repo_loader::load_repo,
    resolution::{DropReason, ResolutionConfidence, ResolutionKind},
};
use std::collections::BTreeMap;
use tempfile::TempDir;

const WS: &str = "[workspace]\nmembers = [\"a\", \"b\"]\n";
const A_TOML: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const A_LIB: &str =
    "pub mod m;\npub fn to_string() {}\npub struct T;\nimpl T {\n    pub fn run() {}\n}\n";
const A_M: &str = "pub fn f() {}\n";
const B_DEP: &str = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                     [dependencies]\na = { path = \"../a\" }\n";
const B_LIB: &str = "pub fn lib_fn() {}\n";
/// A call through the dependency, and the same function through a `use`.
const CALLS_A: &str = "use a::m::f;\nfn main() {\n    a::m::f();\n}\nfn via_use() {\n    f();\n}\n";

/// The default workspace (`a` library; `b` with a library and `b_toml`), with
/// `extra` replacing or adding files.
fn fixture(b_toml: &str, extra: &[(&str, &str)]) -> CallGraph {
    build(b_toml, extra, &[])
}

/// As [`fixture`], with the default files named in `remove` left out.
fn build(b_toml: &str, extra: &[(&str, &str)], remove: &[&str]) -> CallGraph {
    let mut files: BTreeMap<String, String> = [
        ("Cargo.toml", WS),
        ("a/Cargo.toml", A_TOML),
        ("a/src/lib.rs", A_LIB),
        ("a/src/m.rs", A_M),
        ("b/src/lib.rs", B_LIB),
    ]
    .iter()
    .map(|(p, s)| ((*p).to_string(), (*s).to_string()))
    .collect();
    files.insert("b/Cargo.toml".into(), b_toml.into());
    for (p, s) in extra {
        files.insert((*p).into(), (*s).into());
    }
    for p in remove {
        files.remove(*p);
    }
    let dir = TempDir::new().unwrap();
    for (path, src) in &files {
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

/// Target file, function name, start line, confidence and kind of every resolved target.
type Targets = Vec<(String, String, usize, ResolutionConfidence, ResolutionKind)>;

fn outcome(
    cg: &CallGraph,
    file: &str,
    caller: &str,
    callee: &str,
) -> (Targets, Option<DropReason>) {
    let o = cg.resolve_call_site_full(site(cg, file, caller, callee));
    let targets = o
        .resolved
        .iter()
        .map(|r| {
            (
                r.target.file.clone(),
                r.target.name.clone(),
                r.target.start_line,
                r.confidence,
                r.kind,
            )
        })
        .collect();
    (targets, o.drop)
}

/// The complete outcome of an Exact bind: `target` is `file::function@start_line`.
fn exact_t(target: &str, kind: ResolutionKind) -> (Targets, Option<DropReason>) {
    let (file, rest) = target.split_once("::").expect("file::name@line");
    let (name, line) = rest.split_once('@').expect("file::name@line");
    (
        vec![(
            file.to_string(),
            name.to_string(),
            line.parse().unwrap(),
            ResolutionConfidence::Exact,
            kind,
        )],
        None,
    )
}

fn dropped() -> (Targets, Option<DropReason>) {
    (Vec::new(), Some(DropReason::UnknownName))
}

/// The call is bound: exactly one Exact target, `file::function@line`, with `kind`.
fn exact(
    cg: &CallGraph,
    file: &str,
    caller: &str,
    callee: &str,
    target: &str,
    kind: ResolutionKind,
) {
    assert_eq!(
        outcome(cg, file, caller, callee),
        exact_t(target, kind),
        "{file}:{caller} {callee}"
    );
}

/// The call is refused: no target, dropped `UnknownName`.
fn refused(cg: &CallGraph, file: &str, caller: &str, callee: &str) {
    assert_eq!(
        outcome(cg, file, caller, callee),
        dropped(),
        "{file}:{caller} {callee}"
    );
}

const QO: ResolutionKind = ResolutionKind::QualifiedOwner;
const FS: ResolutionKind = ResolutionKind::FreeSingle;

// ── RED: bin roots get the package's dependencies ───────────────────────────

#[test]
fn c1_every_bin_shape_resolves_dependency_paths() {
    let cg = fixture(
        B_DEP,
        &[
            ("b/src/main.rs", CALLS_A),
            ("b/src/bin/tool.rs", CALLS_A),
            ("b/src/bin/multi/main.rs", CALLS_A),
        ],
    );
    for bin in [
        "b/src/main.rs",
        "b/src/bin/tool.rs",
        "b/src/bin/multi/main.rs",
    ] {
        exact(&cg, bin, "main", "a::m::f", "a/src/m.rs::f@1", QO);
        exact(&cg, bin, "via_use", "f", "a/src/m.rs::f@1", FS);
    }
}

#[test]
fn c1b_the_bins_own_library_still_resolves_beside_the_dependency() {
    // KEEP: the package's own library entry (`binary_library_deps`) is untouched.
    let cg = fixture(
        B_DEP,
        &[(
            "b/src/main.rs",
            "fn main() {\n    b::lib_fn();\n    a::m::f();\n}\n",
        )],
    );
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "b::lib_fn",
        "b/src/lib.rs::lib_fn@1",
        QO,
    );
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "a::m::f",
        "a/src/m.rs::f@1",
        QO,
    );
}

#[test]
fn c5_renamed_dependency_resolves_under_its_in_source_name() {
    let toml = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                [dependencies]\nalpha = { package = \"a\", path = \"../a\" }\n";
    let cg = fixture(
        toml,
        &[
            ("b/src/main.rs", "fn main() {\n    alpha::m::f();\n}\n"),
            // KEEP: the package name is not an in-source name.
            ("b/src/bin/old.rs", "fn main() {\n    a::m::f();\n}\n"),
        ],
    );
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "alpha::m::f",
        "a/src/m.rs::f@1",
        QO,
    );
    refused(&cg, "b/src/bin/old.rs", "main", "a::m::f");
}

#[test]
fn c6_bin_only_package_resolves_dependency_paths() {
    // No library root in `b`: a bin-only package can still name its dependencies.
    let cg = build(
        B_DEP,
        &[("b/src/main.rs", "fn main() {\n    a::m::f();\n}\n")],
        &["b/src/lib.rs"],
    );
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "a::m::f",
        "a/src/m.rs::f@1",
        QO,
    );
}

#[test]
fn c9_explicit_bin_path_resolves_and_replaces_auto_discovery() {
    let toml = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                [[bin]]\nname = \"x\"\npath = \"tools/x.rs\"\n\
                [dependencies]\na = { path = \"../a\" }\n";
    let cg = fixture(
        toml,
        &[
            ("b/tools/x.rs", "fn main() {\n    a::m::f();\n}\n"),
            // Retained exclusion: an explicit `[[bin]]` list replaces auto-discovery.
            ("b/src/main.rs", "fn main() {\n    a::m::f();\n}\n"),
        ],
    );
    exact(
        &cg,
        "b/tools/x.rs",
        "main",
        "a::m::f",
        "a/src/m.rs::f@1",
        QO,
    );
    refused(&cg, "b/src/main.rs", "main", "a::m::f");
}

// ── KEEP: what must not be keyed ────────────────────────────────────────────

#[test]
fn c2_no_dependency_entry_drops() {
    let toml = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    let cg = fixture(
        toml,
        &[("b/src/main.rs", "fn main() {\n    a::m::f();\n}\n")],
    );
    refused(&cg, "b/src/main.rs", "main", "a::m::f");
}

#[test]
fn c3_dev_dependency_only_drops() {
    let toml = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                [dev-dependencies]\na = { path = \"../a\" }\n";
    let cg = fixture(
        toml,
        &[("b/src/main.rs", "fn main() {\n    a::m::f();\n}\n")],
    );
    refused(&cg, "b/src/main.rs", "main", "a::m::f");
}

#[test]
fn c4_build_script_and_other_target_roots_get_no_dependency_entry() {
    let call = "fn main() {\n    a::m::f();\n}\n";
    let cg = fixture(
        B_DEP,
        &[
            ("b/build.rs", call),
            ("b/src/main.rs", "fn main() {}\n"),
            ("b/tests/it.rs", call),
            ("b/examples/ex.rs", call),
            ("b/benches/bn.rs", call),
        ],
    );
    // Inherited on base and head, never the graph's QualifiedOwner: build.rs is no modeled
    // root (it sees `[build-dependencies]`, not parsed) and falls to the legacy stem route;
    // tests, examples and benches are roots that also see dev-dependencies (not parsed), so
    // the graph refuses them.
    exact(
        &cg,
        "b/build.rs",
        "main",
        "a::m::f",
        "a/src/m.rs::f@1",
        ResolutionKind::StemSingle,
    );
    for file in ["b/tests/it.rs", "b/examples/ex.rs", "b/benches/bn.rs"] {
        refused(&cg, file, "main", "a::m::f");
    }
}

#[test]
fn c8_external_dependency_has_no_in_repo_edge() {
    let toml = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                [dependencies]\nserde_json = \"1\"\na = { path = \"../a\" }\n";
    let cg = fixture(
        toml,
        &[(
            "b/src/main.rs",
            "fn main() {\n    serde_json::to_string();\n}\n",
        )],
    );
    // `a` exports an in-repo `to_string`; the external crate must not bind it.
    refused(&cg, "b/src/main.rs", "main", "serde_json::to_string");
}

#[test]
fn c10_edition_2015_package_drops() {
    let a = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2015\"\n";
    let b = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2015\"\n\
             [dependencies]\na = { path = \"../a\" }\n";
    let cg = fixture(
        b,
        &[
            ("a/Cargo.toml", a),
            ("b/src/main.rs", "fn main() {\n    a::m::f();\n}\n"),
        ],
    );
    refused(&cg, "b/src/main.rs", "main", "a::m::f");
}

#[test]
fn c11_autobins_false_keys_no_auto_bin() {
    let toml =
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\nautobins = false\n\
                [dependencies]\na = { path = \"../a\" }\n";
    let cg = fixture(
        toml,
        &[
            ("b/src/main.rs", "fn main() {\n    a::m::f();\n}\n"),
            ("b/src/bin/tool.rs", "fn main() {\n    a::m::f();\n}\n"),
        ],
    );
    refused(&cg, "b/src/bin/tool.rs", "main", "a::m::f");
    refused(&cg, "b/src/main.rs", "main", "a::m::f");
}

#[test]
fn c12_bin_with_its_own_edition_is_not_keyed() {
    let toml = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                [[bin]]\nname = \"tool\"\npath = \"src/bin/tool.rs\"\nedition = \"2018\"\n\
                [[bin]]\nname = \"plain\"\npath = \"src/bin/plain.rs\"\n\
                [dependencies]\na = { path = \"../a\" }\n";
    let cg = fixture(
        toml,
        &[
            ("b/src/bin/tool.rs", "fn main() {\n    a::m::f();\n}\n"),
            ("b/src/bin/plain.rs", "fn main() {\n    a::m::f();\n}\n"),
        ],
    );
    refused(&cg, "b/src/bin/tool.rs", "main", "a::m::f");
    // The sibling without an edition override is keyed (the skip is per target).
    exact(
        &cg,
        "b/src/bin/plain.rs",
        "main",
        "a::m::f",
        "a/src/m.rs::f@1",
        QO,
    );
}

// ── shadows ─────────────────────────────────────────────────────────────────

#[test]
fn c13_bin_local_module_shadows_the_dependency() {
    let cg = fixture(
        B_DEP,
        &[(
            "b/src/main.rs",
            "mod a {\n    pub mod m {\n        pub fn f() {}\n    }\n}\nfn main() {\n    a::m::f();\n}\n",
        )],
    );
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "a::m::f",
        "b/src/main.rs::f@3",
        QO,
    );
}

#[test]
fn c14_block_local_binding_of_the_dependency_name_declines() {
    // A block-local import (and a block-local module) named `a`: the extern fallback
    // declines instead of binding the dependency (`rust_policy::extern_crate_root`).
    let cg = fixture(
        B_DEP,
        &[(
            "b/src/main.rs",
            "mod other {\n    pub mod m {\n        pub fn f() {}\n    }\n}\n\
             fn imported() {\n    use other as a;\n    a::m::f();\n}\n\
             fn modular() {\n    mod a {\n        pub mod m {\n            pub fn f() {}\n        }\n    }\n    a::m::f();\n}\n",
        )],
    );
    refused(&cg, "b/src/main.rs", "imported", "a::m::f");
    refused(&cg, "b/src/main.rs", "modular", "a::m::f");
}

#[test]
fn c16_block_local_type_shadow_binds_the_local_associated_fn() {
    let cg = fixture(
        B_DEP,
        &[
            ("a/src/lib.rs", "pub fn m() {}\n"),
            (
                "b/src/main.rs",
                "fn main() {\n    struct a;\n    impl a {\n        fn m() {}\n    }\n    a::m();\n}\n",
            ),
        ],
    );
    // Same on base (where `a` has no entry) and head: the `NS_TYPE` guard declines the
    // dependency and the local associated `a::m` binds.
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "a::m",
        "b/src/main.rs::m@4",
        QO,
    );
}

// ── conflict rule and owner-pool fallback ───────────────────────────────────

const B_CONFLICT: &str = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                          [lib]\nname = \"x\"\n\
                          [dependencies]\nx = { package = \"a\", path = \"../a\" }\n";

#[test]
fn c7_dependency_named_like_the_own_library_is_not_keyed() {
    let cg = fixture(
        B_CONFLICT,
        &[(
            "b/src/main.rs",
            "fn main() {\n    x::m::f();\n    x::T::run();\n}\n",
        )],
    );
    // Free-function path: the graph rung refuses (no winner for the name `x`).
    refused(&cg, "b/src/main.rs", "main", "x::m::f");
    // Associated path with a singleton `(T, run)` owner pool: the owner-pool fallback of
    // `rust_authoritative_nonself` is INHERITED (identical on base and head), not a
    // refusal this rule adds or removes.
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "x::T::run",
        "a/src/lib.rs::run@5",
        QO,
    );
}

#[test]
fn c7b_only_the_conflicting_dependency_is_dropped() {
    let toml = "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                [lib]\nname = \"x\"\n\
                [dependencies]\na = { path = \"../a\" }\nx = { package = \"a\", path = \"../a\" }\n";
    let cg = fixture(
        toml,
        &[(
            "b/src/main.rs",
            "fn main() {\n    a::m::f();\n    x::m::f();\n}\n",
        )],
    );
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "a::m::f",
        "a/src/m.rs::f@1",
        QO,
    );
    refused(&cg, "b/src/main.rs", "main", "x::m::f");
}

#[test]
fn c15_bin_local_owner_with_the_dependencys_owner_name() {
    let cg = fixture(
        B_DEP,
        &[
            ("a/src/lib.rs", "pub mod m;\npub struct T;\nimpl T {\n    pub fn run() {}\n}\n"),
            (
                "b/src/main.rs",
                "struct T;\nimpl T {\n    fn run() {}\n}\nfn main() {\n    a::T::run();\n    T::run();\n}\n",
            ),
        ],
    );
    // The bare name `T::run` is a two-owner pool here; the local type binds the local fn.
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "T::run",
        "b/src/main.rs::run@3",
        QO,
    );
    // Pinned as inherited (identical on base and head): the dependency's `T::run` through the
    // owner pool, not the local type's.
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "a::T::run",
        "a/src/lib.rs::run@4",
        QO,
    );
}

// ── #[path] placement (rustc's since #765; no guard here) ───────────────────
//
// `#[path = "q.rs"] pub mod q;` in `a/src/p.rs`: rustc loads `a/src/q.rs`; the retained decoy
// `a/src/p/q.rs` must not be bound by a bin root's dependency path.

fn mount_files(call: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("a/src/lib.rs", "pub mod p;\n"),
        ("a/src/p.rs", "#[path = \"q.rs\"]\npub mod q;\n"),
        ("a/src/q.rs", "pub fn f() -> bool { true }\n"), // what rustc loads
        ("a/src/p/q.rs", "pub fn f() -> bool { false }\n"), // the decoy
        ("b/src/main.rs", call),
    ]
}

const MOUNT_CALL: &str = "fn main() {\n    a::p::q::f();\n}\n";

#[test]
fn c17_bin_dependency_path_binds_the_file_rustc_loads_for_a_path_mount() {
    let cg = fixture(B_DEP, &mount_files(MOUNT_CALL));
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "a::p::q::f",
        "a/src/q.rs::f@1",
        QO,
    );
}

#[test]
fn c17b_bin_dependency_path_via_use_binds_the_file_rustc_loads_for_a_path_mount() {
    let cg = fixture(
        B_DEP,
        &mount_files("use a::p::q::f;\nfn main() {\n    f();\n}\n"),
    );
    exact(&cg, "b/src/main.rs", "main", "f", "a/src/q.rs::f@1", FS);
}

#[test]
fn c17c_bin_dependency_path_binds_a_correctly_placed_mount() {
    let cg = fixture(
        B_DEP,
        &[
            ("a/src/lib.rs", "pub mod p;\n"),
            // Directly in a mod-rs file both calculations give `a/src/p/q.rs`.
            ("a/src/p/mod.rs", "#[path = \"q.rs\"]\npub mod q;\n"),
            ("a/src/p/q.rs", "pub fn f() -> bool { true }\n"),
            ("b/src/main.rs", MOUNT_CALL),
        ],
    );
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "a::p::q::f",
        "a/src/p/q.rs::f@1",
        QO,
    );
}

#[test]
fn c17d_library_root_consumer_binds_the_file_rustc_loads_for_a_path_mount() {
    // The library-root route agrees with the bin-root route (#765).
    let mut files = mount_files(MOUNT_CALL);
    files.push(("b/src/lib.rs", "pub fn lib_fn() {\n    a::p::q::f();\n}\n"));
    let cg = fixture(B_DEP, &files);
    let got = outcome(&cg, "b/src/lib.rs", "lib_fn", "a::p::q::f");
    assert_eq!(got, exact_t("a/src/q.rs::f@1", QO), "{got:?}");
}

// ── block-scope shadows through a module-level import (review fold, #768) ────
//
// A qualified path starts its lookup at the enclosing MODULE, chases a module-level
// import and hands the import's own scope to the extern fallback, so a block-scope
// binding of the first segment is never seen. rustc binds the block's.

const MODULE_IMPORT: &str = "use a::m as alias;\nmod local {\n    pub fn f() {}\n}\n";

#[test]
fn c18_block_import_shadows_a_module_level_import_of_the_dependency() {
    // rustc binds `local::f`; head must not bind the dependency's `a::m::f`.
    let main = format!(
        "{MODULE_IMPORT}fn main() {{\n    use crate::local as alias;\n    alias::f();\n}}\n"
    );
    let cg = fixture(B_DEP, &[("b/src/main.rs", main.as_str())]);
    refused(&cg, "b/src/main.rs", "main", "alias::f");
}

#[test]
fn c18b_block_module_shadows_a_module_level_import_of_the_dependency() {
    let main = format!(
        "{MODULE_IMPORT}fn main() {{\n    mod alias {{\n        pub fn f() {{}}\n    }}\n    alias::f();\n}}\n"
    );
    let cg = fixture(B_DEP, &[("b/src/main.rs", main.as_str())]);
    refused(&cg, "b/src/main.rs", "main", "alias::f");
}

#[test]
fn c18c_unshadowed_module_level_import_of_the_dependency_binds() {
    // Control: no block binding, the import chases to the dependency.
    let main = format!("{MODULE_IMPORT}fn main() {{\n    alias::f();\n}}\n");
    let cg = fixture(B_DEP, &[("b/src/main.rs", main.as_str())]);
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "alias::f",
        "a/src/m.rs::f@1",
        QO,
    );
}

#[test]
fn c18d_block_binding_of_another_name_does_not_refuse() {
    // Control: a block binding of a different name is not a shadow of `alias`.
    let main = format!(
        "{MODULE_IMPORT}fn main() {{\n    use crate::local as other;\n    other::f();\n    alias::f();\n}}\n"
    );
    let cg = fixture(B_DEP, &[("b/src/main.rs", main.as_str())]);
    exact(
        &cg,
        "b/src/main.rs",
        "main",
        "alias::f",
        "a/src/m.rs::f@1",
        QO,
    );
}

// ── re-export and receiver routes through a `#[path]` mount (#765) ──────────
//
// Each row below asserts the file rustc loads (never the decoy `.../p/q.rs`): they bound the
// decoy past the placement stopgap this PR carried until #765's fix landed.

/// Resolved targets as `file::function@line`, all Exact.
fn exact_targets(cg: &CallGraph, file: &str, caller: &str, callee: &str) -> Vec<String> {
    let (targets, drop) = outcome(cg, file, caller, callee);
    assert!(
        targets.iter().all(|t| t.3 == ResolutionConfidence::Exact),
        "{targets:?} {drop:?}"
    );
    targets
        .iter()
        .map(|t| format!("{}::{}@{}", t.0, t.1, t.2))
        .collect()
}

const WS3: &str = "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\n";
const C_TOML: &str = "[package]\nname = \"c\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const A_WITH_C: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                        [dependencies]\nc = { path = \"../c\" }\n";
const OWNER_Q: &str =
    "pub struct S;\nimpl S {\n    pub fn run(&self) {}\n}\npub fn make() -> S {\n    S\n}\n";

/// Dependency `a` whose `p.rs` mounts `q.rs` by `#[path]` (rustc loads `a/src/q.rs`;
/// `a/src/p/q.rs` is the decoy), with `q`, `decoy` as file bodies.
fn a_mount<'a>(q: &'a str, decoy: &'a str, lib: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("a/src/lib.rs", lib),
        ("a/src/p.rs", "#[path = \"q.rs\"]\npub mod q;\n"),
        ("a/src/q.rs", q),
        ("a/src/p/q.rs", decoy),
    ]
}

#[test]
fn w2a_reexport_in_a_misplaced_mount_binds_rustcs_target() {
    let mut files = a_mount(
        "pub use crate::right as f;\n",
        "pub use crate::wrong as f;\n",
        "pub mod p;\npub fn right() {}\npub fn wrong() {}\n",
    );
    files.push(("b/src/main.rs", "fn main() {\n    a::p::q::f();\n}\n"));
    let cg = fixture(B_DEP, &files);
    assert_eq!(
        exact_targets(&cg, "b/src/main.rs", "main", "a::p::q::f"),
        ["a/src/lib.rs::right@2"]
    );
}

/// Crate `c` holds the `#[path]` mount (`c/src/q.rs` is rustc's file, `c/src/p/q.rs` the decoy);
/// `a` depends on `c` and re-exports it; `b` depends on `a` only.
fn c_mount(a_lib: &'static str, call: &'static str) -> CallGraph {
    build(
        B_DEP,
        &[
            ("Cargo.toml", WS3),
            ("a/Cargo.toml", A_WITH_C),
            ("a/src/lib.rs", a_lib),
            ("c/Cargo.toml", C_TOML),
            ("c/src/lib.rs", "pub mod p;\n"),
            ("c/src/p.rs", "#[path = \"q.rs\"]\npub mod q;\n"),
            ("c/src/q.rs", "pub fn f() -> bool {\n    true\n}\n"),
            ("c/src/p/q.rs", "pub fn f() -> bool {\n    false\n}\n"),
            ("b/src/main.rs", call),
        ],
        &["a/src/m.rs"],
    )
}

#[test]
fn w2b_item_reexport_of_a_third_crates_misplaced_module_binds_rustcs_target() {
    let cg = c_mount("pub use c::p::q::f;\n", "fn main() {\n    a::f();\n}\n");
    assert_eq!(
        exact_targets(&cg, "b/src/main.rs", "main", "a::f"),
        ["c/src/q.rs::f@1"]
    );
}

#[test]
fn r3_transitive_module_reexport_binds_rustcs_target() {
    let cg = c_mount("pub use c::p;\n", "fn main() {\n    a::p::q::f();\n}\n");
    assert_eq!(
        exact_targets(&cg, "b/src/main.rs", "main", "a::p::q::f"),
        ["c/src/q.rs::f@1"]
    );
}

#[test]
fn r1_typed_parameter_receiver_binds_rustcs_target() {
    let mut files = a_mount(OWNER_Q, OWNER_Q, "pub mod p;\n");
    files.push((
        "b/src/main.rs",
        "fn go(s: a::p::q::S) {\n    s.run();\n}\nfn main() {}\n",
    ));
    let cg = fixture(B_DEP, &files);
    assert_eq!(
        exact_targets(&cg, "b/src/main.rs", "go", "run"),
        ["a/src/q.rs::run@3"]
    );
}

#[test]
fn r2_return_typed_receiver_binds_rustcs_target() {
    let mut files = a_mount(OWNER_Q, OWNER_Q, "pub mod p;\n");
    files.push((
        "b/src/main.rs",
        "fn go() {\n    let s = a::p::q::make();\n    s.run();\n}\nfn main() {}\n",
    ));
    let cg = fixture(B_DEP, &files);
    assert_eq!(
        exact_targets(&cg, "b/src/main.rs", "go", "run"),
        ["a/src/q.rs::run@3"]
    );
}
