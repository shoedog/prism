//! Static synthetic join population: transparent syntax must not sever flow.
use crate::common::*;
use prism::cpg::FlowConfidence;
use serde_json::{json, Value};

fn rows() -> Vec<Value> {
    include_str!("../fixtures/go_join/population.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_join")
}
#[test]
fn go_join_population() {
    let mut failures = Vec::new();
    let base: Vec<Value> = include_str!("../fixtures/go_join/base.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let population = rows();
    assert_eq!(base.len(), population.len());
    for (row, before) in population.into_iter().zip(base) {
        let file = row["file"].as_str().unwrap();
        let source = std::fs::read_to_string(root().join(file)).unwrap();
        let p = ParsedFile::parse(file, &source, Language::Go).unwrap();
        assert!(!p.tree.root_node().has_error(), "{file}");
        let cfg = prism::cfg::build_cfg_edges(&p);
        let statements = p.statements_in_function(&p.all_functions()[0]);
        let graph = CodePropertyGraph::build(&BTreeMap::from([(file.into(), p)]));
        let mut endpoints: Vec<_> = graph
            .dfg
            .edges()
            .iter()
            .map(|e| json!([e.from, e.to]))
            .collect();
        endpoints.sort_by_key(Value::to_string);
        assert_eq!(before["file"], file);
        assert_eq!(
            before["edges"],
            json!(endpoints),
            "{file}: label-stripped multiset"
        );
        assert_eq!(
            before["statements"],
            json!(statements),
            "{file}: statement universe"
        );
        assert_eq!(
            before["availability"],
            json!(graph.dfg.rd_function_stats()),
            "{file}: availability"
        );
        let labels: Vec<_> = graph
            .dfg
            .labels()
            .filter(|(e, _)| {
                e.from.path.base == "x"
                    && e.from.line == row["definition"].as_u64().unwrap() as usize
                    && e.to.line == row["use"].as_u64().unwrap() as usize
            })
            .map(|(_, l)| l)
            .collect();
        let mut missing = Vec::new();
        for edge in row["edges"].as_array().unwrap() {
            let from = edge[0].as_u64().unwrap() as usize;
            let to = edge[1].as_u64().unwrap() as usize;
            if !cfg.iter().any(|e| e.from_line == from && e.to_line == to) {
                missing.push(edge.clone());
            }
        }
        let mut reachable = std::collections::BTreeSet::from([3]);
        loop {
            let before = reachable.len();
            for e in &cfg {
                if reachable.contains(&e.from_line) {
                    reachable.insert(e.to_line);
                }
            }
            if reachable.len() == before {
                break;
            }
        }
        let unreachable: Vec<_> = statements
            .iter()
            .filter(|(line, _)| !reachable.contains(line))
            .collect();
        if labels != [FlowConfidence::Exact] || !missing.is_empty() || !unreachable.is_empty() {
            failures.push(format!(
                "{file}: labels={labels:?}, missing={missing:?}, unreachable={unreachable:?}"
            ));
        }
        if std::env::var_os("PRISM_EXPORT_GO_JOIN").is_some() {
            let mut edges: Vec<_> = graph
                .dfg
                .edges()
                .iter()
                .map(|e| json!([e.from, e.to]))
                .collect();
            edges.sort_by_key(Value::to_string);
            let mut all_labels: Vec<_> = graph
                .dfg
                .labels()
                .map(|(e, l)| json!([e.from, e.to, l]))
                .collect();
            all_labels.sort_by_key(Value::to_string);
            println!(
                "GO_JOIN_SNAPSHOT {}",
                json!({"file":file,"edges":edges,"labels":all_labels,
                "cfg":cfg.iter().map(|e|[e.from_line,e.to_line]).collect::<Vec<_>>(),
                "statements":statements,"stats":graph.dfg_label_stats,"availability":graph.dfg.rd_function_stats()})
            );
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
fn go_join_reverse_transitions_are_entry_unreachable() {
    use prism::cpg::FlowDoubt;
    let mut failures = Vec::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_join_reverse");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    files.sort();
    for path in files {
        let file = path.file_name().unwrap().to_str().unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        let line = |marker| source.lines().position(|l| l.contains(marker)).unwrap() + 1;
        let p = ParsedFile::parse(file, &source, Language::Go).unwrap();
        assert!(!p.tree.root_node().has_error(), "{file}");
        let cfg = prism::cfg::build_cfg_edges(&p);
        let graph = CodePropertyGraph::build(&BTreeMap::from([(file.into(), p)]));
        let labels: Vec<_> = graph
            .dfg
            .labels()
            .filter(|(e, _)| e.from.line == line("// def") && e.to.line == line("// use"))
            .map(|(_, l)| l)
            .collect();
        println!(
            "GO_REVERSE_SNAPSHOT {}",
            json!({"file":file,"labels":graph.dfg.labels().map(|(e,l)|json!([e.from,e.to,l])).collect::<Vec<_>>(),"edges":graph.dfg.edges().iter().map(|e|json!([e.from,e.to])).collect::<Vec<_>>(),"cfg":cfg.iter().map(|e|[e.from_line,e.to_line]).collect::<Vec<_>>() })
        );
        if labels.is_empty()
            || !labels
                .iter()
                .all(|l| *l == FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))
        {
            failures.push(format!("{file}: {labels:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn go_join_reachable_label_classes() {
    use prism::cpg::FlowDoubt;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_join_classes");
    let mut failures = Vec::new();
    for class in ["exact", "killed", "alias_unstable"] {
        let file = format!("{class}.go");
        let source = std::fs::read_to_string(dir.join(&file)).unwrap();
        let line = |marker| source.lines().position(|l| l.contains(marker)).unwrap() + 1;
        let p = ParsedFile::parse(&file, &source, Language::Go).unwrap();
        let graph = CodePropertyGraph::build(&BTreeMap::from([(file.clone(), p)]));
        let labels: Vec<_> = graph
            .dfg
            .labels()
            .filter(|(e, _)| {
                e.from.line == line("// def")
                    && e.to.line == line("// use")
                    && e.from.path.base == if class == "alias_unstable" { "p" } else { "x" }
            })
            .map(|(_, l)| l)
            .collect();
        let expected = match class {
            "exact" => FlowConfidence::Exact,
            "alias_unstable" => FlowConfidence::NameOnly(FlowDoubt::AliasUnstable),
            _ => FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
        };
        println!(
            "GO_CLASS_SNAPSHOT {}",
            json!({"file":file,"labels":graph.dfg.labels().map(|(e,l)|json!([e.from,e.to,l])).collect::<Vec<_>>(),"edges":graph.dfg.edges().iter().map(|e|json!([e.from,e.to])).collect::<Vec<_>>() })
        );
        if labels != [expected] {
            failures.push(format!("{class}: actual={labels:?} expected={expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn go_join_nonanchor_population_matches_pinned_grammar() {
    let grammar: Vec<Value> = serde_json::from_str(include_str!(
        "../../vendor/tree-sitter-go/src/node-types.json"
    ))
    .unwrap();
    fn expand(kind: &str, grammar: &[Value], out: &mut Vec<String>) {
        let row = grammar.iter().find(|r| r["type"] == kind).unwrap();
        if let Some(subtypes) = row["subtypes"].as_array() {
            for s in subtypes {
                expand(s["type"].as_str().unwrap(), grammar, out);
            }
        } else {
            out.push(kind.into());
        }
    }
    let mut kinds = Vec::new();
    expand("_statement", &grammar, &mut kinds);
    kinds.retain(|k| !Language::Go.is_statement_node(k));
    kinds.sort();
    assert_eq!(
        kinds,
        [
            "block",
            "dec_statement",
            "empty_statement",
            "inc_statement",
            "type_declaration"
        ]
    );
}

#[test]
#[ignore = "explicit static source control export"]
fn go_join_export_all() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut paths = Vec::new();
    for dir in [
        "go_join",
        "go_join_classes",
        "go_join_reverse",
        "go_join_controller",
    ] {
        paths.extend(
            std::fs::read_dir(root.join(dir))
                .unwrap()
                .map(|p| p.unwrap().path())
                .filter(|p| p.extension().is_some_and(|e| e == "go")),
        );
    }
    paths.sort();
    for path in paths {
        println!(
            "GO_BRANCH_SNAPSHOT {}",
            super::go_branches_test::snapshot(&path)
        );
    }
}

#[test]
fn go_join_complete_label_and_auxiliary_endpoint_receipts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let extra: Vec<Value> = include_str!("../fixtures/go_join/extra-base.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let mut failures = Vec::new();
    for line in include_str!("../fixtures/go_join/labels.jsonl").lines() {
        let expected: Value = serde_json::from_str(line).unwrap();
        let file = expected["file"].as_str().unwrap();
        let actual = super::go_branches_test::snapshot(
            &root.join(expected["dir"].as_str().unwrap()).join(file),
        );
        let a = actual["labels"].as_array().unwrap();
        let h = expected["labels"].as_array().unwrap();
        assert_eq!(a.len(), h.len(), "{file}: labels must retain endpoints");
        for (a, h) in a.iter().zip(h) {
            if a != h {
                failures.push(format!("{file}: actual={a} expected={h}"));
            }
        }
        if let Some(before) = extra.iter().find(|r| r["file"] == file) {
            for key in ["edges", "statements", "availability"] {
                assert_eq!(actual[key], before[key], "{file}: {key}");
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} label mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn go_join_controller_shapes_restore_uses_without_new_anchors() {
    use prism::cpg::FlowDoubt;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_join_controller");
    for (file, uses, own) in [
        ("v4.go", vec![11, 12, 13], Some(10)),
        ("v10.go", vec![11, 12], None),
        ("v9.go", vec![12, 13], Some(11)),
        ("vitess_shape.go", vec![18], None),
    ] {
        let p = ParsedFile::parse(
            file,
            &std::fs::read_to_string(root.join(file)).unwrap(),
            Language::Go,
        )
        .unwrap();
        let graph = CodePropertyGraph::build(&BTreeMap::from([(file.into(), p)]));
        for line in uses.into_iter().chain(own) {
            let expected = if own == Some(line) {
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
            } else {
                FlowConfidence::Exact
            };
            let actual: Vec<_> = graph
                .dfg
                .labels()
                .filter(|(e, _)| e.from.path.base == "i" && e.from.line == 4 && e.to.line == line)
                .map(|(_, l)| l)
                .collect();
            assert_eq!(actual, [expected], "{file}: i4->{line}");
        }
    }
}
