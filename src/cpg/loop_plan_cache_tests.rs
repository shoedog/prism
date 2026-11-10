//! #693/#697 slice 2 fold 1: the loop plan is a pure function of one file, so a file holding one admitted and
//! one refused loop tree gives the same labels and the same counters (`dfg_loop_plan` included) on a cold
//! build, a direct full cache hit and a partial hit that rebuilds the file.
use super::*;
use crate::ast::ParsedFile;
use crate::data_flow::test_support::label_snapshot;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

/// `ok` is admitted (a plain multi-line `while`); `bad` holds a `while` whose body starts on its head line
/// (`HeadCollapse`), so it is refused and its owner labelled `cfg_incomplete`.
const SOURCE: &str = "function ok(c) {\n  let z = 1;\n  let x = 0;\n  while (c()) {\n    x = g();\n  }\n  sink(x, z);\n}\nfunction bad(c) {\n  let z = 1;\n  while (c()) x = g();\n  sink(x, z);\n}\n";

fn summary(g: &CodePropertyGraph) -> String {
    format!("{:#?}\n{:#?}", label_snapshot(&g.dfg), g.dfg_label_stats,)
}

#[test]
fn an_admitted_and_a_refused_tree_agree_on_cold_full_hit_and_partial_hit() {
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
    let sources = BTreeMap::from([("a.js".to_string(), SOURCE.to_string())]);
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(p, s)| {
            (
                p.clone(),
                ParsedFile::parse(p, s, Language::JavaScript).unwrap(),
            )
        })
        .collect();
    let cold = CodePropertyGraph::build(&files);
    let plan = &cold.dfg_label_stats.dfg_loop_plan;
    assert_eq!(plan.get("trees"), Some(&2), "{plan:?}");
    assert_eq!(plan.get("admitted"), Some(&1), "{plan:?}");
    assert_eq!(plan.get("owners_refused"), Some(&1), "{plan:?}");
    let hashes = compute_file_hashes(&sources);
    let dir = tempfile::tempdir().unwrap();
    save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(full) = load_cache(&hashes, false, dir.path()) else {
        panic!("cache miss");
    };
    let CacheResult::Hit(loaded) = load_cache(&hashes, false, dir.path()) else {
        panic!("cache miss");
    };
    let partial = CodePropertyGraph::build_incremental(
        loaded.call_graph,
        loaded.dfg,
        &BTreeSet::from(["a.js".to_string()]),
        &files,
        None,
    );
    let expected = summary(&cold);
    assert_eq!(summary(&full), expected, "direct full hit");
    assert_eq!(summary(&partial), expected, "partial hit");
}
