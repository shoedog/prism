//! Frozen S1 contracts. The programs are parsed, never executed.
use super::Producer;
use crate::ast::ParsedFile;
use crate::cpg::FlowConfidence;
use crate::languages::Language;
use std::collections::BTreeSet;

pub(super) fn matrix(case: &str) {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_field_write");
    let language = match &case[..2] {
        "35" => Language::Python,
        "36" => Language::JavaScript,
        _ => Language::Go,
    };
    let extension = match language {
        Language::Python => "py",
        Language::JavaScript => "js",
        _ => "go",
    };
    let file = format!("main.{extension}");
    let source = std::fs::read_to_string(root.join(case).join(&file)).unwrap();
    let parsed = ParsedFile::parse(&file, &source, language).unwrap();
    let parts = Producer::file_refs(&file, &parsed);
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/go_field_write/expected.json"
    ))
    .unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["case"] == case)
        .unwrap();
    let expected: BTreeSet<_> = row["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e[0].as_str().unwrap().to_string(),
                e[1].as_u64().unwrap() as usize,
                e[2].as_str().unwrap().to_string(),
                e[3].as_u64().unwrap() as usize,
                e[4].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let mut actual = BTreeSet::new();
    for edge in &parts.edges {
        let d = &edge.from.path;
        let u = &edge.to.path;
        if d.base == u.base && d.fields.len() > u.fields.len() && d.fields.starts_with(&u.fields) {
            let label = match parts.labels.get(&(edge.from.clone(), edge.to.clone())) {
                Some(FlowConfidence::Exact) => "Exact",
                Some(FlowConfidence::NameOnly(crate::cpg::FlowDoubt::CfgIncomplete)) => {
                    "CfgIncomplete"
                }
                Some(FlowConfidence::NameOnly(crate::cpg::FlowDoubt::SameLine)) => "SameLine",
                other => panic!("{case}: {edge:?}: {other:?}"),
            };
            actual.insert((
                d.to_string(),
                edge.from.line,
                u.to_string(),
                edge.to.line,
                label.to_string(),
            ));
            assert!(edge.from.start_byte < edge.from.end_byte);
            assert!(edge.to.start_byte < edge.to.end_byte);
            assert!(parts.defs.values().flatten().any(|d| d == &edge.from
                && d.start_byte == edge.from.start_byte
                && d.end_byte == edge.from.end_byte));
            assert!(parts.uses.values().flatten().any(|u| u == &edge.to
                && u.start_byte == edge.to.start_byte
                && u.end_byte == edge.to.end_byte));
        }
    }
    assert_eq!(actual, expected, "{case}");
    if case.starts_with("32") {
        assert!(!parts
            .edges
            .iter()
            .any(|e| e.from.path.base == "err" && e.from.line == 11 && e.to.line == 9));
    }
    if case.starts_with("37") {
        assert!(parts.edges.iter().any(|e| e.from.path.to_string() == "p.f"
            && e.from.line == 4
            && e.to.path.to_string() == "p.f"
            && e.to.line == 5));
    }
}

macro_rules! cases {
    ($($test:ident => $case:literal),* $(,)?) => { $(
        #[test] fn $test() { matrix($case); }
    )* };
}
cases! {
    go_field_recall_01 => "01_call_arg", go_field_recall_02 => "02_return",
    go_field_recall_03 => "03_short_copy", go_field_recall_04 => "04_assign_copy",
    go_field_recall_05 => "05_composite", go_field_recall_06 => "06_address",
    go_field_recall_07 => "07_receiver", go_field_recall_08 => "08_channel",
    go_field_recall_09 => "09_defer", go_field_recall_10 => "10_go",
    go_field_recall_11 => "11_capture", go_field_recall_12 => "12_sibling",
    go_field_recall_13 => "13_nested", go_field_recall_14 => "14_value",
    go_field_recall_15 => "15_pointer", go_field_recall_16 => "16_root_kill",
    go_field_recall_17 => "17_shadow", go_field_recall_18 => "18_deref_kill",
    go_field_recall_19 => "19_field_kill", go_field_recall_20 => "20_branch",
    go_field_recall_21 => "21_loop_carried", go_field_recall_22 => "22_loop_exit",
    go_field_recall_23 => "23_map", go_field_recall_24 => "24_slice",
    go_field_recall_25 => "25_alias", go_field_recall_26 => "26_nested_kill",
    go_field_recall_27 => "27_sibling_write", go_field_recall_28 => "28_same_line",
    go_field_recall_29 => "29_capture_write", go_field_recall_30 => "30_both_branches_kill",
    go_field_recall_31 => "31_pointer_copy", go_field_recall_32 => "32_answer_shape",
    go_field_recall_33 => "33_map_key_use", go_field_recall_34 => "34_deref_read",
    go_field_recall_35 => "35_python", go_field_recall_36 => "36_javascript",
    go_field_recall_37 => "37_field_control", go_field_recall_38 => "38_index_controls",
    go_field_recall_39 => "39_sibling_with_bare", go_field_recall_40 => "40_independent_fields",
    go_field_recall_41 => "41_nested_sibling", go_field_recall_42 => "42_reused_short_binding",
    go_field_recall_43 => "43_nested_sibling_write", go_field_recall_44 => "44_compound_write",
    go_field_recall_45 => "45_answer_literal_control",
}

fn answer() -> crate::cpg::CodePropertyGraph {
    let source = include_str!("../../../tests/fixtures/go_field_write/32_answer_shape/main.go");
    let files = std::collections::BTreeMap::from([(
        "answer.go".to_string(),
        ParsedFile::parse("answer.go", source, Language::Go).unwrap(),
    )]);
    crate::cpg::CodePropertyGraph::build(&files)
}

#[test]
fn go_field_recall_answer_taint() {
    let graph = answer();
    let trace = graph.taint_trace(&[("answer.go".to_string(), 6)]);
    assert!(trace.warnings.is_empty(), "{:?}", trace.warnings);
    assert!(trace
        .frontier_by_root
        .values()
        .flatten()
        .any(|n| graph.node(*n).line() == 13));
}

#[test]
fn go_field_recall_answer_chop() {
    use crate::cpg::{CpgEdge, CpgNode};
    let graph = answer();
    let at = |line: usize, path: &str| -> Vec<_> {
        graph
            .nodes_at("answer.go", line)
            .into_iter()
            .filter(|&i| matches!(&graph.graph[i], CpgNode::Variable { path: p, .. } if p.to_string() == path))
            .collect()
    };
    let (src, sink) = (at(8, "req.Username"), at(9, "req"));
    assert!(!src.is_empty() && !sink.is_empty());
    let on_path = graph.chop(&src, &sink, &|e| matches!(e, CpgEdge::DataFlow(_)));
    assert!(
        src.iter().all(|n| on_path.contains(n)) && sink.iter().all(|n| on_path.contains(n)),
        "the new field->ancestor edge must connect the chop endpoints"
    );
}

#[test]
fn go_field_recall_answer_leftflow() {
    let graph = answer();
    let def = graph
        .dfg
        .edges()
        .iter()
        .find(|e| e.from.line == 8 && e.from.path.to_string() == "req.Username")
        .unwrap()
        .from
        .clone();
    let reached = graph.dfg_forward_reachable(&def);
    assert!(reached
        .iter()
        .any(|u| u.path.to_string() == "req" && u.line == 9));
    assert!(reached.iter().any(|u| u.line == 13));
}

#[test]
fn go_field_recall_answer_kill_and_sibling_taint_guards() {
    let source = include_str!("../../../tests/fixtures/go_field_write/32_answer_shape/main.go");
    for replacement in [
        "req.Username = username\n req = &Request{}",
        "req.Username = username\n req.Username = \"clean\"",
        "req.Username = username\n // sibling consumer",
    ] {
        let source = source.replace("req.Username = username", replacement);
        let source = if replacement.contains("sibling consumer") {
            source.replace("lookup(req)", "lookup(req.Other)")
        } else {
            source
        };
        let graph = crate::cpg::CodePropertyGraph::build(&std::collections::BTreeMap::from([(
            "answer.go".into(),
            ParsedFile::parse("answer.go", &source, Language::Go).unwrap(),
        )]));
        let trace = graph.taint_trace(&[("answer.go".into(), 6)]);
        assert!(trace.warnings.is_empty());
        assert!(
            !trace
                .frontier_by_root
                .values()
                .flatten()
                .any(|n| graph.node(*n).line() == 14),
            "{source}"
        );
        assert!(
            !graph.dfg.edges().iter().any(|e| e.from.line == 8
                && e.to.line == 10
                && e.from.path.has_fields()
                && e.to.path.is_simple()),
            "{source}"
        );
        assert!(!graph
            .dfg
            .edges()
            .iter()
            .any(|e| e.from.path.base == "err" && e.from.line == 12 && e.to.line == 10));
    }
}

#[test]
fn go_field_recall_syntax_refusals_and_nested_eager_argument() {
    for source in [
        "package p\nfunc f(p T, q T) {\np.f = 1\nif p == q { use(p.g) }\n}\n",
        "package p\nfunc f(p T) {\np.a.b.c.d.e.f = 1\nuse(p)\n}\n",
        "package p\nfunc f(p T) {\np.a[0].b = 1\nuse(p)\n}\n",
        "package p\nfunc f(p T) {\np.f = 1; p = T{}\nuse(p)\n}\n",
        "package p\nfunc f(p T) {\np.f = 1\nuse(func() { use(p) })\n}\n",
    ] {
        let parsed = ParsedFile::parse("a.go", source, Language::Go).unwrap();
        let refs = Producer::file_refs(&parsed.path, &parsed);
        assert!(
            !refs
                .edges
                .iter()
                .any(|e| e.from.path.has_fields() && e.to.path.is_simple()),
            "{source}"
        );
    }
    let parsed = ParsedFile::parse(
        "a.go",
        "package p\nfunc f(p T) {\np.f = 1\nif ready(p) { use(p.g) }\n}\n",
        Language::Go,
    )
    .unwrap();
    let refs = Producer::file_refs(&parsed.path, &parsed);
    assert!(refs.edges.iter().any(|e| e.from.path.to_string() == "p.f"
        && e.from.line == 3
        && e.to.path.to_string() == "p"
        && e.to.line == 4));
}

#[test]
fn go_field_recall_compound_preserves_read_modify_input() {
    let parsed = ParsedFile::parse(
        "a.go",
        "package p\nfunc f(p T, x int) {\np.f = x\np.f += 1\nuse(p)\n}\n",
        Language::Go,
    )
    .unwrap();
    let refs = Producer::file_refs(&parsed.path, &parsed);
    assert!(refs.edges.iter().any(|e| e.from.path.to_string() == "p.f"
        && e.from.line == 3
        && e.to.path.to_string() == "p.f"
        && e.to.line == 4));
    assert!(!refs.edges.iter().any(|e| e.from.path.to_string() == "p.f"
        && e.from.line == 3
        && e.to.path.to_string() == "p"
        && e.to.line == 5));
    assert!(refs.edges.iter().any(|e| e.from.path.to_string() == "p.f"
        && e.from.line == 4
        && e.to.path.to_string() == "p"
        && e.to.line == 5));
}

#[test]
fn go_field_recall_fallback_binding_is_uncertain() {
    let parsed = ParsedFile::parse(
        "a.go",
        "package p\nfunc f() {\np.f = 1\nuse(p)\n}\n",
        Language::Go,
    )
    .unwrap();
    let refs = Producer::file_refs(&parsed.path, &parsed);
    let edge = refs
        .edges
        .iter()
        .find(|e| {
            e.from.path.to_string() == "p.f"
                && e.from.line == 3
                && e.to.path.to_string() == "p"
                && e.to.line == 4
        })
        .unwrap();
    assert_eq!(
        refs.labels.get(&(edge.from.clone(), edge.to.clone())),
        Some(&FlowConfidence::NameOnly(
            crate::cpg::FlowDoubt::CfgIncomplete
        ))
    );
}

#[test]
fn go_field_recall_literal_key_value_and_field_name_guard() {
    for (literal, expect) in [("map[T]int{p: 1}", true), ("struct { p int }{p: 1}", false)] {
        let source = format!("package p\nfunc f(p T) {{\np.f = 1\nx := {literal}\nuse(x)\n}}\n");
        let parsed = ParsedFile::parse("a.go", &source, Language::Go).unwrap();
        let refs = Producer::file_refs(&parsed.path, &parsed);
        assert_eq!(
            refs.edges.iter().any(|e| e.from.path.to_string() == "p.f"
                && e.from.line == 3
                && e.to.path.to_string() == "p"
                && e.to.line == 4),
            expect,
            "{literal}"
        );
    }
}
