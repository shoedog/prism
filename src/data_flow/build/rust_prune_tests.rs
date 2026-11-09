//! Rust U/D obligations (#630 slice 5, Rust): frozen-reference differential plus marked rows.
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::path::PathBuf;

fn collect(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn fixtures(subdir: &str) -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(subdir);
    let mut paths = Vec::new();
    collect(&root, &mut paths);
    paths.sort();
    paths
}

fn pairs(refs: &super::FileRefs) -> std::collections::BTreeSet<(String, usize, usize)> {
    refs.edges
        .iter()
        .map(|e| (e.from.path.to_string(), e.from.line, e.to.line))
        .collect()
}

fn check_subset(name: &str, source: &str) -> (usize, usize) {
    let parsed = ParsedFile::parse(name, source, Language::Rust).unwrap();
    let file = name.to_owned();
    let before = Producer::file_refs_frozen(&file, &parsed);
    let after = Producer::file_refs(&file, &parsed);
    let (b, a) = (pairs(&before), pairs(&after));
    assert!(a.is_subset(&b), "{name}: edges_after must be a subset");
    for (key, label) in &after.labels {
        assert_eq!(before.labels.get(key), Some(label), "{name}: {key:?}");
    }
    assert_eq!(
        before.edges.len() - after.edges.len(),
        after.rd_function_stats.dfg_pruned_write_only + after.rd_function_stats.dfg_pruned_dominated,
        "{name}: counters must count removed expanded edges"
    );
    (before.edges.len(), after.edges.len())
}

/// `edges_after ⊆ edges_before` over every Rust file of the fixture tree (differential).
#[test]
fn rust_prune_is_removal_only_over_fixture_tree() {
    for path in fixtures("tests/fixtures") {
        if path.to_string_lossy().contains("secbench") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        check_subset(path.file_name().unwrap().to_str().unwrap(), &source);
    }
}

#[test]
fn rust_prune_matrix() {
    let obligations: Vec<serde_json::Value> =
        include_str!("../../../tests/fixtures/dfg_prune/rust/obligations.jsonl")
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
    let mut failures = Vec::new();
    for path in fixtures("tests/fixtures/dfg_prune/rust") {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let file = path.file_name().unwrap().to_str().unwrap().to_owned();
        let source = std::fs::read_to_string(&path).unwrap();
        let parsed = ParsedFile::parse(&file, &source, Language::Rust).unwrap();
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
        check_subset(&file, &source);
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
