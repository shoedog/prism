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

type Targets = Vec<(String, ResolutionConfidence, ResolutionKind)>;

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
        .map(|r| (r.target.file.clone(), r.confidence, r.kind))
        .collect();
    (targets, o.drop)
}

fn exact_t(file: &str, kind: ResolutionKind) -> (Targets, Option<DropReason>) {
    (
        vec![(file.to_string(), ResolutionConfidence::Exact, kind)],
        None,
    )
}

fn dropped() -> (Targets, Option<DropReason>) {
    (Vec::new(), Some(DropReason::UnknownName))
}

/// The call is bound: exactly one Exact target in `target_file` with `kind`.
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

/// No Exact edge into `forbidden` (whatever else the site resolves to).
fn never_binds(cg: &CallGraph, file: &str, caller: &str, callee: &str, forbidden: &str) {
    let (targets, drop) = outcome(cg, file, caller, callee);
    assert!(
        targets
            .iter()
            .all(|(f, c, _)| !(f == forbidden && *c == ResolutionConfidence::Exact)),
        "{file}:{caller} {callee}: {targets:?} {drop:?}"
    );
}

const QO: ResolutionKind = ResolutionKind::QualifiedOwner;

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
        exact(&cg, bin, "main", "a::m::f", "a/src/m.rs", QO);
        let via_use = outcome(&cg, bin, "via_use", "f");
        assert_eq!(via_use.0.len(), 1, "{bin}: {via_use:?}");
        assert_eq!(
            (via_use.0[0].0.as_str(), via_use.0[0].1, via_use.1),
            ("a/src/m.rs", ResolutionConfidence::Exact, None),
            "{bin}"
        );
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
        "b/src/lib.rs",
        QO,
    );
    exact(&cg, "b/src/main.rs", "main", "a::m::f", "a/src/m.rs", QO);
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
        "a/src/m.rs",
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
    exact(&cg, "b/src/main.rs", "main", "a::m::f", "a/src/m.rs", QO);
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
    exact(&cg, "b/tools/x.rs", "main", "a::m::f", "a/src/m.rs", QO);
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
        "a/src/m.rs",
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
    never_binds(
        &cg,
        "b/src/main.rs",
        "main",
        "serde_json::to_string",
        "a/src/lib.rs",
    );
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
        "a/src/m.rs",
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
    exact(&cg, "b/src/main.rs", "main", "a::m::f", "b/src/main.rs", QO);
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
    never_binds(&cg, "b/src/main.rs", "imported", "a::m::f", "a/src/m.rs");
    never_binds(&cg, "b/src/main.rs", "modular", "a::m::f", "a/src/m.rs");
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
    never_binds(&cg, "b/src/main.rs", "main", "a::m", "a/src/lib.rs");
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
        "a/src/lib.rs",
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
    exact(&cg, "b/src/main.rs", "main", "a::m::f", "a/src/m.rs", QO);
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
    exact(&cg, "b/src/main.rs", "main", "T::run", "b/src/main.rs", QO);
    let qualified = outcome(&cg, "b/src/main.rs", "main", "a::T::run");
    // Pinned (see the observed outcomes in the PR): whatever it is, it is never the local fn.
    never_binds(&cg, "b/src/main.rs", "main", "a::T::run", "b/src/main.rs");
    assert!(qualified.0.len() <= 1, "{qualified:?}");
}

// ── #[path] placement (the #765 discipline for what this rule newly admits) ──
//
// `#[path = "q.rs"] pub mod q;` in `a/src/p.rs`: rustc loads `a/src/q.rs`, the populator
// binds the retained decoy `a/src/p/q.rs`. A bin root's dependency path must not bind it.

fn mount_files(call: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("a/src/lib.rs", "pub mod p;\n"),
        ("a/src/p.rs", "#[path = \"q.rs\"]\npub mod q;\n"),
        ("a/src/q.rs", "pub fn f() -> bool { true }\n"), // what rustc loads
        ("a/src/p/q.rs", "pub fn f() -> bool { false }\n"), // the decoy the populator binds
        ("b/src/main.rs", call),
    ]
}

const MOUNT_CALL: &str = "fn main() {\n    a::p::q::f();\n}\n";

#[test]
fn c17_bin_dependency_path_refuses_a_misplaced_mount() {
    let cg = fixture(B_DEP, &mount_files(MOUNT_CALL));
    refused(&cg, "b/src/main.rs", "main", "a::p::q::f");
}

#[test]
fn c17b_bin_dependency_path_via_use_refuses_a_misplaced_mount() {
    let cg = fixture(
        B_DEP,
        &mount_files("use a::p::q::f;\nfn main() {\n    f();\n}\n"),
    );
    never_binds(&cg, "b/src/main.rs", "main", "f", "a/src/p/q.rs");
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
        "a/src/p/q.rs",
        QO,
    );
}

#[test]
fn c17d_library_root_consumer_keeps_its_inherited_outcome() {
    // KEEP, pinned as inherited (#765): the library-root route into a misplaced mount is
    // unchanged by this rule (it only guards what it newly admits).
    let mut files = mount_files(MOUNT_CALL);
    files.push(("b/src/lib.rs", "pub fn lib_fn() {\n    a::p::q::f();\n}\n"));
    let cg = fixture(B_DEP, &files);
    let got = outcome(&cg, "b/src/lib.rs", "lib_fn", "a::p::q::f");
    assert_eq!(got, exact_t("a/src/p/q.rs", QO), "{got:?}");
}
