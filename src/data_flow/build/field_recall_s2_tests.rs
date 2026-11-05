//! S2 matrix (#639): receivers, closure capture reads and same-line pairs.
//! The programs are parsed, never executed. Rows 07, 11 and 28 live in
//! `field_recall_tests.rs`; their expected edges are in `expected.json` too.
use super::field_recall_tests::matrix;
use crate::ast::ParsedFile;
use crate::languages::Language;

macro_rules! cases {
    ($($test:ident => $case:literal),* $(,)?) => { $(
        #[test] fn $test() { matrix($case); }
    )* };
}
cases! {
    go_field_recall_s2_46 => "46_recv_prefix", go_field_recall_s2_47 => "47_recv_deref_paren",
    go_field_recall_s2_48 => "48_recv_value_var", go_field_recall_s2_49 => "49_recv_defer_go",
    go_field_recall_s2_50 => "50_recv_chain", go_field_recall_s2_51 => "51_recv_root_killed",
    go_field_recall_s2_52 => "52_recv_branch", go_field_recall_s2_53 => "53_recv_loop_carried",
    go_field_recall_s2_54 => "54_recv_field_call", go_field_recall_s2_55 => "55_recv_method_value",
    go_field_recall_s2_56 => "56_recv_import_qualifier",
    go_field_recall_s2_57 => "57_recv_unresolved_root", go_field_recall_s2_58 => "58_recv_sibling",
    go_field_recall_s2_59 => "59_recv_same_path", go_field_recall_s2_60 => "60_exec_cmd",
    go_field_recall_s2_61 => "61_cap_defer", go_field_recall_s2_62 => "62_cap_go",
    go_field_recall_s2_63 => "63_cap_write_after", go_field_recall_s2_64 => "64_cap_killed_before",
    go_field_recall_s2_65 => "65_cap_sibling", go_field_recall_s2_66 => "66_cap_shadow_param",
    go_field_recall_s2_67 => "67_cap_shadow_local", go_field_recall_s2_68 => "68_cap_writes_root",
    go_field_recall_s2_69 => "69_cap_writes_prefix", go_field_recall_s2_70 => "70_cap_writes_sibling",
    go_field_recall_s2_71 => "71_cap_nested", go_field_recall_s2_72 => "72_cap_loop",
    go_field_recall_s2_73 => "73_cap_prefix", go_field_recall_s2_74 => "74_sl_reverse",
    go_field_recall_s2_75 => "75_sl_reverse_loop", go_field_recall_s2_76 => "76_sl_sibling",
    go_field_recall_s2_77 => "77_sl_kill", go_field_recall_s2_78 => "78_sl_rhs",
    go_field_recall_s2_79 => "79_sl_tuple", go_field_recall_s2_80 => "80_sl_if_header",
    go_field_recall_s2_81 => "81_sl_switch_header", go_field_recall_s2_82 => "82_sl_for_init",
    go_field_recall_s2_83 => "83_sl_for_post", go_field_recall_s2_84 => "84_sl_for_post_acyclic",
    go_field_recall_s2_85 => "85_sl_write_after_root",
}

fn exec_cmd() -> crate::cpg::CodePropertyGraph {
    let source = include_str!("../../../tests/fixtures/go_field_write/60_exec_cmd/main.go");
    crate::cpg::CodePropertyGraph::build(&std::collections::BTreeMap::from([(
        "cmd.go".to_string(),
        ParsedFile::parse("cmd.go", source, Language::Go).unwrap(),
    )]))
}

#[test]
fn go_field_recall_s2_exec_cmd_taint() {
    let graph = exec_cmd();
    let trace = graph.taint_trace(&[("cmd.go".to_string(), 7)]);
    assert!(trace.warnings.is_empty(), "{:?}", trace.warnings);
    assert!(trace
        .frontier_by_root
        .values()
        .flatten()
        .any(|n| graph.node(*n).line() == 10));
}

#[test]
fn go_field_recall_s2_exec_cmd_chop() {
    use crate::cpg::{CpgEdge, CpgNode};
    let graph = exec_cmd();
    let at = |line: usize, path: &str| -> Vec<_> {
        graph
            .nodes_at("cmd.go", line)
            .into_iter()
            .filter(|&i| matches!(&graph.graph[i], CpgNode::Variable { path: p, .. } if p.to_string() == path))
            .collect()
    };
    let (src, sink) = (at(9, "cmd.Dir"), at(10, "cmd"));
    assert!(!src.is_empty() && !sink.is_empty());
    let on_path = graph.chop(&src, &sink, &|e| matches!(e, CpgEdge::DataFlow(_)));
    assert!(src.iter().all(|n| on_path.contains(n)) && sink.iter().all(|n| on_path.contains(n)));
}

#[test]
fn go_field_recall_s2_exec_cmd_leftflow() {
    let graph = exec_cmd();
    let def = graph
        .dfg
        .edges()
        .iter()
        .find(|e| e.from.line == 9 && e.from.path.to_string() == "cmd.Dir")
        .expect("cmd.Dir def")
        .from
        .clone();
    let reached = graph.dfg_forward_reachable(&def);
    assert!(reached
        .iter()
        .any(|u| u.path.to_string() == "cmd" && u.line == 10));
}
