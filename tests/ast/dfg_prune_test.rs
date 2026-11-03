//! Independent complete edge-set goldens for the Go lexical producer matrix.
use crate::common::*;
use std::path::Path;

#[test]
fn go_prune_complete_edge_sets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dfg_prune/go");
    let expected: Vec<serde_json::Value> = std::fs::read_to_string(root.join("expected.jsonl"))
        .expect("complete edge golden must be present")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut cases: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "go"))
        .collect();
    cases.sort();
    let mut failures = Vec::new();
    for path in cases {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let file = path.file_name().unwrap().to_str().unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        let parsed = ParsedFile::parse(file, &source, Language::Go).unwrap();
        let cpg = CodePropertyGraph::build(&BTreeMap::from([(file.to_owned(), parsed)]));
        let actual: BTreeSet<_> = cpg
            .dfg
            .edges()
            .iter()
            .map(|edge| {
                (
                    edge.from.function.clone(),
                    edge.from.path.to_string(),
                    edge.from.line,
                    edge.to.line,
                )
            })
            .collect();
        let wanted: BTreeSet<_> = expected
            .iter()
            .filter(|r| r["case"] == name && r["keep"] == true)
            .map(|r| {
                (
                    r["function"].as_str().unwrap().to_owned(),
                    r["path"].as_str().unwrap().to_owned(),
                    r["def_line"].as_u64().unwrap() as usize,
                    r["use_line"].as_u64().unwrap() as usize,
                )
            })
            .collect();
        assert!(!wanted.is_empty(), "{name}: empty golden is not a test");
        for row in expected
            .iter()
            .filter(|r| r["case"] == name && r.get("label").is_some())
        {
            let labels: Vec<_> = cpg
                .dfg
                .labels()
                .filter(|(e, _)| {
                    e.from.function == row["function"].as_str().unwrap()
                        && e.from.path.to_string() == row["path"].as_str().unwrap()
                        && e.from.line == row["def_line"].as_u64().unwrap() as usize
                        && e.to.line == row["use_line"].as_u64().unwrap() as usize
                })
                .map(|(_, label)| serde_json::to_value(label).unwrap())
                .collect();
            assert_eq!(labels, vec![row["label"].clone()], "RD label: {row}");
        }
        if actual != wanted {
            failures.push(format!(
                "{name}: extra={}, missing={}",
                actual.difference(&wanted).count(),
                wanted.difference(&actual).count()
            ));
        } else {
            println!("{name}: complete edge set matches");
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
