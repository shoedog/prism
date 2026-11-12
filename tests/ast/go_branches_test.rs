//! Go branch topology, wrong-label regressions and base-bound endpoint receipts.
use crate::common::*;
use prism::cpg::{FlowConfidence, FlowDoubt};
use serde_json::{json, Value};

fn paths() -> Vec<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut paths = Vec::new();
    for dir in [
        "go_branches",
        "dfg_prune/go",
        "go_jumps",
        "go_jumps_round2",
        "go_loop_cycles",
    ] {
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
fn parsed(name: &str) -> ParsedFile {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_branches");
    ParsedFile::parse(
        name,
        &std::fs::read_to_string(root.join(name)).unwrap(),
        Language::Go,
    )
    .unwrap()
}
pub(super) fn snapshot(path: &Path) -> Value {
    let file = path.file_name().unwrap().to_str().unwrap();
    let source = std::fs::read_to_string(path).unwrap();
    let parsed = ParsedFile::parse(file, &source, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error(), "{file}");
    let statements: Vec<_> = parsed
        .all_functions()
        .iter()
        .map(|f| parsed.statements_in_function(f))
        .collect();
    let cfg: Vec<_> = prism::cfg::build_cfg_edges(&parsed)
        .iter()
        .map(|e| json!([e.from_line, e.to_line]))
        .collect();
    let graph = CodePropertyGraph::build(&BTreeMap::from([(file.to_owned(), parsed)]));
    use super::recall_frozen as frozen;
    json!({"file":file,"edges":frozen::edges(&graph),"labels":frozen::labels(&graph),
        "stats":frozen::stats(&graph),
        "availability":frozen::availability(&graph)[file],"statements":statements,"cfg":cfg})
}
#[test]
fn go_branches_receipts() {
    let actual: Vec<_> = paths().iter().map(|p| snapshot(p)).collect();
    if std::env::var_os("PRISM_EXPORT_GO_BRANCHES").is_some() {
        for row in actual {
            println!("GO_BRANCH_SNAPSHOT {row}");
        }
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_branches");
    let base: Vec<Value> = std::fs::read_to_string(root.join("base.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let head: Vec<Value> = std::fs::read_to_string(root.join("expected.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(actual.len(), base.len());
    assert_eq!(actual.len(), head.len());
    let mut failures = Vec::new();
    for ((a, b), h) in actual.iter().zip(base).zip(head) {
        let b = super::go_blank_test::expected_without_blank_twins(b);
        let h = super::go_blank_test::expected_without_blank_twins(h);
        assert_eq!(a["file"], b["file"]);
        assert_eq!(
            a["edges"], b["edges"],
            "label-stripped endpoints: {}",
            a["file"]
        );
        assert_eq!(
            a["labels"].as_array().unwrap().len(),
            h["labels"].as_array().unwrap().len()
        );
        for (a_label, h_label) in a["labels"]
            .as_array()
            .unwrap()
            .iter()
            .zip(h["labels"].as_array().unwrap())
        {
            if a_label != h_label {
                failures.push(format!(
                    "{} / actual {a_label} / expected {h_label}",
                    a["file"]
                ));
            }
        }
        for key in ["stats", "availability"] {
            if a[key] != h[key] {
                failures.push(format!("{} / {key}", a["file"]));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
fn label(name: &str, d: usize, u: usize, expected: FlowConfidence) {
    let graph = CodePropertyGraph::build(&BTreeMap::from([(name.to_owned(), parsed(name))]));
    let actual: Vec<_> = graph
        .dfg
        .labels()
        .filter(|(e, _)| e.from.path.base == "x" && e.from.line == d && e.to.line == u)
        .map(|(_, l)| l)
        .collect();
    assert_eq!(actual, [expected], "{name} {d}->{u}");
}
fn absent(name: &str, d: usize, u: usize) {
    let graph = CodePropertyGraph::build(&BTreeMap::from([(name.to_owned(), parsed(name))]));
    let actual: Vec<_> = graph
        .dfg
        .labels()
        .filter(|(e, _)| e.from.path.base == "x" && e.from.line == d && e.to.line == u)
        .collect();
    assert!(
        actual.is_empty(),
        "{name} {d}->{u}: {} labels",
        actual.len()
    );
}
#[test]
fn go_branches_k13_k23_case_values_follow_the_loop_kill() {
    label(
        "case_value_loop.go",
        7,
        6,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
    );
    label(
        "case_value_second.go",
        7,
        9,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
    );
}
#[test]
fn go_branches_k21_empty_case_preserves_the_kill_free_path() {
    label("k21.go", 4, 12, FlowConfidence::Exact);
}
#[test]
fn go_branches_select_requires_an_arm() {
    for name in ["select_no_default.go", "select_default_only.go"] {
        label(
            name,
            2,
            7,
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 }),
        );
    }
    let p = parsed("select_empty.go");
    assert!(!prism::cfg::build_cfg_edges(&p)
        .iter()
        .any(|e| e.from_line == 3));
}
#[test]
fn go_branches_else_if_enters_the_header() {
    for (name, from, to, wrong) in [("else_if.go", 3, 5, 6), ("else_if_newline.go", 3, 6, 7)] {
        let edges = prism::cfg::build_cfg_edges(&parsed(name));
        assert!(
            edges.iter().any(|e| (e.from_line, e.to_line) == (from, to)),
            "{name}: {edges:?}"
        );
        assert!(
            !edges
                .iter()
                .any(|e| (e.from_line, e.to_line) == (from, wrong)),
            "{name}: skipped condition"
        );
    }
}
#[test]
fn go_branches_dispatch_and_no_sibling_fallthrough() {
    let p = parsed("expr.go");
    let edges = prism::cfg::build_cfg_edges(&p);
    for (from, to) in [(3, 4), (3, 6), (4, 5), (6, 7), (5, 9), (7, 9)] {
        assert!(
            edges.iter().any(|e| (e.from_line, e.to_line) == (from, to)),
            "{from}->{to}: {edges:?}"
        );
    }
    assert!(!edges.iter().any(|e| e.from_line == 5 && e.to_line < 9));
}

#[test]
fn go_branches_availability_empty_controls() {
    for (name, count) in [
        ("expr_empty_switch.go", 0),
        ("type_empty_switch.go", 0),
        ("only_empty_switch.go", 0),
        ("select_empty.go", 1),
        ("only_empty_select.go", 1),
    ] {
        let graph = CodePropertyGraph::build(&BTreeMap::from([(name.to_owned(), parsed(name))]));
        assert_eq!(
            graph.dfg_label_stats.dfg_rd_functions_without_cfg, count,
            "{name}"
        );
        assert_eq!(graph.dfg_label_stats.dfg_rd_functions_over_cap, 0);
    }
}
#[test]
fn go_branches_statement_limit_counts_dispatch_anchors() {
    let mut failures = Vec::new();
    for (calls, capped) in [(4093, 0), (4094, 1)] {
        let source = format!(
            "package p\nfunc f() {{\n{}switch 0 {{\ncase 0:\n}}\nreturn\n}}\n",
            "use()\n".repeat(calls)
        );
        let p = ParsedFile::parse("limit.go", &source, Language::Go).unwrap();
        let statements = p.statements_in_function(&p.all_functions()[0]).len();
        let graph = CodePropertyGraph::build(&BTreeMap::from([("limit.go".into(), p)]));
        let actual_cap = graph.dfg_label_stats.dfg_rd_functions_over_cap;
        println!(
            "GO_BRANCH_LIMIT calls={calls} statements={statements} capped={actual_cap} edges={}",
            graph.dfg.edges().len()
        );
        if statements != calls + 3 || actual_cap != capped {
            failures.push(format!(
                "calls={calls} statements={statements} capped={actual_cap}"
            ));
        }
        assert_eq!(graph.dfg_label_stats.dfg_rd_functions_without_cfg, 0);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn go_branches_fallthrough_skips_values_and_empty_destination_exits() {
    for (name, from, to, forbidden) in [
        ("fallthrough.go", 6, 8, 7),
        ("fallthrough_empty_next.go", 6, 11, 9),
    ] {
        let edges = prism::cfg::build_cfg_edges(&parsed(name));
        assert!(
            edges.iter().any(|e| (e.from_line, e.to_line) == (from, to)),
            "{name}: {edges:?}"
        );
        assert!(!edges
            .iter()
            .any(|e| (e.from_line, e.to_line) == (from, forbidden)));
    }
}

#[test]
fn go_branches_unreachable_select_def_keeps_doubt() {
    label(
        "select_dead_def.go",
        4,
        5,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}
#[test]
fn go_branches_fallthrough_ignores_intervening_comments() {
    let edges = prism::cfg::build_cfg_edges(&parsed("fallthrough_comment.go"));
    assert!(
        edges.iter().any(|e| (e.from_line, e.to_line) == (6, 9)),
        "{edges:?}"
    );
    assert!(!edges.iter().any(|e| (e.from_line, e.to_line) == (6, 8)));
}

#[test]
fn go_branches_initializer_alias_does_not_declare_its_source() {
    label("multiline_init.go", 6, 10, FlowConfidence::Exact);
    label("multiline_init.go", 8, 10, FlowConfidence::Exact);
    label(
        "multiline_init.go",
        2,
        10,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 }),
    );
    // A real x declaration in the initializer still owns the body writes.
    // The body writes bind the initializer's x; `return x` binds the parameter,
    // so rule S (#630) drops both pairs.
    absent("init_shadow.go", 5, 9);
    absent("init_shadow.go", 7, 9);
    label("init_shadow.go", 2, 9, FlowConfidence::Exact);
}
