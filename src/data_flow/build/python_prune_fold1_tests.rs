//! Python U/D round-1 fold (#630 slice 5, PR #713): the dominator-edge invariant and
//! cache custody with positive U and D counts.
use super::lexical_prune::LexicalFacts;
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::path::PathBuf;

fn python_files() -> Vec<PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap();
            if name.starts_with("f-")
                || name.starts_with("F-private")
                || name == "private-corpus"
                || name == "secbench"
            {
                continue;
            }
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "py") {
                out.push(path);
            }
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    walk(&root.join("tests/fixtures"), &mut out);
    walk(&root.join("eval/fixtures/python"), &mut out);
    out.sort();
    out
}

/// For every pair D removes (identifier paths), an edge into the same use from a def
/// located in the dominating write's statement must survive. A mismatch between the
/// def location D judges and the window it proves would show up here.
#[test]
fn every_d_removal_leaves_an_edge_from_the_dominating_write() {
    crate::build_pool::install(|| {
        let (mut files, mut checked) = (0, 0);
        for path in python_files() {
            let file = path.to_str().unwrap().to_owned();
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            let parsed = ParsedFile::parse(&file, &source, Language::Python).unwrap();
            let before = Producer::file_refs_frozen(&file, &parsed);
            let after = Producer::file_refs(&file, &parsed);
            files += 1;
            for edge in before.edges.iter().filter(|e| !after.edges.contains(e)) {
                let Some(owner) = parsed.all_functions().into_iter().find(|f| {
                    f.start_byte() <= edge.from.start_byte && edge.to.end_byte <= f.end_byte()
                }) else {
                    continue;
                };
                let lines = (owner.start_position().row + 1..=owner.end_position().row + 1)
                    .collect::<std::collections::BTreeSet<_>>();
                let lvalues = parsed.assignment_lvalue_spans_on_lines(&owner, &lines);
                let facts = LexicalFacts::new(&parsed, owner, &lvalues, &[]);
                let Some(writes) = facts.dominator_writes(&edge.to.path, edge.to.line) else {
                    continue; // U removal, or D proves nothing here
                };
                checked += 1;
                let survives = after.edges.iter().any(|e| {
                    e.to == edge.to
                        && e.from.path == edge.to.path
                        && writes
                            .iter()
                            .any(|(s, t)| *s <= e.from.start_byte && e.from.end_byte <= *t)
                });
                assert!(survives, "{file}: no dominator edge for removed {edge:?}");
            }
        }
        assert!(
            files > 20 && checked > 10,
            "walk selected {files}/{checked}"
        );
    });
}

fn python_sources(extra: &str) -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::from([
        (
            "a.py".to_owned(),
            format!("def a():\n    x = 1\n    x = 2\n    use(x)\n{extra}"),
        ),
        (
            "b.py".to_owned(),
            "def b():\n    y = 3\n    y = 4\n    use(y)\n    z = 1; z = 2\n    use(z)\n".to_owned(),
        ),
    ])
}

/// Positive U and D counts agree across cold, full-hit and partial-hit builds, and
/// editing one file drops only its own contribution.
#[test]
fn python_prune_stats_cache_and_incremental_custody() {
    use crate::cpg::CodePropertyGraph;
    use crate::cpg_cache::{self, CacheResult};
    use std::collections::{BTreeMap, BTreeSet};
    let mut sources = python_sources("");
    let parse = |sources: &BTreeMap<String, String>| {
        sources
            .iter()
            .map(|(f, s)| {
                (
                    f.clone(),
                    ParsedFile::parse(f, s, Language::Python).unwrap(),
                )
            })
            .collect()
    };
    let files = parse(&sources);
    let cold = CodePropertyGraph::build(&files);
    assert!(cold.dfg_label_stats.dfg_pruned_write_only > 0);
    assert!(cold.dfg_label_stats.dfg_pruned_dominated > 0);
    let hashes = cpg_cache::compute_file_hashes(&sources);
    let dir = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(cold.dfg_label_stats, hit.dfg_label_stats);
    assert_eq!(cold.dfg.rd_function_stats(), hit.dfg.rd_function_stats());
    sources.insert("a.py".into(), "def a():\n    return 1\n".into());
    let edited = parse(&sources);
    let hashes = cpg_cache::compute_file_hashes(&sources);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&hashes, false, dir.path())
    else {
        panic!("expected partial hit");
    };
    assert_eq!(changed_files, BTreeSet::from(["a.py".into()]));
    let incremental = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited,
        None,
    );
    let rebuilt = CodePropertyGraph::build(&edited);
    assert_eq!(incremental.dfg_label_stats, rebuilt.dfg_label_stats);
    assert_eq!(
        incremental.dfg.rd_function_stats(),
        rebuilt.dfg.rd_function_stats()
    );
    let stats = incremental.dfg.rd_function_stats();
    assert_eq!(stats["a.py"].dfg_pruned_dominated, 0);
    assert!(stats["b.py"].dfg_pruned_write_only > 0);
    assert!(stats["b.py"].dfg_pruned_dominated > 0);
}

/// The survivor-cap rule (#634) applies to Python: pruned candidates do not count
/// toward the cap, and replay at and above the surviving count is identical.
#[test]
fn python_prune_cap_counts_survivors_and_replays() {
    let source = format!(
        "def f(x):\n    x = 1\n    consume(x)\n    x = 2\n    consume(x)\n{}",
        "    g()\n".repeat(4100)
    );
    let parsed = ParsedFile::parse("a.py", &source, Language::Python).unwrap();
    let file = parsed.path.clone();
    let before = Producer::file_refs_frozen_with_cap(&file, &parsed, None);
    let single = Producer::file_refs_with_cap(&file, &parsed, None);
    let s = &single.rd_function_stats;
    assert_eq!(s.functions_over_cap, 1);
    assert!(s.dfg_pruned_write_only > 0 && s.dfg_pruned_dominated > 0);
    let count = single.edges.len();
    assert!(count > 0 && count < before.edges.len());
    assert_eq!(
        before.edges.len() - count,
        s.dfg_pruned_write_only + s.dfg_pruned_dominated
    );
    for cap in [count, count + 1] {
        let replay = Producer::file_refs_with_cap(&file, &parsed, Some(cap));
        assert_eq!(replay.edges, single.edges);
        assert_eq!(replay.labels, single.labels);
        assert_eq!(replay.rd_function_stats, single.rd_function_stats);
    }
    let capped = Producer::file_refs_with_cap(&file, &parsed, Some(count - 1));
    assert!(capped.edges.is_empty());
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 1);
    assert_eq!(capped.rd_function_stats.fallback_edges_capped, count);
    assert_eq!(
        capped.rd_function_stats.dfg_pruned_dominated,
        s.dfg_pruned_dominated
    );
}
