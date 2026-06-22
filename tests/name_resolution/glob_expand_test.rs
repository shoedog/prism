use std::collections::BTreeMap;

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::engine::{resolve_path_with_stats, resolve_with_stats, ScopeGraph};
use prism::name_resolution::glob_stats::{GlobExpandSnapshot, GlobExpandStats};
use prism::name_resolution::rust_policy::{RustPolicy, NS_TYPE};
use prism::name_resolution::rust_populator::{
    enclosing_scope, file_id, populate_rust, RustCrateConfig,
};
use prism::name_resolution::types::{
    Anchor, BindTarget, CfgCond, NamespaceId, RawPath, ResStatus, Resolution, ResolveQuery,
    SourceLoc, Target,
};

fn rs(path: &str, src: &str) -> (String, ParsedFile) {
    (
        path.to_string(),
        ParsedFile::parse(path, src, Language::Rust).unwrap(),
    )
}

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs.iter().map(|(p, s)| rs(p, s)).collect()
}

fn byte_of(src: &str, needle: &str) -> usize {
    src.find(needle)
        .unwrap_or_else(|| panic!("needle {needle:?} not found in source"))
}

fn convention(fs: &BTreeMap<String, ParsedFile>) -> RustCrateConfig {
    RustCrateConfig::from_convention(fs)
}

fn graph_from_files(fs: &BTreeMap<String, ParsedFile>) -> ScopeGraph {
    populate_rust(fs, &convention(fs), None)
}

fn resolve_bare_with_glob_stats(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    edition: u16,
    path: &str,
    byte: usize,
    name: &str,
    ns: NamespaceId,
) -> (Resolution, GlobExpandSnapshot) {
    let fid = file_id(fs, path).expect("file id");
    let from = enclosing_scope(g, fid, byte)
        .unwrap_or_else(|| panic!("no enclosing scope at {path}:{byte}"));
    let policy = RustPolicy::new(g, edition);
    let q = ResolveQuery {
        name: name.to_string(),
        ns,
        from,
        at: SourceLoc { file: fid, byte },
        cfg: Default::default(),
        ctx: Default::default(),
    };
    let stats = GlobExpandStats::default();
    let res = resolve_with_stats(g, &q, &policy, &stats);
    (res, stats.snapshot())
}

fn single_file_resolve(
    src: &str,
    marker: &str,
    name: &str,
) -> (
    Resolution,
    GlobExpandSnapshot,
    ScopeGraph,
    BTreeMap<String, ParsedFile>,
) {
    let owned: &'static str = Box::leak(src.to_string().into_boxed_str());
    let fs = files(&[("src/lib.rs", owned)]);
    let g = graph_from_files(&fs);
    assert!(
        g.edges
            .iter()
            .any(|e| matches!(e.to, BindTarget::Pending(_, _))),
        "fixture must populate at least one deferred glob edge"
    );
    let (res, snap) = resolve_bare_with_glob_stats(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, marker),
        name,
        NS_TYPE,
    );
    (res, snap, g, fs)
}

fn resolve_crate_path_with_glob_stats(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    edition: u16,
    root_path: &str,
    at_byte: usize,
    segs: &[&str],
    ns: NamespaceId,
) -> (Resolution, GlobExpandSnapshot) {
    let fid = file_id(fs, root_path).expect("file id");
    let from = enclosing_scope(g, fid, at_byte)
        .unwrap_or_else(|| panic!("no enclosing scope at {root_path}:{at_byte}"));
    let policy = RustPolicy::new(g, edition);
    let path = RawPath(segs.iter().map(|s| s.to_string()).collect());
    let stats = GlobExpandStats::default();
    let res = resolve_path_with_stats(
        g,
        &path,
        ns,
        &Anchor::crate_root(),
        from,
        NS_TYPE,
        &SourceLoc {
            file: fid,
            byte: at_byte,
        },
        &policy,
        &stats,
    );
    (res, stats.snapshot())
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

#[test]
fn glob_expand_single_hop_resolves() {
    let src = "mod m { pub struct S; }\npub use m::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_resolved_item(&res);
    assert_eq!(snap.resolved_l1, 1);
}

#[test]
fn glob_expand_two_hops_resolves() {
    let src = "pub use a::*;\nmod a { pub use b::*; pub mod b { pub struct S; } }\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_resolved_item(&res);
    assert_eq!(snap.resolved_l2, 1);
}

#[test]
fn glob_expand_third_hop_blocked() {
    let src = "pub use a::*;\nmod a { pub use b::*; pub mod b { pub use c::*; pub mod c { pub struct S; } } }\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.depth_exceeded, 1);

    let cycle_src = "pub use a::*;\npub mod a { pub use crate::b::*; }\npub mod b { pub use crate::a::*; }\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(cycle_src, "S>;", "S");
    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.depth_exceeded, 1);
    assert_eq!(snap.cycle, 0);
}

#[test]
fn glob_expand_self_glob_cycle_fails_closed() {
    let src = "pub mod a { pub use crate::a::*; pub fn probe(){ let _: Option<Missing>; } }\n";
    let (res, snap, _, _) = single_file_resolve(src, "Missing>;", "Missing");

    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.cycle, 1);
}

#[test]
fn glob_expand_ambiguous_member_fails_closed() {
    let src =
        "mod m { pub struct S; pub type S = u8; }\npub use m::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.ambiguous, 1);
}

#[test]
fn glob_expand_resolved_set_member_fails_closed() {
    let src = "mod m { #[cfg(target_os = \"linux\")] pub struct S; #[cfg(target_os = \"windows\")] pub struct S; }\npub use m::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.ambiguous, 1);
}

#[test]
fn glob_expand_external_target_fails_closed() {
    let src = "pub use missing::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.external, 1);
}

#[test]
fn glob_expand_multi_target_fails_closed() {
    let lib = "mod m;\npub use m::*;\nfn f(){ let _: Option<S>; }\n";
    let fs = files(&[
        ("src/lib.rs", lib),
        ("src/m.rs", "pub struct S;\n"),
        ("src/m/mod.rs", "pub struct S;\n"),
    ]);
    let g = graph_from_files(&fs);
    let (res, snap) = resolve_bare_with_glob_stats(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(lib, "S>;"),
        "S",
        NS_TYPE,
    );

    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.multi_target, 1);
}

#[test]
fn glob_expand_target_lacks_name_continues() {
    let src = "mod a { pub struct A; }\nmod b { pub struct S; }\npub use a::*;\npub use b::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_resolved_item(&res);
    assert_eq!(snap.resolved_l1, 1);
}

#[test]
fn glob_expand_respects_member_visibility() {
    let src = "mod m { pub struct S; struct Hidden; }\npub use m::*;\nfn f(){ let _: Option<S>; let _: Option<Hidden>; }\n";
    let (public_res, public_snap, g, fs) = single_file_resolve(src, "S>;", "S");
    let (hidden_res, hidden_snap) = resolve_bare_with_glob_stats(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "Hidden>;"),
        "Hidden",
        NS_TYPE,
    );

    assert_resolved_item(&public_res);
    assert_eq!(public_snap.resolved_l1, 1);
    assert_ne!(hidden_res.status, ResStatus::Resolved);
    assert_eq!(hidden_snap.resolved_l1, 0);
}

#[test]
fn glob_expand_skips_private_glob_edge() {
    let src = "pub mod mid { pub mod m { pub struct S; } use m::*; pub fn inside(){ let _: Option<S>; } }\nfn outside(){ let _: Option<S>; }\n";
    let (inside_res, inside_snap, g, fs) = single_file_resolve(src, "S>;", "S");
    let (outside_res, outside_snap) = resolve_crate_path_with_glob_stats(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "outside"),
        &["mid", "S"],
        NS_TYPE,
    );

    assert_resolved_item(&inside_res);
    assert_eq!(inside_snap.resolved_l1, 1);
    assert_eq!(outside_res.status, ResStatus::Unresolved);
    assert_eq!(outside_snap.resolved_l1, 0);
}

#[test]
fn glob_expand_pub_in_unknown_fails_closed() {
    let src = "pub mod facade { pub mod m { pub struct S; } pub(in crate::some) use m::*; pub fn inside(){ let _: Option<S>; } }\nfn outside(){ let _: Option<S>; }\n";
    let (inside_res, inside_snap, g, fs) = single_file_resolve(src, "S>;", "S");
    let (outside_res, outside_snap) = resolve_crate_path_with_glob_stats(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "outside"),
        &["facade", "S"],
        NS_TYPE,
    );

    assert_eq!(inside_res.status, ResStatus::Poisoned);
    assert_eq!(inside_snap.vis_unknown, 1);
    assert_eq!(outside_res.status, ResStatus::Poisoned);
    assert_eq!(outside_snap.vis_unknown, 1);
}

#[test]
fn glob_expand_preserves_conditions() {
    let src = "mod m { pub struct S; }\n#[cfg(feature = \"x\")]\npub use m::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_resolved_item(&res);
    assert_eq!(snap.resolved_l1, 1);
    assert_eq!(
        res.candidates[0].cond,
        CfgCond::Atom("feature".to_string(), Some("x".to_string()))
    );
}

#[test]
fn glob_expand_diamond_resolves_single() {
    let src =
        "mod m { pub struct S; }\npub use m::*;\npub use m::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_resolved_item(&res);
    assert_eq!(snap.resolved_l1, 2);
}

#[test]
fn glob_expand_distinct_targets_two_globs() {
    let src = "mod a { pub struct S; }\nmod b { pub struct S; }\npub use a::*;\npub use b::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");

    assert_eq!(res.status, ResStatus::Ambiguous);
    assert_eq!(snap.resolved_l1, 2);
    assert_ne!(res.candidates[0].target, res.candidates[1].target);
}
