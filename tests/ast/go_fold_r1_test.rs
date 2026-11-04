//! Static regressions for PR 652 review round one; never execute Go sources.
use crate::common::*;
use serde_json::{json, Value};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_fold_r1")
}
fn rows() -> Vec<Value> {
    include_str!("../fixtures/go_fold_r1/population.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn check(group: &str) {
    let mut failures = Vec::new();
    for row in rows().into_iter().filter(|r| r["group"] == group) {
        let file = row["file"].as_str().unwrap();
        let snapshot = super::go_branches_test::snapshot(&root().join(file));
        for check in row["checks"].as_array().unwrap() {
            let actual: Vec<_> = snapshot["labels"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|l| {
                    l[0]["path"]["base"] == check[0]
                        && l[0]["line"] == check[1]
                        && l[1]["line"] == check[2]
                })
                .map(|l| &l[2])
                .collect();
            let expected = check[3].as_str().unwrap();
            let matches = if expected == "absent" {
                actual.is_empty()
            } else {
                actual.len() == 1
                    && match expected {
                        "exact" => actual[0] == "Exact",
                        "killed" => actual[0]["NameOnly"]["Killed"].is_object(),
                        "sameline" => actual[0]["NameOnly"] == "SameLine",
                        _ => false,
                    }
            };
            if !matches {
                failures.push(format!("{file}: {check}: {actual:?}"));
            }
        }
        for edge in row["edges"].as_array().unwrap() {
            if !snapshot["cfg"].as_array().unwrap().contains(edge) {
                failures.push(format!("{file}: missing {edge}"));
            }
        }
        for edge in row["forbidden"].as_array().unwrap() {
            if snapshot["cfg"].as_array().unwrap().contains(edge) {
                failures.push(format!("{file}: bypass {edge}"));
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
#[test]
fn go_fold_r1_alias_declarations() {
    check("alias");
}
#[test]
fn go_fold_r1_clause_scopes() {
    check("scope");
}
#[test]
fn go_fold_r1_initializer_entry() {
    check("initializer");
}
#[test]
fn go_fold_r1_real_binding_controls() {
    check("control");
}
#[test]
fn go_fold_r1_other_simple_initializers() {
    check("simple");
}

#[test]
fn go_fold_r1_terminal_initializer_availability() {
    let mut failures = Vec::new();
    let base: Vec<Value> = include_str!("../fixtures/go_fold_r1/terminal-base.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    for (owner, keyword, condition) in [
        ("if", "if", "true"),
        ("expr", "switch", "0"),
        ("type", "switch", "interface{}(nil).(type)"),
    ] {
        for (form, init) in [("assign", "x = 1"), ("short", "x := 1")] {
            let file = format!("terminal_{owner}_{form}.go");
            let source =
                format!("package p\nfunc f(x int) {{\n{keyword}\n{init}; {condition} {{}}\n}}\n");
            let parsed = ParsedFile::parse(&file, &source, Language::Go).unwrap();
            assert!(!parsed.tree.root_node().has_error(), "{file}");
            let statements = parsed.statements_in_function(&parsed.all_functions()[0]);
            let cfg: Vec<_> = prism::cfg::build_cfg_edges(&parsed)
                .iter()
                .map(|e| [e.from_line, e.to_line])
                .collect();
            let graph = CodePropertyGraph::build(&BTreeMap::from([(file.clone(), parsed)]));
            let before = base.iter().find(|r| r["file"] == file).unwrap();
            if before["edges"]
                != json!(graph
                    .dfg
                    .edges()
                    .iter()
                    .map(|e| json!([e.from, e.to]))
                    .collect::<Vec<_>>())
                || before["statements"] != json!(statements)
            {
                failures.push(format!(
                    "{file}: endpoints or statement universe differs from4db"
                ));
            }
            println!(
                "GO_BRANCH_SNAPSHOT {}",
                json!({"file":file,"source":source,
                "edges":graph.dfg.edges().iter().map(|e|json!([e.from,e.to])).collect::<Vec<_>>(),
                "labels":graph.dfg.labels().map(|(e,l)|json!([e.from,e.to,l])).collect::<Vec<_>>(),
                "cfg":cfg,"statements":statements,"stats":graph.dfg_label_stats,
                "availability":graph.dfg.rd_function_stats()[&file]})
            );
            if cfg != [[3, 4]]
                || graph.dfg_label_stats.dfg_rd_functions_without_cfg != 0
                || graph.dfg.labels().count() != 1
                || !graph.dfg.labels().all(|(_, l)| {
                    l == prism::cpg::FlowConfidence::NameOnly(prism::cpg::FlowDoubt::CfgIncomplete)
                })
            {
                failures.push(format!(
                    "{file}: cfg={cfg:?}, availability={:?}, edges={}",
                    graph.dfg.rd_function_stats(),
                    graph.dfg.edges().len()
                ));
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

#[test]
#[ignore = "explicit static synthetic export"]
fn go_fold_r1_export() {
    for row in rows() {
        println!(
            "GO_BRANCH_SNAPSHOT {}",
            super::go_branches_test::snapshot(&root().join(row["file"].as_str().unwrap()))
        );
    }
}

#[test]
fn go_fold_r1_complete_receipts() {
    let base: Vec<Value> = include_str!("../fixtures/go_fold_r1/base.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let expected: Vec<Value> = include_str!("../fixtures/go_fold_r1/expected.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(base.len(), rows().len());
    assert_eq!(expected.len(), base.len());
    let mut failures = Vec::new();
    for (before, expected) in base.iter().zip(expected) {
        let file = before["file"].as_str().unwrap();
        let actual = super::go_branches_test::snapshot(&root().join(file));
        for key in ["edges", "statements", "availability"] {
            assert_eq!(actual[key], before[key], "{file}: {key}");
        }
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
        assert_eq!(
            actual["labels"].as_array().unwrap().len(),
            expected["labels"].as_array().unwrap().len()
        );
    }
    assert!(
        failures.is_empty(),
        "{} label mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
