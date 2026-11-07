//! Python U/D obligations (#630 slice 5): frozen-reference differential plus marked rows.
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::path::PathBuf;

fn fixtures() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dfg_prune/python");
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "py"))
        .collect();
    paths.sort();
    paths
}

fn pairs(refs: &super::FileRefs) -> std::collections::BTreeSet<(String, usize, usize)> {
    refs.edges
        .iter()
        .map(|e| (e.from.path.to_string(), e.from.line, e.to.line))
        .collect()
}

#[test]
fn python_prune_matrix() {
    let obligations: Vec<serde_json::Value> =
        include_str!("../../../tests/fixtures/dfg_prune/python/obligations.jsonl")
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
    let mut failures = Vec::new();
    for path in fixtures() {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let file = path.file_name().unwrap().to_str().unwrap().to_owned();
        let source = std::fs::read_to_string(&path).unwrap();
        let parsed = ParsedFile::parse(&file, &source, Language::Python).unwrap();
        let before = Producer::file_refs_frozen(&file, &parsed);
        let after = Producer::file_refs(&file, &parsed);
        let (b, a) = (pairs(&before), pairs(&after));
        if std::env::var_os("PRISM_DUMP_PRUNE_BASELINE").is_some() {
            for (p, d, u) in &b {
                println!(
                    "BASELINE {name} {p} {d} {u} kept={}",
                    a.contains(&(p.clone(), *d, *u))
                );
            }
        }
        // Removal-only: surviving pairs are a subset of the baseline, labels unchanged.
        assert!(a.is_subset(&b), "{name}: edges_after must be a subset");
        for (key, label) in &after.labels {
            assert_eq!(before.labels.get(key), Some(label), "{name}: {key:?}");
        }
        assert_eq!(
            before.edges.len() - after.edges.len(),
            after.rd_function_stats.dfg_pruned_write_only
                + after.rd_function_stats.dfg_pruned_dominated,
            "{name}: counters must count removed expanded edges"
        );
        // Invariant (Opus F1): every removed pair that targets a real use keeps another
        // surviving edge from a def of the path into that same use (the dominating write).
        for (p, d, u) in b.difference(&a) {
            let real_use = after
                .uses
                .values()
                .flatten()
                .any(|l| l.path.to_string() == *p && l.line == *u);
            if real_use {
                assert!(
                    a.iter().any(|(p2, d2, u2)| p2 == p && u2 == u && d2 != d),
                    "{name}: removed {p} {d}->{u} left no surviving dominator edge into the use"
                );
            }
        }
        for row in obligations.iter().filter(|r| r["case"] == name) {
            let key = (
                row["path"].as_str().unwrap().to_owned(),
                row["def_line"].as_u64().unwrap() as usize,
                row["use_line"].as_u64().unwrap() as usize,
            );
            if !b.contains(&key) {
                failures.push(format!("inadmissible missing baseline pair: {row}"));
            }
            if a.contains(&key) != row["keep"].as_bool().unwrap() {
                failures.push(format!("{row}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
