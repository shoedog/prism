//! Fold 3 (#630 slice 5, Rust): the repo-wide macro census reaches U and D across files,
//! and the cache keeps cold, full-hit and partial-hit builds in agreement.
use crate::ast::ParsedFile;
use crate::call_graph::rust::macro_args::{RustMacroCensus, Verdict};
use crate::cpg::CodePropertyGraph;
use crate::cpg_cache::{self, CacheResult};
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

const OWNER: &str = "fn gen_u() -> i32 {\n    let mut x = 1;\n    install_get!(x);\n    x = get!() + 1;\n    x\n}\n";
const GENERATING: &str =
    "macro_rules! install_get { ($v:ident) => { macro_rules! get { () => { $v }; } }; }\n";
const CLEAN: &str = "macro_rules! install_get { ($v:ident) => { $v }; }\n";

fn parse(sources: &BTreeMap<String, String>) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(f, s)| (f.clone(), ParsedFile::parse(f, s, Language::Rust).unwrap()))
        .collect()
}

fn sources(b: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("a.rs".to_owned(), OWNER.to_owned()),
        ("b.rs".to_owned(), b.to_owned()),
    ])
}

/// The def at line 2 of `a.rs` reaches the use anchor at line 4.
fn has_pair(g: &DataFlowGraph) -> bool {
    g.edges().iter().any(|e| {
        e.from.file == "a.rs"
            && e.from.path.to_string() == "x"
            && e.from.line == 2
            && e.to.line == 4
    })
}

fn edge_keys(g: &DataFlowGraph) -> BTreeSet<(String, usize, usize, String)> {
    g.edges()
        .iter()
        .map(|e| {
            (
                e.from.file.clone(),
                e.from.line,
                e.to.line,
                e.from.path.to_string(),
            )
        })
        .collect()
}

#[test]
fn census_is_repo_wide_a_generator_in_another_file_keeps() {
    let generating = DataFlowGraph::build(&parse(&sources(GENERATING)));
    assert!(has_pair(&generating), "generator in b.rs refuses x in a.rs");
    // The same owner with the generator made clean prunes: the verdict crosses files.
    let clean = DataFlowGraph::build(&parse(&sources(CLEAN)));
    assert!(!has_pair(&clean), "a clean in-repo macro is Allowed");
    // Alone, the owner's `install_get!` is an unknown spelling: Refused.
    let alone = DataFlowGraph::build(&parse(&BTreeMap::from([(
        "a.rs".to_owned(),
        OWNER.to_owned(),
    )])));
    assert!(has_pair(&alone));
}

#[test]
fn scoped_build_reads_the_census_from_the_whole_repository() {
    let all = parse(&sources(GENERATING));
    let subset: BTreeMap<String, ParsedFile> = all
        .iter()
        .filter(|(k, _)| *k == "a.rs")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let scoped = CodePropertyGraph::build_scoped_subset(&subset, &all, None);
    assert!(
        has_pair(&scoped.dfg),
        "scoped build refuses as the full build does"
    );
    let full = CodePropertyGraph::build(&all);
    assert_eq!(
        edge_keys(&scoped.dfg),
        edge_keys(&full.dfg)
            .into_iter()
            .filter(|k| k.0 == "a.rs")
            .collect()
    );
}

#[test]
fn census_verdicts_and_digest() {
    let files = parse(&BTreeMap::from([(
        "m.rs".to_owned(),
        "macro_rules! clean { ($v:ident) => { $v }; }\n\
         macro_rules! generator { ($v:ident) => { macro_rules! g { () => { $v }; } }; }\n\
         macro_rules! via { ($m:ident) => { $m!(x) }; }\n\
         macro_rules! outer { ($v:ident) => { generator!($v) }; }\n\
         macro_rules! cycle_a { () => { cycle_b!() }; }\n\
         macro_rules! cycle_b { () => { cycle_a!() }; }\n\
         macro_rules! dup { () => { 1 }; }\n\
         macro_rules! dup { () => { macro_rules! z { () => {}; } }; }\n"
            .to_owned(),
    )]));
    let c = RustMacroCensus::from_files(&files);
    let v = |q: Option<&str>, n: &str| c.verdict(q, n);
    assert_eq!(v(None, "clean"), Verdict::Allowed);
    assert_eq!(v(Some("crate"), "clean"), Verdict::Allowed);
    assert_eq!(v(Some("$crate"), "clean"), Verdict::Allowed);
    assert_eq!(v(None, "generator"), Verdict::Refused);
    assert_eq!(v(None, "via"), Verdict::Refused);
    assert_eq!(v(None, "outer"), Verdict::Refused);
    assert_eq!(
        v(None, "cycle_a"),
        Verdict::Allowed,
        "a cycle of clean macros stays clean"
    );
    assert_eq!(v(None, "dup"), Verdict::Refused);
    assert_eq!(v(None, "println"), Verdict::Allowed);
    assert_eq!(v(Some("std"), "println"), Verdict::Allowed);
    assert_eq!(v(Some("std"), "include"), Verdict::Refused);
    assert_eq!(v(None, "json"), Verdict::Refused);
    assert_eq!(v(Some("serde_json"), "json"), Verdict::Refused);
    assert_eq!(v(None, "paste"), Verdict::Refused);
    // Digest: stable, empty only for an empty census, and verdict-relevant facts move it.
    assert_eq!(c.digest(), RustMacroCensus::from_files(&files).digest());
    assert!(!c.digest().is_empty());
    assert!(RustMacroCensus::default().digest().is_empty());
    let shadowed = parse(&BTreeMap::from([(
        "s.rs".to_owned(),
        "use pretty_assertions::assert_eq;\n".to_owned(),
    )]));
    let s = RustMacroCensus::from_files(&shadowed);
    assert_eq!(
        s.verdict(None, "assert_eq"),
        Verdict::Refused,
        "an external use shadows the std spelling"
    );
    assert_eq!(s.verdict(None, "assert"), Verdict::Allowed);
    let ext = parse(&BTreeMap::from([(
        "e.rs".to_owned(),
        "#[macro_use]\nextern crate foo;\n".to_owned(),
    )]));
    assert_eq!(
        RustMacroCensus::from_files(&ext).verdict(None, "assert"),
        Verdict::Refused
    );
    assert_eq!(
        RustMacroCensus::from_files(&ext).verdict(Some("std"), "assert"),
        Verdict::Allowed
    );
}

#[test]
fn macro_census_cache_parity_cold_full_partial() {
    let mut src = sources(CLEAN);
    let files = parse(&src);
    let cold = CodePropertyGraph::build(&files);
    assert!(!has_pair(&cold.dfg), "v1 prunes under the clean macro");
    let hashes = cpg_cache::compute_file_hashes(&src);
    let dir = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(
        edge_keys(&cold.dfg),
        edge_keys(&hit.dfg),
        "full hit replays the cold build"
    );
    assert_eq!(
        hit.dfg.rust_macro_census_digest(),
        RustMacroCensus::from_files(&files).digest()
    );
    // b.rs flips from non-generating to generating; a.rs is unchanged and keeps a stale DFG
    // unless the partial hit notices the census moved.
    src.insert("b.rs".into(), GENERATING.into());
    let edited = parse(&src);
    let hashes = cpg_cache::compute_file_hashes(&src);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&hashes, false, dir.path())
    else {
        panic!("expected partial hit");
    };
    assert_eq!(changed_files, BTreeSet::from(["b.rs".to_owned()]));
    assert_ne!(
        cached_dfg.rust_macro_census_digest(),
        RustMacroCensus::from_files(&edited).digest(),
        "the cached DFG was built under another census"
    );
    let incremental = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited,
        None,
    );
    let rebuilt = CodePropertyGraph::build(&edited);
    assert!(has_pair(&rebuilt.dfg));
    assert!(
        has_pair(&incremental.dfg),
        "the partial hit refused x in the unchanged a.rs"
    );
    assert_eq!(edge_keys(&incremental.dfg), edge_keys(&rebuilt.dfg));
    assert_eq!(incremental.dfg_label_stats, rebuilt.dfg_label_stats);
    assert_eq!(
        incremental.dfg.rd_function_stats(),
        rebuilt.dfg.rd_function_stats()
    );
    assert_eq!(
        incremental.dfg.rust_macro_census_digest(),
        rebuilt.dfg.rust_macro_census_digest()
    );
    // An edit that leaves the census alone takes the incremental path and still agrees.
    let mut same = sources(CLEAN);
    same.insert(
        "b.rs".into(),
        "macro_rules! install_get { ($v:ident) => { $v + 0 }; }\n".into(),
    );
    let (hashes, files) = (cpg_cache::compute_file_hashes(&same), parse(&same));
    let base = parse(&sources(CLEAN));
    let base_hashes = cpg_cache::compute_file_hashes(&sources(CLEAN));
    let dir2 = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(
        &CodePropertyGraph::build(&base),
        &base_hashes,
        false,
        dir2.path(),
    )
    .unwrap();
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&hashes, false, dir2.path())
    else {
        panic!("expected partial hit");
    };
    assert_eq!(
        cached_dfg.rust_macro_census_digest(),
        RustMacroCensus::from_files(&files).digest()
    );
    let incremental = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &files,
        None,
    );
    assert_eq!(
        edge_keys(&incremental.dfg),
        edge_keys(&CodePropertyGraph::build(&files).dfg)
    );
    assert!(!has_pair(&incremental.dfg));
}

#[test]
fn keyword_spelled_macros_are_invocations_in_soup() {
    // `gen` and `try` are anonymous keyword tokens in token soup; a user macro of that name
    // is still an unknown spelling and refuses the names it receives (the wrapper is std).
    let refused = |src: &str| {
        let parsed = ParsedFile::parse("k.rs", src, Language::Rust).unwrap();
        let owner = parsed.all_functions().into_iter().next().unwrap();
        let census = RustMacroCensus::of_file(&parsed);
        super::rust_prune::scan_macros(&parsed, owner, &census).refused
    };
    assert!(refused("fn f() { let x = 1; assert!(gen!(x)); }\n").contains("x"));
    assert!(refused("fn f() { let x = 1; assert!(ext::try!(x)); }\n").contains("x"));
    assert!(refused("fn f() { let x = 1; assert!(try!(x)); }\n").contains("x"));
    assert!(refused("fn f() { let x = 1; assert!(x == 1); }\n").is_empty());
}
