//! JS/TS/TSX U/D obligations (#630 slice 5): frozen-reference differential plus marked rows.
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dfg_prune/js")
}

fn fixtures() -> Vec<(PathBuf, Language)> {
    let mut paths: Vec<_> = std::fs::read_dir(root())
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter_map(|p| {
            let lang = match p.extension()?.to_str()? {
                "js" => Language::JavaScript,
                "ts" => Language::TypeScript,
                "tsx" => Language::Tsx,
                _ => return None,
            };
            Some((p, lang))
        })
        .collect();
    paths.sort();
    paths
}

type Pair = (String, usize, usize);

fn pairs(refs: &super::FileRefs) -> BTreeSet<Pair> {
    refs.edges
        .iter()
        .map(|e| (e.from.path.to_string(), e.from.line, e.to.line))
        .collect()
}

#[test]
fn js_prune_matrix() {
    let obligations: Vec<serde_json::Value> = include_str!(
        "../../../tests/fixtures/dfg_prune/js/obligations.jsonl"
    )
    .lines()
    .map(|l| serde_json::from_str(l).unwrap())
    .collect();
    let mut failures = Vec::new();
    for (path, lang) in fixtures() {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let file = path.file_name().unwrap().to_str().unwrap().to_owned();
        let source = std::fs::read_to_string(&path).unwrap();
        let parsed = ParsedFile::parse(&file, &source, lang).unwrap();
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
