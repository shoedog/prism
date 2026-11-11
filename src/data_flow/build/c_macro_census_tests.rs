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
        .map(|(f, s)| (f.clone(), ParsedFile::parse(f, s, Language::C).unwrap()))
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
