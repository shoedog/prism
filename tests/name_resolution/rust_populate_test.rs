//! Task 3a TDD: the Rust **populator** (`populate_rust`) — AST → `ScopeGraph`.
//!
//! Every test parses real Rust source into `ParsedFile`s, calls
//! [`populate_rust`], then **resolves through the PR-1 engine**
//! (`engine::resolve` / `engine::resolve_path` with a [`RustPolicy`]) and
//! asserts on the resolution **status + target** (never a wrong target — the
//! spec §7 cardinal rule). Fall-through / poison cases assert the STATUS
//! (`Unresolved` / `Ambiguous` / `Poisoned`), never an outer same-name.
//!
//! These are end-to-end populate→resolve tests: they pin the populator's job
//! (build the right scopes/bindings/edges/macro-wildcards) *through* the
//! engine's behavior, which is the only thing a consumer (PR-3) cares about.

use std::collections::{BTreeMap, BTreeSet};

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::engine::{resolve, resolve_path, ScopeGraph};
use prism::name_resolution::rust_policy::{RustPolicy, NS_TYPE, NS_VALUE};
use prism::name_resolution::rust_populator::{
    enclosing_scope, file_id, populate_rust, RustCrateConfig,
};
use prism::name_resolution::types::{
    Anchor, NamespaceId, RawPath, ResStatus, Resolution, ResolveQuery, SourceLoc, Target,
};

// ── fixture + query helpers ───────────────────────────────────────────────────

/// Parse one `&str` into a Rust `ParsedFile` keyed by `path`.
fn rs(path: &str, src: &str) -> (String, ParsedFile) {
    (
        path.to_string(),
        ParsedFile::parse(path, src, Language::Rust).unwrap(),
    )
}

/// Assemble a `BTreeMap<String, ParsedFile>` from `(path, src)` pairs.
fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs.iter().map(|(p, s)| rs(p, s)).collect()
}

/// The byte offset of the FIRST occurrence of `needle` in `src` (the test marks
/// a call/use site by its source text).
fn byte_of(src: &str, needle: &str) -> usize {
    src.find(needle)
        .unwrap_or_else(|| panic!("needle {needle:?} not found in source"))
}

/// The byte offset of the Nth (0-based) occurrence of `needle`.
fn byte_of_nth(src: &str, needle: &str, n: usize) -> usize {
    let mut start = 0;
    for _ in 0..n {
        let i = src[start..]
            .find(needle)
            .unwrap_or_else(|| panic!("needle {needle:?} occurrence {n} not found"));
        start += i + needle.len();
    }
    start
        + src[start..]
            .find(needle)
            .unwrap_or_else(|| panic!("needle {needle:?} occurrence {n} not found"))
}

/// Resolve a bare `(name, ns)` from the smallest scope enclosing `(path, byte)`.
fn resolve_bare_at(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    edition: u16,
    path: &str,
    byte: usize,
    name: &str,
    ns: NamespaceId,
) -> Resolution {
    let fid = file_id(fs, path).expect("file id");
    let from = enclosing_scope(g, fid, byte)
        .unwrap_or_else(|| panic!("no enclosing scope at {path}:{byte}"));
    let pol = RustPolicy::new(g, edition);
    let q = ResolveQuery {
        name: name.to_string(),
        ns,
        from,
        at: SourceLoc { file: fid, byte },
        cfg: Default::default(),
        ctx: Default::default(),
    };
    resolve(g, &q, &pol)
}

/// Resolve a `crate::`-anchored path's final `(name, ns)` from the crate root.
fn resolve_crate_path(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    edition: u16,
    root_path: &str,
    segs: &[&str],
    ns: NamespaceId,
) -> Resolution {
    let fid = file_id(fs, root_path).expect("file id");
    // Anchor `crate::` from any scope in that file.
    let from = enclosing_scope(g, fid, 0).expect("root scope");
    let pol = RustPolicy::new(g, edition);
    let path = RawPath(segs.iter().map(|s| s.to_string()).collect());
    resolve_path(
        g,
        &path,
        ns,
        &Anchor::crate_root(),
        from,
        NS_TYPE,
        &SourceLoc { file: fid, byte: 0 },
        &pol,
    )
}

fn assert_resolved_item(res: &Resolution) {
    assert_eq!(
        res.status,
        ResStatus::Resolved,
        "expected Resolved, got {:?} ({:?})",
        res.status,
        res.candidates
    );
    assert_eq!(res.candidates.len(), 1, "expected one candidate");
    assert!(
        matches!(res.candidates[0].target, Target::Item { .. }),
        "expected Item target, got {:?}",
        res.candidates[0].target
    );
}

fn assert_same_resolved_target(actual: &Resolution, expected: &Resolution, msg: &str) {
    assert_resolved_item(actual);
    assert_resolved_item(expected);
    assert_eq!(
        actual.candidates[0].target, expected.candidates[0].target,
        "{msg}; got {:?}, expected {:?}",
        actual.candidates[0].target, expected.candidates[0].target
    );
}

fn assert_local(res: &Resolution) {
    assert_eq!(
        res.status,
        ResStatus::Resolved,
        "expected Resolved(Local), got {:?} ({:?})",
        res.status,
        res.candidates
    );
    assert_eq!(res.candidates.len(), 1, "expected one candidate");
    assert!(
        matches!(res.candidates[0].target, Target::Local(_)),
        "expected a Local target (shadowing the free fn), got {:?}",
        res.candidates[0].target
    );
}

/// Convention-fallback config (no manifest): edition 2015, conventional roots.
fn convention(fs: &BTreeMap<String, ParsedFile>) -> RustCrateConfig {
    RustCrateConfig::from_convention(fs)
}

// ═══════════════════════════════════════════════════════════════════════════
// END-TO-END: mod + use + resolve
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_end_to_end_mod_use_resolves() {
    let g_src = "use crate::engine::start;\nfn g(){ start() }\n";
    let fs = files(&[
        ("src/lib.rs", "mod engine;\nmod g_mod;\n"),
        ("src/engine.rs", "pub fn start(){}\n"),
        ("src/g_mod.rs", g_src),
    ]);
    let g = populate_rust(&fs, &convention(&fs), None);
    // bare `start()` inside g resolves via the `use crate::engine::start` import.
    let res = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/g_mod.rs",
        byte_of(g_src, "start()"),
        "start",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

#[test]
fn test_inline_mod_resolves_via_path() {
    let fs = files(&[("src/lib.rs", "pub mod bar {\n    pub fn inner(){}\n}\n")]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["bar", "inner"], NS_VALUE);
    assert_resolved_item(&res);
}

#[test]
fn test_file_and_mod_both_present_is_ambiguous() {
    // `mod foo;` with BOTH foo.rs and foo/mod.rs present → ambiguous binding.
    let fs = files(&[
        ("src/lib.rs", "mod foo;\n"),
        ("src/foo.rs", "pub fn a(){}\n"),
        ("src/foo/mod.rs", "pub fn a(){}\n"),
    ]);
    let g = populate_rust(&fs, &convention(&fs), None);
    // Resolving the `foo` module name (Type ns) at the root is AMBIGUOUS (two
    // distinct module-scope bindings under compatible cfg).
    let res = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["foo"], NS_TYPE);
    assert_eq!(
        res.status,
        ResStatus::Ambiguous,
        "foo.rs + foo/mod.rs both present must be Ambiguous, got {:?}",
        res.status
    );
}

#[test]
fn test_cfg_dup_mods_are_distinct_conditioned_bindings() {
    // #[cfg(target_os="linux")] mod imp; + #[cfg(target_os="windows")] mod imp;
    // → conditioned, never merged. Same-key/different-value cfgs are PROVABLY
    // exclusive, so the engine reports a `ResolvedSet` of distinct worlds (the
    // spec's "conditioned, never merged"). (Valueless keys like `unix`/`windows`
    // are NOT provably exclusive in Phase 1 → `Ambiguous`/fall-through, also
    // recall-safe; this fixture pins the stronger ResolvedSet outcome.)
    let fs = files(&[
        (
            "src/lib.rs",
            "#[cfg(target_os = \"linux\")]\nmod imp;\n#[cfg(target_os = \"windows\")]\nmod imp;\n",
        ),
        ("src/imp.rs", "pub fn p(){}\n"),
    ]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["imp"], NS_TYPE);
    assert_eq!(
        res.status,
        ResStatus::ResolvedSet,
        "cfg-exclusive dup mods must be a ResolvedSet of distinct worlds, got {:?} ({:?})",
        res.status,
        res.candidates
    );
    assert_eq!(res.candidates.len(), 2, "expected two conditioned bindings");
}

// ═══════════════════════════════════════════════════════════════════════════
// NESTED OUT-OF-LINE MODULE — declaring-module-directory rule
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_nested_module_via_declaring_dir_foors() {
    // src/foo.rs with `mod bar;` → src/foo/bar.rs (NOT src/bar.rs).
    let fs = files(&[
        ("src/lib.rs", "mod foo;\n"),
        ("src/foo.rs", "pub mod bar;\n"), // declaring module dir is src/foo/
        ("src/foo/bar.rs", "pub fn deep(){}\n"),
        // A decoy at src/bar.rs that must NOT be chosen.
        ("src/bar.rs", "pub fn wrong(){}\n"),
    ]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_crate_path(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        &["foo", "bar", "deep"],
        NS_VALUE,
    );
    assert_resolved_item(&res);
    // The decoy path crate::foo::bar::wrong must NOT resolve (wrong lives in
    // src/bar.rs, which is unreachable from foo::bar).
    let decoy = resolve_crate_path(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        &["foo", "bar", "wrong"],
        NS_VALUE,
    );
    assert_ne!(
        decoy.status,
        ResStatus::Resolved,
        "src/bar.rs must NOT be the child of foo (declaring-dir rule)"
    );
}

#[test]
fn test_nested_module_via_declaring_dir_foomod() {
    // src/foo/mod.rs with `mod bar;` → src/foo/bar.rs.
    let fs = files(&[
        ("src/lib.rs", "mod foo;\n"),
        ("src/foo/mod.rs", "pub mod bar;\n"),
        ("src/foo/bar.rs", "pub fn deep(){}\n"),
    ]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_crate_path(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        &["foo", "bar", "deep"],
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

// ═══════════════════════════════════════════════════════════════════════════
// LOCAL BINDINGS — every form → Target::Local, shadows a free `fn f`
// ═══════════════════════════════════════════════════════════════════════════

/// Helper: parse a single-file crate, resolve bare `name` at the byte of `marker`.
fn single_file_resolve(
    src: &str,
    edition: u16,
    marker: &str,
    name: &str,
    ns: NamespaceId,
) -> (Resolution, BTreeMap<String, ParsedFile>) {
    let owned: &'static str = Box::leak(src.to_string().into_boxed_str());
    let fs = files(&[("src/lib.rs", owned)]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_bare_at(
        &g,
        &fs,
        edition,
        "src/lib.rs",
        byte_of(src, marker),
        name,
        ns,
    );
    (res, fs)
}

#[test]
fn test_local_let_shadows_free_fn() {
    let src = "pub fn f(){}\nfn host(){ let f = 1; let _z = f; }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_param_shadows_free_fn() {
    let src = "pub fn f(){}\nfn host(g: u32){ let _z = g; }\nfn other(){}\n";
    // param `g` resolves to a Local inside host.
    let (res, _) = single_file_resolve(src, 2015, "_z = g", "g", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_closure_arg_shadows_free_fn() {
    let src = "pub fn f(){}\nfn host(){ let c = |f| { let _z = f; }; }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_for_pattern_shadows_free_fn() {
    let src = "pub fn f(){}\nfn host(){ for f in 0..3 { let _z = f; } }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_match_pattern_shadows_free_fn() {
    let src = "pub fn f(){}\nfn host(o: Option<u32>){ match o { Some(f) => { let _z = f; }, None => {} } }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_if_let_shadows_free_fn() {
    let src = "pub fn f(){}\nfn host(o: Option<u32>){ if let Some(f) = o { let _z = f; } }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_while_let_shadows_free_fn() {
    let src = "pub fn f(){}\nfn host(mut it: I){ while let Some(f) = it.next() { let _z = f; } }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

// ── multi-binding / non-first-identifier patterns ─────────────────────────────

#[test]
fn test_local_tuple_pattern_non_first_ident() {
    // let (_, f) = pair(); — f is the SECOND, non-first binding in the tuple.
    let src = "pub fn f(){}\nfn host(){ let (_, f) = (1u32, 2u32); let _z = f; }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_struct_pattern_field_rename() {
    // let Point { y: f, .. } = p; — f bound via a field-pattern rename.
    let src = "pub fn f(){}\nstruct Point{ y: u32 }\nfn host(p: Point){ let Point { y: f, .. } = p; let _z = f; }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_match_tuple_non_first_ident() {
    let src = "pub fn f(){}\nfn host(t: (u32,u32)){ match t { (_, f) => { let _z = f; } } }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

#[test]
fn test_local_closure_tuple_non_first_ident() {
    let src = "pub fn f(){}\nfn host(){ let c = |(_, f)| { let _z = f; }; }\n";
    let (res, _) = single_file_resolve(src, 2015, "_z = f", "f", NS_VALUE);
    assert_local(&res);
}

// ═══════════════════════════════════════════════════════════════════════════
// BLOCK-LOCAL `use` extent
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_block_local_use_extent() {
    // The inner block imports `before`; a `before()` INSIDE resolves via the
    // import, while the calls BEFORE and AFTER the block do NOT (the import's
    // vis_extents cover only the block).
    let fs = files(&[(
        "src/lib.rs",
        concat!(
            "mod m { pub fn before(){} }\n",
            "fn host(){\n",
            "    before();\n",
            "    { use crate::m::before; before(); }\n",
            "    before();\n",
            "}\n",
        ),
    )]);
    let full = fs.get("src/lib.rs").unwrap().source.clone();
    let g = populate_rust(&fs, &convention(&fs), None);

    // The INSIDE call (2nd `before()` occurrence after the import) resolves.
    let inside_byte = byte_of(&full, "before(); }");
    let inside = resolve_bare_at(&g, &fs, 2015, "src/lib.rs", inside_byte, "before", NS_VALUE);
    assert_resolved_item(&inside);

    // The first `before();` (before the block) does NOT resolve to m::before.
    let before_byte = byte_of(&full, "    before();\n    {");
    let outside_pre = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        before_byte + 4,
        "before",
        NS_VALUE,
    );
    assert_eq!(
        outside_pre.status,
        ResStatus::Unresolved,
        "the call BEFORE the block must fall through, not see the block-local import"
    );
    assert!(
        outside_pre.candidates.is_empty(),
        "fall-through must not carry a wrong target: {:?}",
        outside_pre.candidates
    );

    // The last `before();` (after the block) does NOT resolve either.
    let after_byte = byte_of_nth(&full, "before();", 2);
    let outside_post = resolve_bare_at(&g, &fs, 2015, "src/lib.rs", after_byte, "before", NS_VALUE);
    assert_eq!(
        outside_post.status,
        ResStatus::Unresolved,
        "the call AFTER the block must fall through, not see the block-local import"
    );
    assert!(
        outside_post.candidates.is_empty(),
        "fall-through must not carry a wrong target: {:?}",
        outside_post.candidates
    );
}

#[test]
fn test_same_callable_named_use_is_position_gated_above_declaration() {
    // The first `h()` is above the block-local named use, so it must resolve to
    // the module free fn, not the same-name import. The second call sees the use.
    let lib = concat!(
        "fn h(){}\n",
        "mod m { pub fn h(){} }\n",
        "fn host(){ h(); use crate::m::h; h(); }\n",
    );
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let free_h = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["h"], NS_VALUE);
    let imported_h = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["m", "h"], NS_VALUE);

    let before = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "h(); use"),
        "h",
        NS_VALUE,
    );
    assert_same_resolved_target(
        &before,
        &free_h,
        "call above same-callable named use must resolve to the free fn decoy",
    );
    assert_ne!(
        before.candidates[0].target, imported_h.candidates[0].target,
        "call above same-callable named use must not resolve via the later import"
    );

    let after = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of_nth(lib, "h();", 1),
        "h",
        NS_VALUE,
    );
    assert_same_resolved_target(
        &after,
        &imported_h,
        "call after same-callable named use should resolve through the import",
    );
}

#[test]
fn test_same_callable_glob_use_is_position_gated_above_declaration() {
    // Before the block-local glob, `f()` must recover the module free fn. After the
    // glob (now in scope), `f()` resolves through the expanded glob to the module fn.
    let lib = concat!(
        "fn f(){}\n",
        "mod m { pub fn f(){} }\n",
        "fn g(){ f(); use crate::m::*; f(); }\n",
    );
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let free_f = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["f"], NS_VALUE);
    let glob_decoy = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["m", "f"], NS_VALUE);

    let before = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "f(); use"),
        "f",
        NS_VALUE,
    );
    assert_same_resolved_target(
        &before,
        &free_f,
        "call above block-local glob must resolve as if the glob is out of scope",
    );
    assert_ne!(
        before.candidates[0].target, glob_decoy.candidates[0].target,
        "call above block-local glob must not resolve to the glob decoy"
    );

    let after = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of_nth(lib, "f();", 1),
        "f",
        NS_VALUE,
    );
    assert_same_resolved_target(
        &after,
        &glob_decoy,
        "call after block-local glob must resolve through the in-scope glob",
    );
}

#[test]
fn test_module_level_use_visible_before_declaration() {
    // Module/item-level `use` is order-independent: a call above the import
    // still sees it. Block-local `use` remains position-gated by
    // `test_block_local_use_extent`.
    let lib = concat!(
        "mod m { pub fn f(){} }\n",
        "fn host(){ f(); }\n",
        "use crate::m::f;\n",
    );
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "f();"),
        "f",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

// ═══════════════════════════════════════════════════════════════════════════
// USE GROUPS / ALIAS / EXTERN CRATE / WORKSPACE / CRATE ROOTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_use_group_and_self() {
    // use crate::a::{b, c::d, self}; — b, d, and `a` itself all become bindings.
    // The import lives in `user.rs` (NOT co-located with `mod a`, which would be
    // a redundant-definition error in Rust).
    let user = "use crate::a::{b, c::d, self};\nfn h(){ b(); d(); }\n";
    let fs = files(&[
        ("src/lib.rs", "mod a;\nmod user;\n"),
        ("src/a.rs", "pub fn b(){}\npub mod c { pub fn d(){} }\n"),
        ("src/user.rs", user),
    ]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let rb = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/user.rs",
        byte_of(user, "b()"),
        "b",
        NS_VALUE,
    );
    assert_resolved_item(&rb);
    let rd = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/user.rs",
        byte_of(user, "d()"),
        "d",
        NS_VALUE,
    );
    assert_resolved_item(&rd);
    // `self` in the group binds the module `a` itself in the Type ns.
    let ra = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/user.rs",
        byte_of(user, "fn h"),
        "a",
        NS_TYPE,
    );
    assert_eq!(
        ra.status,
        ResStatus::Resolved,
        "use a::{{self}} should bind `a` as a Type, got {:?}",
        ra.status
    );
}

#[test]
fn test_use_alias() {
    // use crate::a::orig as c; → bare `c()` resolves to a::orig.
    let fs = files(&[
        (
            "src/lib.rs",
            "mod a;\nuse crate::a::orig as c;\nfn h(){ c(); }\n",
        ),
        ("src/a.rs", "pub fn orig(){}\n"),
    ]);
    let full = fs.get("src/lib.rs").unwrap().source.clone();
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(&full, "c()"),
        "c",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

#[test]
fn test_use_alias_binds_type_namespace_under_alias_not_original() {
    // `use crate::a::Orig as C;` binds `C` in the Type namespace. It must NOT
    // create a bare `Orig` Type binding in this scope.
    let lib = "mod a { pub struct Orig; }\nuse crate::a::Orig as C;\nfn h(){}\n";
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let alias = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "fn h"),
        "C",
        NS_TYPE,
    );
    assert_resolved_item(&alias);

    let original = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "fn h"),
        "Orig",
        NS_TYPE,
    );
    assert_eq!(
        original.status,
        ResStatus::Unresolved,
        "aliased use must not bind the original last segment as a bare Type name; got {:?} ({:?})",
        original.status,
        original.candidates
    );
}

#[test]
fn test_non_aliased_use_binds_type_namespace_under_original_name() {
    // Control for the alias case: without `as`, the imported Type name is `Orig`.
    let lib = "mod a { pub struct Orig; }\nuse crate::a::Orig;\nfn h(){}\n";
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let expected = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["a", "Orig"], NS_TYPE);

    let actual = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "fn h"),
        "Orig",
        NS_TYPE,
    );
    assert_same_resolved_target(
        &actual,
        &expected,
        "non-aliased Type import should bind the original Type name",
    );
}

#[test]
fn test_extern_crate_alias_binds_in_root() {
    // extern crate dep as bar; with `dep` an in-workspace member → `bar` binds
    // (as a Type) at the crate root.
    let lib = "extern crate dep as bar;\n";
    let fs = files(&[
        ("a/src/lib.rs", lib),
        ("dep/src/lib.rs", "pub fn thing(){}\n"),
    ]);
    let cfg = RustCrateConfig {
        edition: 2015,
        crate_roots: vec!["a/src/lib.rs".to_string(), "dep/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "dep".to_string()],
        dep_renames: BTreeMap::new(),
        lib_path: None,
        bin_paths: vec![],
        edition_uniform: true,
        member_in_repo_deps: BTreeMap::new(),
    };
    let g = populate_rust(&fs, &cfg, None);
    // `bar` is a Type binding at a/src/lib.rs's root scope.
    let res = resolve_bare_at(
        &g,
        &fs,
        2015,
        "a/src/lib.rs",
        byte_of(lib, "bar"),
        "bar",
        NS_TYPE,
    );
    assert_eq!(
        res.status,
        ResStatus::Resolved,
        "extern crate dep as bar must bind `bar` at the root, got {:?}",
        res.status
    );
}

#[test]
fn test_workspace_two_members_distinct_roots() {
    // Two workspace members each get their own Root; an item in one does NOT
    // leak as a bare name into the other.
    let a_src = "fn use_site(){ helper(); }\n";
    let fs = files(&[
        ("crate_a/src/lib.rs", a_src),
        ("crate_b/src/lib.rs", "pub fn helper(){}\n"),
    ]);
    let cfg = RustCrateConfig {
        edition: 2018,
        crate_roots: vec![
            "crate_a/src/lib.rs".to_string(),
            "crate_b/src/lib.rs".to_string(),
        ],
        workspace_members: vec!["crate_a".to_string(), "crate_b".to_string()],
        dep_renames: BTreeMap::new(),
        lib_path: None,
        bin_paths: vec![],
        edition_uniform: true,
        member_in_repo_deps: BTreeMap::new(),
    };
    let g = populate_rust(&fs, &cfg, None);
    // `helper()` in crate_a must NOT resolve to crate_b's helper (separate crate).
    let res = resolve_bare_at(
        &g,
        &fs,
        2018,
        "crate_a/src/lib.rs",
        byte_of(a_src, "helper()"),
        "helper",
        NS_VALUE,
    );
    assert_ne!(
        res.status,
        ResStatus::Resolved,
        "a bare name must not cross a crate boundary"
    );
}

#[test]
fn test_explicit_lib_and_bin_roots() {
    // [lib] path + [[bin]] path overrides (non-convention file names).
    let fs = files(&[
        ("weird/entry.rs", "pub fn libfn(){}\n"),
        ("weird/cli.rs", "fn binfn(){}\n"),
    ]);
    let cfg = RustCrateConfig {
        edition: 2018,
        crate_roots: vec!["weird/entry.rs".to_string(), "weird/cli.rs".to_string()],
        workspace_members: vec![],
        dep_renames: BTreeMap::new(),
        lib_path: Some("weird/entry.rs".to_string()),
        bin_paths: vec!["weird/cli.rs".to_string()],
        edition_uniform: true,
        member_in_repo_deps: BTreeMap::new(),
    };
    let g = populate_rust(&fs, &cfg, None);
    // Each root file is its own crate Root scope (enclosing_scope at byte 0 is a Root).
    for p in ["weird/entry.rs", "weird/cli.rs"] {
        let fid = file_id(&fs, p).unwrap();
        let s = enclosing_scope(&g, fid, 0).unwrap_or_else(|| panic!("no root scope for {p}"));
        let kind = &g.scope(s).unwrap().kind;
        assert_eq!(
            *kind,
            prism::name_resolution::types::ScopeKind::Root,
            "{p} should be a crate Root, got {kind:?}"
        );
    }
}

#[test]
fn test_convention_bin_and_tests_roots() {
    // Convention fallback discovers src/bin/<n>/main.rs, benches/, tests/, examples/.
    let fs = files(&[
        ("src/lib.rs", "pub fn l(){}\n"),
        ("src/bin/tool/main.rs", "fn m(){}\n"),
        ("benches/b.rs", "fn bench(){}\n"),
        ("tests/it.rs", "fn t(){}\n"),
        ("examples/e.rs", "fn ex(){}\n"),
    ]);
    let cfg = convention(&fs);
    let roots: BTreeSet<&str> = cfg.crate_roots.iter().map(|s| s.as_str()).collect();
    for expected in [
        "src/lib.rs",
        "src/bin/tool/main.rs",
        "benches/b.rs",
        "tests/it.rs",
        "examples/e.rs",
    ] {
        assert!(
            roots.contains(expected),
            "convention fallback should discover {expected} as a root; got {roots:?}"
        );
    }
    let g = populate_rust(&fs, &cfg, None);
    // Each is its own Root.
    for p in ["src/lib.rs", "src/bin/tool/main.rs", "benches/b.rs"] {
        let fid = file_id(&fs, p).unwrap();
        let s = enclosing_scope(&g, fid, 0).unwrap();
        assert_eq!(
            g.scope(s).unwrap().kind,
            prism::name_resolution::types::ScopeKind::Root,
            "{p} should be a crate Root"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MACRO WILDCARD POISON
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_macro_wildcard_poison() {
    // m!(); f() — a name-introducing macro could emit `fn f`, so a bare `f` in
    // the macro's range is Poisoned; a call BEFORE the macro is unaffected.
    let src = concat!(
        "pub fn f(){}\n",
        "macro_rules! m { () => {}; }\n",
        "fn host(){\n",
        "    f();\n", // BEFORE the macro invocation: resolves to free fn f
        "    m!();\n",
        "    f();\n", // AFTER the macro: poisoned
        "}\n",
    );
    let fs = files(&[("src/lib.rs", src)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let pre = byte_of_nth(src, "f();", 0);
    let res_pre = resolve_bare_at(&g, &fs, 2015, "src/lib.rs", pre, "f", NS_VALUE);
    assert_resolved_item(&res_pre);

    let post = byte_of_nth(src, "f();", 1);
    let res_post = resolve_bare_at(&g, &fs, 2015, "src/lib.rs", post, "f", NS_VALUE);
    assert_eq!(
        res_post.status,
        ResStatus::Poisoned,
        "a bare name after a name-introducing macro must be Poisoned, got {:?}",
        res_post.status
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// REAL GLOB EXPANSION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_real_glob_expands() {
    // use other::*; thing() — other has `pub fn thing`. Deferred globs expand
    // on demand, so the bare call resolves through the glob.
    let lib = "mod other;\nuse crate::other::*;\nfn host(){ thing(); }\n";
    let fs = files(&[("src/lib.rs", lib), ("src/other.rs", "pub fn thing(){}\n")]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "thing()"),
        "thing",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

// ═══════════════════════════════════════════════════════════════════════════
// VISIBILITY — both re-export cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_private_import_of_private_fn_falls_through() {
    // use crate::m::private_fn; private_fn() where private_fn is private → the
    // import's chased path is not visible at the import site → falls through.
    let lib = "mod m;\nuse crate::m::private_fn;\nfn host(){ private_fn(); }\n";
    let fs = files(&[
        ("src/lib.rs", lib),
        ("src/m.rs", "fn private_fn(){}\n"), // private (no pub)
    ]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "private_fn()"),
        "private_fn",
        NS_VALUE,
    );
    assert_ne!(
        res.status,
        ResStatus::Resolved,
        "importing a sibling-module-private fn must not resolve, got {:?}",
        res.status
    );
}

#[test]
fn test_pub_use_facade_reexport_resolves() {
    // mod m { pub fn f(){} } (m private) + `pub use crate::m::f;` at root + a
    // call to the re-exported `f` → resolves to m::f (the facade).
    let lib = "mod m { pub fn f(){} }\npub use crate::m::f;\nfn host(){ f(); }\n";
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "f();"),
        "f",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

// ═══════════════════════════════════════════════════════════════════════════
// ASSOCIATED ITEMS — NOT bare-visible (the critical recall-safety obligation)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_associated_item_not_bare_visible_but_path_reachable() {
    // impl S { fn helper(){} fn run(){ helper() } } — a BARE helper() inside run
    // must NOT resolve to S::helper (needs Self::helper); but T::helper DOES.
    let lib = concat!(
        "pub struct S;\n",
        "impl S {\n",
        "    fn helper(){}\n",
        "    fn run(){ helper() }\n",
        "}\n",
    );
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    // BARE `helper()` inside run → NOT resolved to the associated fn.
    let bare = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "helper() }"),
        "helper",
        NS_VALUE,
    );
    assert_ne!(
        bare.status,
        ResStatus::Resolved,
        "a BARE associated-fn call must NOT resolve (needs Self::); got {:?} ({:?})",
        bare.status,
        bare.candidates
    );

    // `S::helper` (path) DOES resolve.
    let path = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["S", "helper"], NS_VALUE);
    assert_eq!(
        path.status,
        ResStatus::Resolved,
        "S::helper must resolve via a path, got {:?} ({:?})",
        path.status,
        path.candidates
    );
}

#[test]
fn test_bare_call_in_method_routes_to_free_fn_not_assoc() {
    // A free `fn helper` AND an assoc `S::helper` of the same name. A BARE
    // `helper()` inside a method must resolve to the FREE module fn (which IS
    // bare-reachable from inside an impl), NOT the associated fn — and the two
    // must be DISTINCT items (the re-parent-to-module rule, faithful Rust).
    let lib = concat!(
        "pub fn helper() -> u32 { 1 }\n",
        "pub struct S;\n",
        "impl S {\n",
        "    fn helper() -> u32 { 2 }\n",
        "    fn run() -> u32 { helper() }\n",
        "}\n",
    );
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    // The bare `helper()` inside `run` resolves to the FREE fn (a single Item).
    let bare = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "helper() }"),
        "helper",
        NS_VALUE,
    );
    assert_resolved_item(&bare);
    let free_id = match &bare.candidates[0].target {
        Target::Item { id, .. } => *id,
        other => panic!("expected the free-fn Item, got {other:?}"),
    };

    // `S::helper` resolves to a DIFFERENT item (the associated fn).
    let assoc = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["S", "helper"], NS_VALUE);
    assert_eq!(assoc.status, ResStatus::Resolved);
    let assoc_id = match &assoc.candidates[0].target {
        Target::Item { id, .. } => *id,
        other => panic!("expected the assoc-fn Item, got {other:?}"),
    };
    assert_ne!(
        free_id, assoc_id,
        "the bare call must bind the FREE fn, distinct from S::helper"
    );
}

#[test]
fn test_block_locals_inside_impl_method_resolve_as_locals() {
    // Method bodies are re-parented to the enclosing module so associated items
    // are not bare-visible. Locals inside the method body and nested method
    // blocks must still live in the right inner scopes.
    let lib = concat!(
        "pub fn x_call(_: u32){}\n",
        "pub fn y_call(_: u32){}\n",
        "pub struct S;\n",
        "impl S {\n",
        "    fn run(){\n",
        "        let x = 0;\n",
        "        x_call(x);\n",
        "        { let y = 1; y_call(y); }\n",
        "    }\n",
        "}\n",
    );
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let x = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "x);"),
        "x",
        NS_VALUE,
    );
    assert_local(&x);

    let y = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "y);"),
        "y",
        NS_VALUE,
    );
    assert_local(&y);
}

// ═══════════════════════════════════════════════════════════════════════════
// FAIL-OPEN PER-REGION — missing / malformed; Poisoned vs Unmodeled
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_missing_mod_file_poisons_that_region_only() {
    // mod missing; (no file) → resolving INTO missing = Poisoned; the containing
    // module still resolves its own items. No panic.
    let lib = "mod missing;\npub fn here(){}\nfn host(){ here(); }\n";
    let fs = files(&[("src/lib.rs", lib)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    // crate::missing::anything → Poisoned (the module is modeled-but-hazardous).
    let into_missing = resolve_crate_path(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        &["missing", "anything"],
        NS_VALUE,
    );
    assert_eq!(
        into_missing.status,
        ResStatus::Poisoned,
        "a missing mod file must poison resolution INTO that module, got {:?}",
        into_missing.status
    );

    // The containing module still resolves its own `here`.
    let here = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(lib, "here();"),
        "here",
        NS_VALUE,
    );
    assert_resolved_item(&here);
}

#[test]
fn test_parse_failed_target_module_poisons() {
    // mod broken; where src/broken.rs fails to parse (control bytes) → Poisoned.
    let lib = "mod broken;\nfn host(){}\n";
    // Embedded NUL/control bytes push the error rate well past the parse-fail floor.
    let broken: String = "\u{0}\u{1}\u{2}fn\u{0}x\u{0}(\u{0}\u{1}\u{2}".to_string();
    let fs = files(&[
        ("src/lib.rs", lib),
        ("src/broken.rs", Box::leak(broken.into_boxed_str())),
    ]);
    let g = populate_rust(&fs, &convention(&fs), None);
    let res = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["broken", "x"], NS_VALUE);
    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "a parse-failed target module must be Poisoned, got {:?}",
        res.status
    );
}

#[test]
fn test_parse_failed_containing_file_is_unmodeled() {
    // A parse-failed CONTAINING file → Unmodeled: no scopes built for it; its
    // sites are not modeled. The rest of the graph still resolves. No panic.
    let good = "pub fn lives(){}\nfn h(){ lives(); }\n";
    let broken: String = "\u{0}\u{1}\u{2}fn\u{0}gone\u{0}(\u{0}\u{1}".to_string();
    let owned_broken: &'static str = Box::leak(broken.into_boxed_str());
    let fs = files(&[("src/lib.rs", good), ("src/extra.rs", owned_broken)]);
    // Treat BOTH as crate roots (so extra.rs would normally be modeled, but it
    // fails to parse → Unmodeled).
    let cfg = RustCrateConfig {
        edition: 2015,
        crate_roots: vec!["src/lib.rs".to_string(), "src/extra.rs".to_string()],
        workspace_members: vec![],
        dep_renames: BTreeMap::new(),
        lib_path: Some("src/lib.rs".to_string()),
        bin_paths: vec!["src/extra.rs".to_string()],
        edition_uniform: true,
        member_in_repo_deps: BTreeMap::new(),
    };
    let g = populate_rust(&fs, &cfg, None);

    // The good file still resolves its own items.
    let res = resolve_bare_at(
        &g,
        &fs,
        2015,
        "src/lib.rs",
        byte_of(good, "lives();"),
        "lives",
        NS_VALUE,
    );
    assert_resolved_item(&res);

    // The broken file is UNMODELED: no scope exists at its bytes.
    let fid = file_id(&fs, "src/extra.rs").unwrap();
    assert!(
        enclosing_scope(&g, fid, 0).is_none(),
        "a parse-failed containing file must be Unmodeled (no scopes)"
    );
}

#[test]
fn test_only_files_filter_restricts_modeled_containing_files() {
    // only_files restricts which CONTAINING files are walked (build wiring will
    // pass the changed-file set); modules outside the set are not walked but the
    // crate graph still resolves what is in-set.
    let lib = "mod a;\npub fn root_fn(){}\n";
    let fs = files(&[("src/lib.rs", lib), ("src/a.rs", "pub fn afn(){}\n")]);
    let only: BTreeSet<String> = ["src/lib.rs".to_string()].into_iter().collect();
    let g = populate_rust(&fs, &convention(&fs), Some(&only));
    // lib.rs is modeled; its own item resolves.
    let r = resolve_crate_path(&g, &fs, 2015, "src/lib.rs", &["root_fn"], NS_VALUE);
    assert_eq!(r.status, ResStatus::Resolved);
    // a.rs was NOT in the set → its body is Unmodeled (no scope at its bytes).
    let fid = file_id(&fs, "src/a.rs").unwrap();
    assert!(
        enclosing_scope(&g, fid, 0).is_none(),
        "a file outside only_files must not be modeled"
    );
}

// A no-panic smoke test over empty / pathological inputs.
#[test]
fn test_no_panic_on_empty_and_pathological() {
    let empty: BTreeMap<String, ParsedFile> = BTreeMap::new();
    let _ = populate_rust(&empty, &RustCrateConfig::from_convention(&empty), None);

    let weird = files(&[("src/lib.rs", "")]);
    let _ = populate_rust(&weird, &convention(&weird), None);

    // file_id for a missing path is None (no panic).
    assert_eq!(file_id(&weird, "nope.rs"), None);
    assert!(file_id(&weird, "src/lib.rs").is_some());
}

#[test]
fn cross_crate_use_resolves_when_dep_declared() {
    // Crate a (2021) `use b_crate::Foo`; crate b defines `Foo`. The leading
    // `b_crate` resolves via the dep map to b's Root, then Foo to b's Foo item.
    let a_src = "use b_crate::Foo;\npub fn drive() { Foo::m(); }\n";
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        (
            "b/src/lib.rs",
            "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n",
        ),
    ]);
    let mut a_deps = BTreeMap::new();
    a_deps.insert("b_crate".to_string(), "b".to_string());
    let mut member_deps = BTreeMap::new();
    member_deps.insert("a".to_string(), a_deps);
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        member_in_repo_deps: member_deps,
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    // Resolve the `use b_crate::Foo` leading-segment path from a's lib scope.
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    let at_byte = byte_of(a_src, "b_crate");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope at the use site");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b_crate".to_string(), "Foo".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(),
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    assert_eq!(res.status, ResStatus::Resolved, "cross-crate use resolves");
    assert_eq!(res.candidates.len(), 1);
    assert!(
        matches!(res.candidates[0].target, Target::Item { .. }),
        "the full path lands on b's Foo item"
    );
}

#[test]
fn cross_crate_use_declines_when_no_dep() {
    // Crate a does NOT declare a dep on b; the leading `b` segment stays Unresolved.
    let a_src = "pub fn drive() { let _ = 0; }\n";
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        (
            "b/src/lib.rs",
            "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n",
        ),
    ]);
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        // member_in_repo_deps deliberately empty (no declared dependency).
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    let at_byte = byte_of(a_src, "drive");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b".to_string(), "Foo".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(),
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "no declared in-repo dep on b → the fallback declines (keep-all)"
    );
}

#[test]
fn cross_crate_use_local_module_shadows_dep() {
    // Crate a has a LOCAL `mod b_crate;` (with a `Foo`) AND declares a dep on a
    // sibling `b_crate`. The local module rib must win (P2) — never the crate root.
    //
    // MAJOR 1 (spec-review): the anchor MUST be `use_path_2015()` (a `UsePath` anchor),
    // NOT `self_mod()`. The crate fallback's eligibility gate is
    // `matches!(anchor.kind, UsePath | Bare)`, so a `SelfMod` anchor makes the fallback
    // INELIGIBLE — the test would pass vacuously (the fallback never even fires) and
    // would prove nothing about shadowing. With `use_path_2015()` + a 2021 policy the
    // fallback IS eligible, so this genuinely proves the local `mod b_crate` rib
    // shadows the cross-crate fallback. Under 2018+, `RustPolicy::anchor(UsePath, from)`
    // anchors at `enclosing_module(from)` (`rust_policy.rs` UsePath branch) — for a
    // `from` in the top-level `drive` body that enclosing module IS the crate root,
    // where `mod b_crate;` is declared, so `scope_member_lookup` finds the local rib
    // (rib_present == true) BEFORE the fallback and the fallback declines.
    let a_src = "mod b_crate;\npub fn drive() { let _ = 0; }\n";
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        (
            "a/src/b_crate.rs",
            "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n",
        ),
        (
            "b/src/lib.rs",
            "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n",
        ),
    ]);
    let mut a_deps = BTreeMap::new();
    a_deps.insert("b_crate".to_string(), "b".to_string());
    let mut member_deps = BTreeMap::new();
    member_deps.insert("a".to_string(), a_deps);
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        member_in_repo_deps: member_deps,
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    // Resolve from inside crate a's Root scope (the `drive` body byte). Under
    // `use_path_2015` + 2021 the anchor is the enclosing module (= the crate root),
    // where the local `mod b_crate;` rib lives.
    let at_byte = byte_of(a_src, "drive");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        // Resolve the LEADING segment alone — the shadowing decision point. A
        // 2-segment `b_crate::Foo` resolves `b_crate` to the local module but then
        // walks to `Foo` (a `Target::Item`); the single segment resolves to the local
        // module `Target::Scope` itself, which is what must win over the crate
        // fallback (the fallback would instead resolve `b_crate` to b's crate Root).
        &RawPath(vec!["b_crate".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(), // UsePath: the fallback IS eligible, so shadowing is real
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    // The local `mod b_crate` (a's own module) resolves; it is NOT b's crate root.
    // An out-of-line `mod foo;` binds to `Target::Scope` (builder `scope_target`,
    // `builder.rs:513`) — so match the Scope and check its file is the LOCAL module
    // file, never b's crate root file (the fallback would have produced a Scope at
    // `b/src/lib.rs`).
    assert_eq!(res.status, ResStatus::Resolved);
    assert_eq!(res.candidates.len(), 1);
    if let Target::Scope(scope) = res.candidates[0].target {
        let module_file = graph.scope(scope).unwrap().extents[0].file;
        assert_eq!(
            module_file,
            file_id(&fs, "a/src/b_crate.rs").unwrap(),
            "the LOCAL module b_crate must win, never b's crate root (P2)"
        );
    } else {
        panic!(
            "expected the local module scope, got {:?}",
            res.candidates[0].target
        );
    }
}

#[test]
fn cross_crate_use_claimed_but_invisible_local_blocks_fallback() {
    // MAJOR 2 / spec §6.6 (REQUIRED, the load-bearing `rib_present` proof): crate a has
    // a LOCAL rib binding for `b_crate` that is CLAIMED but NOT visible at the use site,
    // AND declares a dep on a sibling crate `b_crate`. The leading segment must stay the
    // LOCAL outcome (Unresolved via claimed-but-invisible — rib_present == true), NEVER
    // b's crate root. This is the discriminating case distinct from the visible-rib test
    // above: there the rib resolves; here it is claimed-but-invisible, surfacing as
    // `Unresolved` *with* a rib present — exactly the status the fallback must NOT
    // convert (engine.rs:233-235 / the `!rib_present` guard).
    //
    // Fixture mechanism (verified against source): an inline `pub(in crate::secret) mod
    // b_crate { ... }` mints a NS_TYPE binding for `b_crate` at the crate root scope with
    // vis = (VIS_PUB_IN, restrict). The populator's `resolve_restrict` is a Phase-1 stub
    // that ALWAYS returns `None` (`rust_populator/walk/mod.rs:271-277`), so the binding's
    // `restrict` is `None`, and `RustPolicy::visible` returns `false` for VIS_PUB_IN with
    // no restrict ("pub(in) with no recorded path → fall through", `rust_policy.rs`). The
    // binding still EXISTS, so `scope_member_lookup_probed` reports `rib_present == true`
    // while `resolve_rib` returns `Unresolved` (claimed-but-invisible) — a robust
    // claimed-but-invisible rib that does not depend on where `from` sits. `secret` need
    // not exist (the stub ignores the path).
    let a_src = concat!(
        "pub(in crate::secret) mod b_crate { pub struct Foo; impl Foo { pub fn m(&self) {} } }\n",
        "pub fn drive() { let _ = 0; }\n",
    );
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        (
            "b/src/lib.rs",
            "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n",
        ),
    ]);
    let mut a_deps = BTreeMap::new();
    a_deps.insert("b_crate".to_string(), "b".to_string());
    let mut member_deps = BTreeMap::new();
    member_deps.insert("a".to_string(), a_deps);
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        member_in_repo_deps: member_deps,
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    let at_byte = byte_of(a_src, "drive");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b_crate".to_string(), "Foo".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(), // UsePath: the fallback WOULD be eligible by anchor/edition
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    // The claimed-but-invisible local rib (rib_present == true) blocks the fallback, so
    // the segment stays the LOCAL Unresolved outcome — it must NOT become b's `Foo` item.
    assert_ne!(
        res.status,
        ResStatus::Resolved,
        "a claimed-but-invisible local rib must block the crate fallback (rib_present), \
         got {:?} ({:?})",
        res.status,
        res.candidates
    );
    assert!(
        !res.candidates
            .iter()
            .any(|c| matches!(c.target, Target::Item { .. })),
        "the fallback must not resolve into b's crate (no b::Foo item), got {:?}",
        res.candidates
    );
}

#[test]
fn cross_crate_use_external_same_name_declines_with_in_repo_member() {
    // MAJOR 3 / spec §6.7 (the key soundness case, end-to-end at the resolver): crate a
    // depends on an EXTERNAL `b` (version-only) while an UNRELATED in-repo workspace
    // member `b` ALSO exists (a real crate Root in the graph). `use b::X` from a must
    // DECLINE (stay Unresolved) — the external `b` is NOT in a's in-repo dep map, so the
    // per-crate dep gate (P3) returns None even though an in-repo crate named `b` has a
    // Root. Modeled at the resolver: `b` IS a crate root in `crate_roots` (so it has a
    // Root scope), but `a`'s `member_in_repo_deps` does NOT record `b` (an external
    // version-only dep is never captured — see Task 2 `external_version_dep_is_not_
    // recorded`). The fallback must not invent a resolution to the in-repo `b` root.
    let a_src = "use b::Thing;\npub fn drive() { let _ = 0; }\n";
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        (
            "b/src/lib.rs",
            "pub struct Thing;\nimpl Thing { pub fn m(&self) {} }\n",
        ),
    ]);
    // a declares NO in-repo dep (its real `b = "1.0"` is external → not recorded). The
    // in-repo member `b` still has a crate Root from `crate_roots`.
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        // member_in_repo_deps deliberately EMPTY for a (external dep, not in-repo).
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    let at_byte = byte_of(a_src, "b::Thing");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b".to_string(), "Thing".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(),
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "an external `b` (not in a's in-repo dep map) must DECLINE even though an \
         in-repo member `b` exists (per-crate dep gate, P3); got {:?} ({:?})",
        res.status,
        res.candidates
    );
}
