//! #630 slice 6a: the C/C++ macro census digest is persisted with the DFG, scoped builds read
//! it from the whole repository, and cold, full-hit and partial-hit builds agree when a
//! header's `#define` changes. The census changes no edge yet, so parity is judged on the
//! edge set, the label stats and the persisted digest (a stale digest is the visible trace of
//! a disabled guard).
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::cpg::CodePropertyGraph;
use crate::cpg_cache::{self, CacheResult};
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

const OWNER: &str = "int f(void) {\n    int x = 1;\n    x = NEXT;\n    return x;\n}\n";
const V1: &str = "#define NEXT 2\n";
const V2: &str = "#define NEXT (x + 1)\n";

fn sources(header: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("a.c".to_owned(), OWNER.to_owned()),
        ("m.h".to_owned(), header.to_owned()),
    ])
}

fn parse(src: &BTreeMap<String, String>) -> BTreeMap<String, ParsedFile> {
    src.iter()
        .map(|(f, s)| {
            let lang = if f.ends_with(".cc") {
                Language::Cpp
            } else {
                Language::C
            };
            (f.clone(), ParsedFile::parse(f, s, lang).unwrap())
        })
        .collect()
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
fn digest_is_built_into_every_build_shape() {
    let all = parse(&sources(V2));
    let digest = CMacroCensus::from_files(&all).digest();
    assert!(!digest.is_empty());
    assert_eq!(
        CodePropertyGraph::build(&all).dfg.c_macro_census_digest(),
        digest
    );
    // Scoped build: only a.c is in scope, the census is the whole repository's.
    let subset: BTreeMap<String, ParsedFile> = all
        .iter()
        .filter(|(k, _)| *k == "a.c")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let scoped = CodePropertyGraph::build_scoped_subset(&subset, &all, None);
    assert_eq!(
        scoped.dfg.c_macro_census_digest(),
        digest,
        "scoped census equals full"
    );
    let full = CodePropertyGraph::build(&all);
    assert_eq!(
        edge_keys(&scoped.dfg),
        edge_keys(&full.dfg)
            .into_iter()
            .filter(|k| k.0 == "a.c")
            .collect()
    );
    // Without C/C++ macros the digest is empty.
    let none = parse(&BTreeMap::from([("a.c".to_owned(), OWNER.to_owned())]));
    assert!(CodePropertyGraph::build(&none)
        .dfg
        .c_macro_census_digest()
        .is_empty());
}

#[test]
fn cache_parity_cold_full_partial_when_a_header_define_changes() {
    let mut src = sources(V1);
    let files = parse(&src);
    let cold = CodePropertyGraph::build(&files);
    let hashes = cpg_cache::compute_file_hashes(&src);
    let dir = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(edge_keys(&cold.dfg), edge_keys(&hit.dfg));
    assert_eq!(
        hit.dfg.c_macro_census_digest(),
        CMacroCensus::from_files(&files).digest()
    );

    // m.h changes its #define; a.c is unchanged and keeps a stale DFG unless the partial
    // hit notices that the census moved.
    src.insert("m.h".into(), V2.into());
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
    assert_eq!(changed_files, BTreeSet::from(["m.h".to_owned()]));
    let fresh = CMacroCensus::from_files(&edited).digest();
    assert_ne!(
        cached_dfg.c_macro_census_digest(),
        fresh,
        "built under another census"
    );
    let incremental = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited,
        None,
    );
    let rebuilt = CodePropertyGraph::build(&edited);
    assert_eq!(edge_keys(&incremental.dfg), edge_keys(&rebuilt.dfg));
    assert_eq!(incremental.dfg_label_stats, rebuilt.dfg_label_stats);
    assert_eq!(
        incremental.dfg.rd_function_stats(),
        rebuilt.dfg.rd_function_stats()
    );
    // RED when the guard is disabled: the incremental graph would carry the stale digest.
    assert_eq!(incremental.dfg.c_macro_census_digest(), fresh);
    assert_eq!(
        incremental.dfg.c_macro_census_digest(),
        rebuilt.dfg.c_macro_census_digest()
    );

    // An edit that leaves the census alone takes the incremental path and still agrees.
    let mut same = sources(V1);
    same.insert("m.h".into(), "#define NEXT 3\n".into());
    let (hashes, files) = (cpg_cache::compute_file_hashes(&same), parse(&same));
    let base = parse(&sources(V1));
    let dir2 = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(
        &CodePropertyGraph::build(&base),
        &cpg_cache::compute_file_hashes(&sources(V1)),
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
        cached_dfg.c_macro_census_digest(),
        CMacroCensus::from_files(&files).digest()
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
}

// ---- #829 row cz1: the bracket facts are in the digest ----

const MARK_OWNER: &str = "int f(void) {\n    int x = 1;\n    MARK;\n    x = 2;\n    return x;\n}\n";
const MARK_V1: &str = "#define MARK {}\n";
const MARK_V2: &str = "#define MARK {\n";

fn mark_sources(header: &str) -> BTreeMap<String, String> {
    mark_sources_in("a.c", header)
}

fn mark_sources_in(owner_file: &str, header: &str) -> BTreeMap<String, String> {
    mark_sources_with(owner_file, MARK_OWNER, header)
}

fn mark_sources_with(owner_file: &str, owner: &str, header: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        (owner_file.to_owned(), owner.to_owned()),
        ("m.h".to_owned(), header.to_owned()),
    ])
}

/// The owner of the loose-comma rows: `MARK` is an argument of an identity wrapper, where a
/// loose macro re-splits the argument list on a rescan (G9, `Off::MacroArgLoose`); a bare
/// `MARK;` is no hazard of the amended design, so a loose-comma edit moves no pruning there.
const MARK_OWNER_ARG: &str =
    "int f(void) {\n    int x = 1;\n    ID(MARK);\n    x = 2;\n    return x;\n}\n";
const ID_DEF: &str = "#define ID(v) v\n";

fn has_x_2_to_5(g: &DataFlowGraph, owner_file: &str) -> bool {
    edge_keys(g).contains(&(owner_file.to_owned(), 2, 5, "x".to_owned()))
}

/// `m.h` goes from the `v1` body to the `v2` body: a.c is unchanged, so the partial hit must
/// notice that the census moved and rebuild in full (the pair `x@2 -> x@5` appears).
fn cz_parity(v1: &str, v2: &str, what: &str) {
    cz_parity_in("a.c", v1, v2, what, true, false);
}

/// `first_prunes`: the first census prunes the overwritten def and the edited one keeps it
/// (the forward direction); false is the reverse edit (the first census keeps it, the edited
/// one prunes it), built from a newly saved cache of the first state. `digest` also asserts the
/// census digest moved (the tentative-design half, #829 TEST-FORM section 5: the digest layout
/// belongs to the design, not to the cache-parity requirement, so the behaviour rows run with
/// it off and the `*_digest_half` rows are ignored until the design lands). What the rows
/// require is "incremental equals fresh" and "a full hit equals cold". Which edit keeps the
/// pair is the design's claim (the `MARK` bodies are not valid programs, so the oracle has no
/// truth for them): asserted only by the digest halves, and the first state's pruning is a
/// recall guide (printed, not asserted).
fn cz_parity_in(
    owner_file: &str,
    v1: &str,
    v2: &str,
    what: &str,
    first_prunes: bool,
    digest: bool,
) {
    cz_parity_with(owner_file, MARK_OWNER, v1, v2, what, first_prunes, digest);
}

fn cz_parity_with(
    owner_file: &str,
    owner: &str,
    v1: &str,
    v2: &str,
    what: &str,
    first_prunes: bool,
    digest: bool,
) {
    let src = mark_sources_with(owner_file, owner, v1);
    let files = parse(&src);
    let cold = CodePropertyGraph::build(&files);
    if has_x_2_to_5(&cold.dfg, owner_file) == first_prunes {
        eprintln!(
            "GUIDE {what}: the first build {} the overwritten def",
            if first_prunes { "keeps" } else { "prunes" }
        );
    }
    let hashes = cpg_cache::compute_file_hashes(&src);
    let dir = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(edge_keys(&cold.dfg), edge_keys(&hit.dfg));

    let edited_src = mark_sources_with(owner_file, owner, v2);
    let edited = parse(&edited_src);
    let hashes = cpg_cache::compute_file_hashes(&edited_src);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&hashes, false, dir.path())
    else {
        panic!("expected partial hit");
    };
    assert_eq!(changed_files, BTreeSet::from(["m.h".to_owned()]));
    let fresh = CMacroCensus::from_files(&edited).digest();
    if digest {
        assert_ne!(
            cached_dfg.c_macro_census_digest(),
            fresh,
            "{what}: built under another census: the digest must carry the changed fact"
        );
    }
    let incremental = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited,
        None,
    );
    let rebuilt = CodePropertyGraph::build(&edited);
    assert_eq!(edge_keys(&incremental.dfg), edge_keys(&rebuilt.dfg));
    if digest && first_prunes {
        assert!(
            has_x_2_to_5(&rebuilt.dfg, owner_file),
            "{what}: after the edit the pair must be kept (the design's claim)"
        );
    }
    assert_eq!(
        has_x_2_to_5(&incremental.dfg, owner_file),
        has_x_2_to_5(&rebuilt.dfg, owner_file),
        "{what}: incremental equals fresh"
    );
    if digest {
        assert_eq!(incremental.dfg.c_macro_census_digest(), fresh);
    }
}

#[test]
fn cz1_cache_parity_when_a_header_define_becomes_unbalanced() {
    cz_parity(MARK_V1, MARK_V2, "unbalanced");
}

/// G-terra-15 / G-sol-13 (round 1): only `loose_comma` changes (`(1, 2)` to `1, 2`): no
/// bracket, free name, control or poison fact moves, so a digest that omits the new byte
/// replays the old pruning.
#[test]
fn cz2_cache_parity_when_a_header_define_gains_a_loose_comma() {
    cz_parity_with(
        "a.c",
        MARK_OWNER_ARG,
        &format!("{ID_DEF}#define MARK (1, 2)\n"),
        &format!("{ID_DEF}#define MARK 1, 2\n"),
        "loose comma",
        true,
        false,
    );
}

// ---- #829 chain A round 2, G2-terra-04: the wrapper amendment's input is the free-name set ----

const ALIAS_PLAIN: &str = "#define MARK ALIAS\n#define ALIAS ordinary_name\n";
const ALIAS_LISTED: &str = "#define MARK ALIAS\n#define ALIAS pthread_cleanup_push\n";

/// Only the transitive free-name set of `MARK` changes (`ordinary_name` to a listed block
/// macro): the bracket and comma facts of every definition stay false. The owner must be
/// pruned before the header edit and keep `x@2 -> x@5` after it, in the incremental build
/// exactly as in a fresh one. `.c` and `.cc`.
#[test]
fn cz3_cache_parity_when_a_header_alias_starts_naming_a_listed_macro() {
    for owner in ["a.c", "a.cc"] {
        cz_parity_in(owner, ALIAS_PLAIN, ALIAS_LISTED, owner, true, false);
    }
}

/// The reverse edit, from a newly saved cache of the listed state, against a clean build.
#[test]
fn cz3_cache_parity_when_a_header_alias_stops_naming_a_listed_macro() {
    for owner in ["a.c", "a.cc"] {
        cz_parity_in(owner, ALIAS_LISTED, ALIAS_PLAIN, owner, false, false);
    }
}

/// The digest halves of cz1 to cz3: the census digest must move with the bracket and comma
/// facts and with the free-name set. Quarantined with the facts they hash.
#[test]
#[ignore = "needs the C3 gates (#829 PLAN-v2 task 6): the kept-after-edit half reads G7"]
fn cz1_digest_half() {
    cz_parity_in("a.c", MARK_V1, MARK_V2, "unbalanced", true, true);
}

#[test]
#[ignore = "needs the C3 gates (#829 PLAN-v2 task 6): the kept-after-edit half reads G7"]
fn cz2_digest_half() {
    cz_parity_with(
        "a.c",
        MARK_OWNER_ARG,
        &format!("{ID_DEF}#define MARK (1, 2)\n"),
        &format!("{ID_DEF}#define MARK 1, 2\n"),
        "loose comma",
        true,
        true,
    );
}

#[test]
#[ignore = "needs the C3 gates (#829 PLAN-v2 task 6): the kept-after-edit half reads G7"]
fn cz3_digest_half() {
    for owner in ["a.c", "a.cc"] {
        cz_parity_in(owner, ALIAS_PLAIN, ALIAS_LISTED, owner, true, true);
        cz_parity_in(owner, ALIAS_LISTED, ALIAS_PLAIN, owner, false, true);
    }
}

/// Premise of the pair above: the two header states differ in no bracket or comma fact, so the
/// cache must be moved by the free-name set alone (a digest that tracked only the new facts
/// would replay stale pruning).
#[test]
fn cz3_premise_only_the_free_names_differ() {
    let facts = |header: &str| {
        let c = CMacroCensus::from_files(&parse(&mark_sources(header)));
        let m = c.get("MARK").unwrap().clone();
        (m.unbalanced, m.loose_comma, m.free.clone())
    };
    let (u1, l1, f1) = facts(ALIAS_PLAIN);
    let (u2, l2, f2) = facts(ALIAS_LISTED);
    assert_eq!((u1, l1, u2, l2), (false, false, false, false));
    assert_ne!(f1, f2, "MARK's transitive free set must change");
    assert_ne!(
        CMacroCensus::from_files(&parse(&mark_sources(ALIAS_PLAIN))).digest(),
        CMacroCensus::from_files(&parse(&mark_sources(ALIAS_LISTED))).digest()
    );
}
