//! #645 C/C++ unlabeled break/continue, switch dispatch and do-while condition.
//!
//! Variant matrix (written before the production change). Each fixture in
//! `tests/fixtures/c_jumps` is parsed as C and, where a `.cpp` twin exists, as
//! C++. A row lists CFG edges that must be present (`add`, absent on main),
//! edges that must be absent (`absent`) and RD labels `(var, def, use, label)`.
//! Label rows that already hold on main are the "unchanged" guards; the rest
//! are the label transitions the new edges cause.
use crate::common::*;
use prism::cpg::{FlowConfidence, FlowDoubt};

type Edge = (usize, usize);
type Label = (&'static str, usize, usize, &'static str);

fn label_name(l: FlowConfidence) -> String {
    match l {
        FlowConfidence::Exact => "exact".into(),
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line }) => format!("killed@{kill_line}"),
        FlowConfidence::NameOnly(FlowDoubt::SameLine) => "sameline".into(),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete) => "cfg_incomplete".into(),
        FlowConfidence::NameOnly(FlowDoubt::AliasUnstable) => "alias_unstable".into(),
        FlowConfidence::NameOnly(FlowDoubt::CallNameOnly) => "call_nameonly".into(),
    }
}

fn load(stem: &str, ext: &str) -> Option<(String, String, Language)> {
    let file = format!("{stem}.{ext}");
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/c_jumps")
        .join(&file);
    let source = std::fs::read_to_string(path).ok()?;
    let language = match ext {
        "c" => Language::C,
        "cpp" => Language::Cpp,
        "js" => Language::JavaScript,
        "java" => Language::Java,
        _ => unreachable!(),
    };
    Some((file, source, language))
}

fn edge_set(file: &str, source: &str, language: Language) -> Vec<Edge> {
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    let mut edges: Vec<Edge> = prism::cfg::build_cfg_edges(&parsed)
        .iter()
        .map(|e| (e.from_line, e.to_line))
        .collect();
    edges.sort();
    edges.dedup();
    edges
}

fn labels_of(file: &str, source: &str, language: Language) -> Vec<(String, usize, usize, String)> {
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(file.to_owned(), parsed)]));
    cpg.dfg
        .labels()
        .map(|(e, l)| {
            (
                e.from.path.base.clone(),
                e.from.line,
                e.to.line,
                label_name(l),
            )
        })
        .collect()
}

fn check(stem: &str, add: &[Edge], absent: &[Edge], labels: &[Label]) {
    let mut failures = Vec::new();
    let mut seen = 0;
    for ext in ["c", "cpp"] {
        let Some((file, source, language)) = load(stem, ext) else {
            continue;
        };
        seen += 1;
        let edges = edge_set(&file, &source, language);
        for e in add {
            if !edges.contains(e) {
                failures.push(format!("{file}: missing edge {e:?}; have {edges:?}"));
            }
        }
        for e in absent {
            if edges.contains(e) {
                failures.push(format!("{file}: unexpected edge {e:?}; have {edges:?}"));
            }
        }
        let got = labels_of(&file, &source, language);
        for (var, d, u, want) in labels {
            let rows: Vec<_> = got
                .iter()
                .filter(|(v, gd, gu, _)| v == var && gd == d && gu == u)
                .collect();
            match rows.as_slice() {
                [(_, _, _, label)] if label == want => {}
                other => failures.push(format!(
                    "{file}: {var} {d}->{u}: want {want}, got {other:?}"
                )),
            }
        }
    }
    assert!(seen > 0, "no fixture for {stem}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Edge set equal to a pinned list, for shapes whose edges must not move.
fn check_pinned(stem: &str, ext: &str, expected: &[Edge]) {
    let (file, source, language) = load(stem, ext).unwrap();
    assert_eq!(edge_set(&file, &source, language), expected, "{file}");
}

macro_rules! row {
    ($name:ident, $stem:literal, add: $add:expr, absent: $absent:expr, labels: $labels:expr) => {
        #[test]
        fn $name() {
            check($stem, &$add, &$absent, &$labels);
        }
    };
}

row!(for_break, "for_break", add: [(5, 8)], absent: [(5, 6), (5, 3)],
    labels: [("x", 4, 8, "exact"), ("x", 2, 8, "exact")]);
row!(for_continue, "for_continue", add: [(5, 3)], absent: [(5, 6), (5, 8)],
    labels: [("x", 4, 8, "exact")]);
row!(while_break, "while_break", add: [(5, 8)], absent: [(5, 6), (5, 3)],
    labels: [("x", 4, 8, "exact"), ("x", 2, 8, "exact")]);
row!(while_continue, "while_continue", add: [(5, 3)], absent: [(5, 6), (5, 8)],
    labels: [("x", 4, 8, "exact")]);
row!(do_break, "do_break", add: [(5, 8), (6, 7), (7, 4), (7, 8)], absent: [(3, 8), (5, 6), (5, 7)],
    labels: [("x", 4, 8, "exact"), ("x", 2, 8, "killed@4")]);
row!(do_continue, "do_continue", add: [(5, 7), (6, 7), (7, 4), (7, 8)], absent: [(3, 8), (5, 6), (5, 8)],
    labels: [("x", 4, 8, "exact"), ("x", 2, 8, "killed@4")]);
row!(bare_body_continue, "bare_body", add: [(3, 2)], absent: [(3, 4)],
    labels: [("x", 1, 4, "exact")]);
row!(final_exit_has_no_edge, "final_exit", add: [], absent: [(3, 2), (3, 4), (3, 5)],
    labels: []);
row!(if_exit, "if_exit", add: [(4, 9)], absent: [(4, 7), (4, 3)], labels: []);
row!(label_wrapper, "label_block", add: [(5, 2)], absent: [(5, 9), (5, 4)], labels: []);
row!(terminal_inner_loop, "terminal_inner_loop", add: [(5, 2)], absent: [(5, 8)],
    labels: [("x", 4, 8, "exact")]);
row!(range_for_continue, "range_continue", add: [(4, 2)], absent: [(4, 5), (4, 6)],
    labels: [("x", 3, 6, "exact")]);
row!(switch_break, "switch_break", add: [(3, 5), (3, 10), (6, 12)], absent: [(3, 12), (6, 7), (6, 10)],
    labels: [("x", 5, 12, "exact"), ("x", 10, 12, "exact"), ("x", 2, 12, "killed@5")]);
row!(switch_dispatch, "switch_dispatch", add: [(2, 7), (2, 10), (5, 12), (8, 12)],
    absent: [(2, 12), (5, 7), (8, 10)],
    labels: [("x", 4, 12, "exact"), ("x", 7, 12, "exact"), ("x", 10, 12, "exact")]);
row!(switch_empty_case, "switch_empty_case", add: [(2, 5), (2, 8), (6, 8)], absent: [(6, 5)],
    labels: [("x", 1, 8, "exact"), ("x", 5, 8, "exact")]);
row!(switch_fallthrough_kept, "switch_fallthrough", add: [(2, 4), (2, 6), (2, 9), (4, 6), (7, 9)],
    absent: [(7, 6)],
    labels: [("x", 4, 6, "exact"), ("x", 4, 9, "exact"), ("x", 1, 9, "exact")]);
row!(switch_no_default, "switch_no_default", add: [(2, 4), (2, 7), (5, 7)], absent: [],
    labels: [("x", 1, 7, "exact"), ("x", 4, 7, "exact")]);
row!(nested_switch_continue, "nested_switch_continue",
    add: [(3, 5), (3, 8), (6, 2), (8, 10)], absent: [(6, 10), (6, 8), (3, 10)],
    labels: [("x", 5, 12, "exact")]);

// Rows added beyond the design-stop fixtures.
row!(multi_statement_jump_line, "multi_line_jump", add: [(4, 3)], absent: [], labels: []);
row!(switch_inside_do_with_continue, "do_switch_continue",
    add: [(7, 13), (10, 12), (4, 6), (4, 9), (13, 4), (13, 14)],
    absent: [(3, 14), (7, 12), (7, 14), (4, 12)],
    labels: [("x", 6, 14, "exact")]);
row!(for_inside_switch_case, "for_in_switch",
    add: [(6, 8), (9, 13), (2, 4), (2, 11)], absent: [(6, 13), (2, 13)],
    labels: [("x", 5, 8, "exact")]);
row!(duff_device_has_no_dispatch, "duff", add: [(5, 6), (6, 4), (6, 8)],
    absent: [(2, 5), (2, 8), (3, 8)], labels: []);
row!(lambda_boundary, "lambda_boundary", add: [(9, 2)], absent: [(9, 11), (5, 2), (5, 8), (5, 11)],
    labels: []);
row!(local_class_boundary, "local_class_boundary", add: [(11, 2)],
    absent: [(11, 13), (6, 2), (6, 10), (6, 13)], labels: []);

#[test]
fn one_line_do_while_keeps_main_edges() {
    check_pinned("one_line_do", "c", &[(2, 3), (3, 3), (3, 4)]);
}

#[test]
fn do_condition_on_last_body_line_keeps_main_edges() {
    check_pinned(
        "do_cond_on_last",
        "c",
        &[(2, 3), (3, 4), (3, 5), (4, 3), (4, 5)],
    );
}

/// JS and Java switch/loop shapes are outside this slice: edge sets pinned
/// from main.
#[test]
fn js_and_java_edge_sets_are_unchanged() {
    check_pinned(
        "js_switch_loop",
        "js",
        &[
            (2, 3),
            (3, 4),
            (3, 15),
            (4, 3),
            (4, 6),
            (6, 7),
            (9, 10),
            (12, 15),
            (15, 16),
            (15, 18),
            (16, 15),
            (16, 18),
        ],
    );
    check_pinned(
        "java_switch_loop",
        "java",
        &[
            (3, 4),
            (4, 7),
            (4, 14),
            (7, 8),
            (10, 12),
            (12, 4),
            (12, 12),
            (12, 14),
            (14, 15),
            (14, 17),
            (15, 14),
            (15, 17),
        ],
    );
}
