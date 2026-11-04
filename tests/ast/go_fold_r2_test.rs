//! Lexical introductions without producer endpoints; static Go inputs only.
use crate::common::*;
use serde_json::Value;

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_fold_r2")
}
fn rows() -> Vec<Value> {
    include_str!("../fixtures/go_fold_r2/population.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
#[test]
#[ignore = "explicit static synthetic export"]
fn go_fold_r2_export() {
    for row in rows() {
        println!(
            "GO_BRANCH_SNAPSHOT {}",
            super::go_branches_test::snapshot(&root().join(row["file"].as_str().unwrap()))
        );
    }
}

#[test]
fn go_fold_r2_outer_writes_do_not_define_inner_bindings() {
    let mut failures = Vec::new();
    let mut observed = 0;
    for row in rows() {
        let file = row["file"].as_str().unwrap();
        let source = std::fs::read_to_string(root().join(file)).unwrap();
        let line = |marker: &str| source.lines().position(|l| l.contains(marker)).unwrap() + 1;
        let snapshot = super::go_branches_test::snapshot(&root().join(file));
        let labels: Vec<_> = snapshot["labels"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| {
                l[0]["path"]["base"] == "v"
                    && l[0]["line"] == line("// write")
                    && l[1]["line"] == line("// inner")
            })
            .collect();
        observed += labels.len();
        if row["check"] == "existing" && labels.len() != 1 {
            failures.push(format!("{file}: missing assignment/reuse control"));
        }
        for label in labels {
            if (row["check"] == "distinct" && label[2] == "Exact")
                || (row["check"] == "existing" && label[2] != "Exact")
            {
                failures.push(format!("{file}: {}: {label}", row["check"]));
            }
        }
    }
    assert!(
        observed > 15,
        "probe selected too few actual producer pairs: {observed}"
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn go_fold_r2_complete_receipts() {
    let read = |file| {
        std::fs::read_to_string(root().join(file))
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str::<Value>(s).unwrap())
            .collect::<Vec<_>>()
    };
    let base = read("base.jsonl");
    let expected = read("expected.jsonl");
    assert_eq!(base.len(), rows().len());
    assert_eq!(expected.len(), base.len());
    let mut failures = Vec::new();
    for (base, expected) in base.iter().zip(&expected) {
        let file = base["file"].as_str().unwrap();
        let actual = super::go_branches_test::snapshot(&root().join(file));
        for key in ["file", "edges", "cfg", "statements", "availability"] {
            assert_eq!(actual[key], base[key], "{file}: {key}");
        }
        assert_eq!(
            actual["labels"].as_array().unwrap().len(),
            expected["labels"].as_array().unwrap().len()
        );
        for (a, e) in actual["labels"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected["labels"].as_array().unwrap())
        {
            if a != e {
                failures.push(format!("{file}: actual={a} expected={e}"));
            }
        }
        if actual["stats"] != expected["stats"] {
            failures.push(format!("{file}: stats"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
