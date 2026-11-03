//! Slice-2 CFG contracts and main-bound label-stripped DFG receipts.
use crate::common::*;
use serde_json::{json, Value};

fn fixtures() -> Vec<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut paths = Vec::new();
    for dir in ["dfg_prune/go", "go_jumps"] {
        paths.extend(
            std::fs::read_dir(root.join(dir))
                .unwrap()
                .map(|p| p.unwrap().path())
                .filter(|p| p.extension().is_some_and(|e| e == "go")),
        );
    }
    paths.sort();
    paths
}

#[test]
fn go_jump_cfg_targets() {
    let source = include_str!("../fixtures/go_jumps/jumps.go");
    let parsed = ParsedFile::parse("jumps.go", source, Language::Go).unwrap();
    let edges = prism::cfg::build_cfg_edges(&parsed);
    let lines: Vec<_> = source.lines().collect();
    let mut failures = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some((_, annotation)) = line.split_once("// jump:") {
            let (name, target) = annotation.split_once(':').unwrap();
            let dest = if name == target {
                i + 1
            } else {
                lines
                    .iter()
                    .position(|l| l.contains(&format!("// target:{target}")))
                    .unwrap()
                    + 1
            };
            if !edges
                .iter()
                .any(|e| e.from_line == i + 1 && e.to_line == dest)
            {
                failures.push(format!("{name}: missing {}->{dest}", i + 1));
            }
            if edges
                .iter()
                .any(|e| e.from_line == i + 1 && e.to_line != dest)
            {
                failures.push(format!("{name}: unexpected outgoing edge"));
            }
        }
        if line.contains("// noedge:") && edges.iter().any(|e| e.from_line == i + 1) {
            failures.push(format!(
                "function-final break must have no exit edge: {}",
                i + 1
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn snapshot(path: &Path) -> Value {
    let file = path.file_name().unwrap().to_str().unwrap();
    let source = std::fs::read_to_string(path).unwrap();
    let parsed = ParsedFile::parse(file, &source, Language::Go).unwrap();
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(file.to_owned(), parsed)]));
    let mut edges: Vec<_> = cpg
        .dfg
        .edges()
        .iter()
        .map(|e| json!([e.from, e.to]))
        .collect();
    edges.sort_by_key(Value::to_string);
    let mut labels: Vec<_> = cpg
        .dfg
        .labels()
        .map(|(e, l)| json!([e.from, e.to, l]))
        .collect();
    labels.sort_by_key(Value::to_string);
    json!({"file":file,"edges":edges,"labels":labels,
        "availability":cpg.dfg.rd_function_stats()[file],"stats":cpg.dfg_label_stats})
}

#[test]
fn go_jump_labels_and_edge_keys() {
    let actual: Vec<_> = fixtures().iter().map(|p| snapshot(p)).collect();
    // Explicit export for same-environment base/head receipts; never rewrites goldens.
    if std::env::var_os("PRISM_EXPORT_GO_JUMPS").is_some() {
        for row in &actual {
            println!("GO_JUMP_SNAPSHOT {row}");
        }
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_jumps");
    let read = |name| -> Vec<Value> {
        std::fs::read_to_string(root.join(name))
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    };
    let base = read("base.jsonl");
    let changes = read("label_changes.jsonl");
    let changed_stats = read("expected_stats.jsonl");
    assert_eq!(actual.len(), base.len());
    assert_eq!(changes.len(), 47, "audit every expected label change");
    let mut failures = Vec::new();
    for (a, b) in actual.iter().zip(&base) {
        assert_eq!(a["file"], b["file"]);
        assert_eq!(
            a["edges"], b["edges"],
            "label-stripped edge multiset: {}",
            a["file"]
        );
        let mut wanted = b.clone();
        for row in changes.iter().filter(|r| r["file"] == a["file"]) {
            let matches = |label: &&mut Value| {
                label[0]["function"] == row["function"]
                    && label[0]["path"]["base"] == row["path"]
                    && label[0]["line"] == row["d"]
                    && label[1]["line"] == row["u"]
            };
            let mut entries: Vec<_> = wanted["labels"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .filter(matches)
                .collect();
            assert_eq!(entries.len(), 1, "unbound/ambiguous obligation: {row}");
            assert_eq!(entries[0][2], row["old"], "base label: {row}");
            assert_ne!(row["old"], row["new"], "obligation must fail on main");
            entries[0][2] = row["new"].clone();
            let actual_label = a["labels"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| {
                    l[0]["function"] == row["function"]
                        && l[0]["path"]["base"] == row["path"]
                        && l[0]["line"] == row["d"]
                        && l[1]["line"] == row["u"]
                })
                .unwrap();
            if actual_label[2] != row["new"] {
                failures.push(format!("{row}: actual {}", actual_label[2]));
            }
        }
        if let Some(stats) = changed_stats.iter().find(|s| s["file"] == a["file"]) {
            wanted["stats"] = stats["stats"].clone();
            wanted["availability"] = stats["availability"].clone();
        }
        if *a != wanted {
            failures.push(a["file"].to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "label/availability changes: {}",
        failures.join(", ")
    );
}

#[test]
fn go_jump_defer_arguments_are_not_reads_at_function_exit() {
    use prism::cpg::{FlowConfidence, FlowDoubt};
    let source = include_str!("../fixtures/go_jumps/defer_exit.go");
    let parsed = ParsedFile::parse("defer_exit.go", source, Language::Go).unwrap();
    let dfg = DataFlowGraph::build(&BTreeMap::from([("defer_exit.go".to_owned(), parsed)]));
    let label = |function: &str, from, to| {
        dfg.labels()
            .find_map(|(e, label)| {
                (e.from.function == function
                    && e.from.path.base == "x"
                    && e.from.line == from
                    && e.to.line == to)
                    .then_some(label)
            })
            .expect("required defer-argument edge")
    };
    // Defer arguments were evaluated before this write. Synthetic exit
    // dispatch must have the same doubt as return->defer, never a new Exact.
    assert_eq!(
        label("exitDefer", 5, 3),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
    );
    // A following defer is reached normally: its argument really reads x=1.
    assert_eq!(label("nextDefer", 11, 14), FlowConfidence::Exact);
}
