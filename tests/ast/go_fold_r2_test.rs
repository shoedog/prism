//! Lexical introductions without producer endpoints; static Go inputs only.
use crate::common::*;
use prism::cpg::FlowConfidence;
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
    // 25 pairs on main; 15 `distinct` rows (range/receive `:=`, type-switch aliases including
    // the arm write, grouped and implicit const/var) no longer produce one: rule S drops the
    // pair now that the declaration has its own def (#694 slice A). The 10 left are the
    // `existing` controls and the remaining forms.
    assert!(
        observed > 8,
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

// The case's type list reads the enclosing constant; only its body reads the alias.
fn type_switch_visibility(case: &str, alias: bool, body_exact: bool) {
    let guard = if alias { "n := x.(type)" } else { "x.(type)" };
    let source = format!(
        "package p\nfunc f(x any) {{\nconst n = 1 // def\nswitch {guard} {{\n{case}\nprintln(n) // body\n}}\n}}\n"
    );
    let parsed = ParsedFile::parse("type_switch.go", &source, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error(), "{source}");
    let graph = CodePropertyGraph::build(&BTreeMap::from([("type_switch.go".into(), parsed)]));
    let mut failures = Vec::new();
    for (index, text) in source.lines().enumerate() {
        if !text.contains("// type") && !text.contains("// body") {
            continue;
        }
        let line = index + 1;
        let exact = text.contains("// type") || body_exact;
        let labels: Vec<_> = graph
            .dfg
            .labels()
            .filter(|(edge, _)| {
                edge.from.path.base == "n" && edge.from.line == 3 && edge.to.line == line
            })
            .map(|(_, label)| label)
            .collect();
        // With an alias the body's `n` is the alias, whose own def reaches it: rule S drops the
        // outer constant's pair (#694 slice A). The case's type list still reads the constant.
        let dropped = alias && text.contains("// body") && !body_exact;
        let pass = if dropped {
            labels.is_empty()
        } else {
            labels.len() == 1 && (labels[0] == FlowConfidence::Exact) == exact
        };
        println!("type-switch {case:?} alias={alias}: n@3 -> n@{line}, exact={exact}, actual={labels:?}, pass={pass}");
        if !pass {
            failures.push(format!("n@3 -> n@{line}: exact={exact}, actual={labels:?}"));
        }
    }
    assert!(failures.is_empty(), "{source}\n{}", failures.join("\n"));
}

#[test]
fn go_fold_r3_type_switch_single_line() {
    type_switch_visibility("case [n]int: // type", true, false);
}

#[test]
fn go_fold_r3_type_switch_multiline_comment() {
    type_switch_visibility(
        "case []string, // between types\n[n]int: // type",
        true,
        false,
    );
}

#[test]
fn go_fold_r3_type_switch_multiple_types() {
    type_switch_visibility("case [n]int, []string: // type", true, false);
}

#[test]
fn go_fold_r3_type_switch_controls() {
    type_switch_visibility("case int:", true, false);
    type_switch_visibility("default:", true, false);
    type_switch_visibility("case [n]int: // type", false, true);
}
