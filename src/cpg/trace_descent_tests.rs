use std::collections::BTreeSet;

use petgraph::graph::NodeIndex;

use super::object_literal_arg_tests::build;
use super::trace::{BoundaryKind, Relation, Trace};
use super::{
    CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, FlowDoubt, LossReason, ReturnFlowMode,
    VarAccess,
};
use crate::languages::Language;

const L01: &str = include_str!("../../tests/taint_parity/l01/a.js");
const NAMEONLY: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CallNameOnly);

pub(super) fn occurrence(cpg: &CodePropertyGraph, start: usize, access: VarAccess) -> NodeIndex {
    let mut nodes = cpg.graph.node_indices().filter(|&id| {
        matches!(cpg.node(id), CpgNode::Variable { start_byte, access: actual, .. }
            if *start_byte == start && *actual == access)
    });
    let node = nodes.next().expect("missing occurrence");
    assert!(nodes.next().is_none(), "duplicate occurrence at {start}");
    node
}

fn add_binding(cpg: &mut CodePropertyGraph, from: NodeIndex, to: NodeIndex, label: FlowConfidence) {
    assert!(cpg
        .graph
        .edges_connecting(from, to)
        .all(|edge| !edge.weight().is_data_flow()));
    cpg.graph.add_edge(from, to, CpgEdge::DataFlow(label));
}

fn assert_boundary(trace: &Trace, from: NodeIndex, to: NodeIndex, reason: LossReason) {
    assert!(
        !trace.frontier_by_root[&from].contains(&to),
        "{reason:?}: crossed {from:?} → {to:?}"
    );
    assert!(
        trace.boundary.iter().any(
            |edge| (edge.root, edge.from, edge.to, edge.kind, edge.reason)
                == (from, from, to, BoundaryKind::CrossFunction, reason)
        ),
        "{reason:?}: missing refused crossing"
    );
}

#[test]
fn nameonly_callback_edge_not_crossed_through_same_line_exact_site() {
    let source = include_str!("../../tests/taint_parity/n01/a.js");
    let mut cpg = build(Language::JavaScript, &[("a", source)], "js");
    let from = occurrence(&cpg, source.find("cb(x)").unwrap() + 3, VarAccess::Use);
    let to = occurrence(&cpg, source.find("handler(p)").unwrap() + 8, VarAccess::Def);
    add_binding(&mut cpg, from, to, NAMEONLY);
    assert_boundary(
        &cpg.taint_trace_nodes(&[from], None),
        from,
        to,
        LossReason::NotExact,
    );
}

#[test]
fn byte_carrying_occurrence_matches_by_containment_only() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let from = occurrence(&cpg, L01.find("log(t)").unwrap() + 4, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    add_binding(&mut cpg, from, to, FlowConfidence::Exact);
    assert_boundary(
        &cpg.taint_trace_nodes(&[from], None),
        from,
        to,
        LossReason::NoMatchingSite,
    );
}

#[test]
fn log_then_g_still_descends_from_inner_occurrence() {
    let cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let from = occurrence(&cpg, 166, VarAccess::Use);
    assert!(matches!(
        cpg.node(from),
        CpgNode::Variable {
            start_byte: 166,
            end_byte: 167,
            ..
        }
    ));
    assert_eq!(&L01[164..168], "g(t)");
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    let sink = occurrence(&cpg, 97, VarAccess::Use);
    let trace = cpg.taint_trace_nodes(&[from], None);
    assert_eq!(
        trace.parents_by_root[&(from, to)],
        (from, Relation::CallDescent)
    );
    assert!(trace.frontier_by_root[&from].contains(&sink));
}

#[test]
fn exact_sibling_cache_cannot_admit_not_exact_binding() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let exact = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    // Identical site-proof inputs, separate binding edges with their own labels.
    let nameonly = cpg.graph.add_node(cpg.node(exact).clone());
    add_binding(&mut cpg, nameonly, to, NAMEONLY);
    let trace = cpg.taint_trace_nodes(&[exact, nameonly], None);
    assert_eq!(
        trace.parents_by_root[&(exact, to)],
        (exact, Relation::CallDescent)
    );
    assert_boundary(&trace, nameonly, to, LossReason::NotExact);
}

#[test]
fn proof_cache_distinguishes_occurrence_end() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let exact = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    let mut wider = cpg.node(exact).clone();
    let CpgNode::Variable { end_byte, .. } = &mut wider else {
        unreachable!()
    };
    *end_byte = 169;
    let wider = cpg.graph.add_node(wider);
    add_binding(&mut cpg, wider, to, FlowConfidence::Exact);
    let trace = cpg.taint_trace_nodes(&[exact, wider], None);
    assert_eq!(
        trace.parents_by_root[&(exact, to)],
        (exact, Relation::CallDescent)
    );
    assert_boundary(&trace, wider, to, LossReason::NoMatchingSite);
}

#[test]
fn two_sites_report_ambiguous_site() {
    for (language, ext, source, formal) in [
        (
            Language::Python,
            "py",
            include_str!("../../tests/taint_parity/l02/a.py"),
            "g(c)",
        ),
        (
            Language::Go,
            "go",
            include_str!("../../tests/taint_parity/l03/main.go"),
            "g(c string)",
        ),
    ] {
        let cpg = build(language, &[("a", source)], ext);
        let from = occurrence(&cpg, source.find("g(t)").unwrap() + 2, VarAccess::Use);
        let to = occurrence(&cpg, source.find(formal).unwrap() + 2, VarAccess::Def);
        assert_boundary(
            &cpg.taint_trace_nodes(&[from], None),
            from,
            to,
            LossReason::AmbiguousSite,
        );
    }
}

#[test]
fn empty_occurrence_at_the_exclusive_end_of_a_call_is_not_inside_it() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let exact = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    assert_eq!(&L01[164..168], "g(t)");
    let mut empty = cpg.node(exact).clone();
    let CpgNode::Variable {
        start_byte,
        end_byte,
        ..
    } = &mut empty
    else {
        unreachable!()
    };
    *start_byte = 168;
    *end_byte = 168;
    let empty = cpg.graph.add_node(empty);
    add_binding(&mut cpg, empty, to, FlowConfidence::Exact);
    assert_boundary(
        &cpg.taint_trace_nodes(&[empty], None),
        empty,
        to,
        LossReason::NoMatchingSite,
    );
}

#[test]
fn nameonly_binding_edge_into_the_walked_function_is_a_boundary_not_local_flow() {
    let source = include_str!("../../tests/taint_parity/n01/a.js");
    let mut cpg = build(Language::JavaScript, &[("a", source)], "js");
    let from = occurrence(&cpg, source.find("cb(x)").unwrap() + 3, VarAccess::Use);
    // A name-only edge from a use to another node of the SAME function (a callback that resolves
    // back into its own caller): it must not enter the frontier as local DataFlow.
    let local = cpg
        .graph
        .node_indices()
        .find(|&id| {
            id != from
                && matches!(cpg.node(id), CpgNode::Variable { access: VarAccess::Use, function, .. }
                    if matches!(cpg.node(from), CpgNode::Variable { function: f, .. } if f == function))
        })
        .expect("a second use in the same function");
    add_binding(&mut cpg, from, local, NAMEONLY);
    let trace = cpg.taint_trace_nodes(&[from], None);
    assert!(
        !trace.frontier_by_root[&from].contains(&local),
        "name-only edge became local flow"
    );
    assert!(trace.boundary.iter().any(|edge| (
        edge.root,
        edge.from,
        edge.to,
        edge.kind,
        edge.reason
    ) == (
        from,
        from,
        local,
        BoundaryKind::SelfFunctionParam,
        LossReason::SelfParam
    )));
}

#[test]
fn nameonly_site_reports_not_exact() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let exact = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    // The occurrence sits inside g(t): only the label can refuse it (cold cache, single root).
    let nameonly = cpg.graph.add_node(cpg.node(exact).clone());
    add_binding(&mut cpg, nameonly, to, NAMEONLY);
    assert_boundary(
        &cpg.taint_trace_nodes(&[nameonly], None),
        nameonly,
        to,
        LossReason::NotExact,
    );
}

#[test]
fn nomination_keeps_the_inclusive_base_predicate_so_an_outer_call_still_makes_it_ambiguous() {
    // `g(g(\n x))`: an empty occurrence at the inner call's exclusive end is nominated by BOTH sites
    // (base predicate), so the unique-candidate rule must not turn it into a single accepted site.
    let source = "function g(a) { sink(a); }\nfunction f(x) {\n  g(g(\n x));\n}\n";
    let mut cpg = build(Language::JavaScript, &[("a", source)], "js");
    let inner_end = source.find("x));").unwrap() + 2;
    let template = occurrence(&cpg, source.find("x));").unwrap(), VarAccess::Use);
    let to = occurrence(
        &cpg,
        source
            .find("g(a)")
            .map(|_| source.find("(a)").unwrap() + 1)
            .unwrap(),
        VarAccess::Def,
    );
    let mut empty = cpg.node(template).clone();
    let CpgNode::Variable {
        start_byte,
        end_byte,
        ..
    } = &mut empty
    else {
        unreachable!()
    };
    *start_byte = inner_end;
    *end_byte = inner_end;
    let empty = cpg.graph.add_node(empty);
    add_binding(&mut cpg, empty, to, FlowConfidence::Exact);
    assert_boundary(
        &cpg.taint_trace_nodes(&[empty], None),
        empty,
        to,
        LossReason::AmbiguousSite,
    );
}

#[test]
fn use_target_reports_target_not_binding() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let from = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, 97, VarAccess::Use);
    add_binding(&mut cpg, from, to, FlowConfidence::Exact);
    assert!(cpg
        .taint_trace_nodes(&[from], None)
        .boundary
        .iter()
        .any(|edge| (edge.root, edge.from, edge.to, edge.reason)
            == (from, from, to, LossReason::TargetNotBinding)));

    cpg.call_graph.callers.remove("g").unwrap();
    let trace = cpg.taint_trace_nodes(&[from], None);
    assert_boundary(&trace, from, to, LossReason::TargetNotBinding);
}

#[test]
fn no_site_reports_no_matching_site() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    cpg.call_graph.callers.remove("g").unwrap();
    let from = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    assert_boundary(
        &cpg.taint_trace_nodes(&[from], None),
        from,
        to,
        LossReason::NoMatchingSite,
    );
}

#[test]
fn third_descent_reports_depth_cap() {
    let source = include_str!("../../tests/taint_parity/f01/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let root = occurrence(&cpg, 180, VarAccess::Def);
    let from = occurrence(&cpg, source.find("c(y)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&cpg, source.find("c(z)").unwrap() + 2, VarAccess::Def);
    let trace = cpg.taint_trace_nodes(&[root], None);
    assert!(trace.frontier_by_root[&root].contains(&from));
    assert!(!trace.frontier_by_root[&root].contains(&to));
    assert!(trace
        .boundary
        .iter()
        .any(|edge| (edge.root, edge.from, edge.to, edge.reason)
            == (root, from, to, LossReason::DepthCap)));
}

#[test]
fn context_policy_refusal_reports_policy_ceiling() {
    use super::taint_policy_return_tests as fixture;
    use super::{CrossingCeiling, TaintCrossingPolicy};
    let cpg = &mut fixture::build(&[
        (fixture::MAIN, fixture::MAIN_SRC),
        (fixture::LIB, fixture::LIB_SRC),
    ]);
    let from = fixture::var_at(cpg, fixture::MAIN, 3, "user", VarAccess::Use);
    let to = fixture::var_at(cpg, fixture::LIB, 1, "p", VarAccess::Def);
    fixture::relabel(cpg, from, to, FlowConfidence::NameOnly(FlowDoubt::SameLine));
    let files = [fixture::LIB.to_string()].into();
    let policy = TaintCrossingPolicy::new(&files, CrossingCeiling::Exact);
    let unrestricted = fixture::trace(cpg, &fixture::sources(fixture::MAIN, 3), None);
    assert!(unrestricted.frontier_by_root[&from].contains(&to));
    let restricted = fixture::trace(cpg, &fixture::sources(fixture::MAIN, 3), Some(&policy));
    assert_boundary(&restricted, from, to, LossReason::PolicyCeiling);
}

#[test]
fn self_call_reports_self_param() {
    let src = "function f(x) {\n  f(x);\n}\n";
    let cpg = build(Language::JavaScript, &[("a", src)], "js");
    let from = occurrence(&cpg, src.rfind("f(x)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&cpg, src.find("f(x)").unwrap() + 2, VarAccess::Def);
    let trace = cpg.taint_trace_nodes(&[from], None);
    assert!(trace.boundary.iter().any(|edge| (
        edge.root,
        edge.from,
        edge.to,
        edge.kind,
        edge.reason
    ) == (
        from,
        from,
        to,
        BoundaryKind::SelfFunctionParam,
        LossReason::SelfParam
    )));
}

#[test]
fn self_return_reports_self_return() {
    let source = include_str!("../../tests/taint_parity/s1-js-self-return/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let root = occurrence(&cpg, 83, VarAccess::Def);
    let from = occurrence(&cpg, source.find("return t").unwrap() + 7, VarAccess::Use);
    let to = occurrence(&cpg, 132, VarAccess::Def);
    let trace = cpg.taint_trace_nodes_with_mode(&[root], None, ReturnFlowMode::On);
    assert!(trace.boundary.iter().any(|edge| (
        edge.root,
        edge.from,
        edge.to,
        edge.kind,
        edge.reason
    ) == (
        root,
        from,
        to,
        BoundaryKind::SelfFunctionReturn,
        LossReason::SelfReturn
    )));
}

#[test]
fn depth_zero_cfg_cut_is_recorded() {
    let source = include_str!("../../tests/taint_parity/w05/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let root = occurrence(&cpg, source.find("v = p").unwrap(), VarAccess::Def);
    let from = occurrence(&cpg, source.find("return v").unwrap() + 7, VarAccess::Use);
    let trace = cpg.taint_trace_nodes_with_mode(&[root], None, ReturnFlowMode::On);
    assert!(trace.frontier_by_root[&root].contains(&from));
    assert!(trace.cuts.iter().any(|cut| cut.root == root
        && cut.from == from
        && matches!(cpg.node(cut.to), CpgNode::ReturnValue { child_slot: 0, .. })
        && cut.reason == LossReason::CfgPruned));
}

#[test]
fn suppressed_shortcut_is_recorded() {
    let source = include_str!("../../tests/taint_parity/x01/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let root = occurrence(&cpg, 82, VarAccess::Def);
    let from = occurrence(&cpg, source.find("forget(t)").unwrap() + 7, VarAccess::Use);
    let to = occurrence(&cpg, source.find("z=forget").unwrap(), VarAccess::Def);
    let trace = cpg.taint_trace_nodes_with_mode(&[root], None, ReturnFlowMode::On);
    assert!(!trace.frontier_by_root[&root].contains(&to));
    assert!(trace
        .cuts
        .iter()
        .any(|cut| (cut.root, cut.from, cut.to, cut.reason)
            == (root, from, to, LossReason::ShortcutSuppressed)));
    assert!(!trace
        .boundary
        .iter()
        .any(|edge| edge.root == root && edge.to == to));
}

#[test]
fn added_loss_reasons_use_their_wire_names() {
    for (reason, name) in [
        (LossReason::UncertifiedFlow, "uncertified_flow"),
        (LossReason::MatcherAdmission, "matcher_admission"),
        (LossReason::UnprovenReturnInput, "unproven_return_input"),
    ] {
        assert_eq!(serde_json::to_value(reason).unwrap(), name);
    }
}

#[test]
fn crossing_trace_lists_accepted_and_refused_hops() {
    const CHILD: &str = "PRISM_I1_CROSSING_TRACE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let source =
            "function g(x) { sink(x); }\nfunction f(t) {\n  g(t);\n  g(t); g('fixed');\n}\n";
        let cpg = build(Language::JavaScript, &[("a", source)], "js");
        let first = occurrence(&cpg, source.find("g(t)").unwrap() + 2, VarAccess::Use);
        let second = occurrence(&cpg, source.rfind("g(t)").unwrap() + 2, VarAccess::Use);
        let trace = cpg.taint_trace_nodes(&[first, second], None);
        assert_eq!(trace.boundary.len(), 1);
        return;
    }
    for on in [false, true] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "cpg::trace_descent_tests::crossing_trace_lists_accepted_and_refused_hops",
            "--nocapture",
        ]);
        command
            .env(CHILD, "1")
            .env_remove("PRISM_TAINT_CROSSING_TRACE");
        if on {
            command.env("PRISM_TAINT_CROSSING_TRACE", "1");
        }
        let child = command.output().unwrap();
        assert!(
            child.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );
        let stderr = String::from_utf8(child.stderr).unwrap();
        let lines: Vec<_> = stderr.lines().collect();
        assert_eq!(lines.len(), if on { 2 } else { 0 }, "{stderr}");
        if on {
            for line in &lines {
                for field in [
                    "root=",
                    "from=",
                    "to=",
                    "relation=CallDescent",
                    "site=",
                    "grades=",
                    "depth=0",
                ] {
                    assert!(line.contains(field), "{line}");
                }
            }
            assert!(lines[0].starts_with("taint-crossing "));
            assert!(lines[0].contains("grades=[Exact]"));
            assert!(lines[1].starts_with("taint-refusal "));
            assert!(lines[1].contains("reason=ambiguous_site"));
        }
    }
}

#[test]
fn crossing_trace_separates_traversed_return_input_from_crossings() {
    const CHILD: &str = "PRISM_I1_RETURN_INPUT_TRACE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let source = include_str!("../../tests/taint_parity/w05/a.js");
        let cpg = build(Language::JavaScript, &[("a", source)], "js");
        let root = occurrence(&cpg, source.find("f(y)").unwrap() + 2, VarAccess::Use);
        let parsed = crate::ast::ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
        let files = [("a.js".into(), parsed)].into();
        let order = crate::reasoning::order::AstOrderView::new(&cpg, &files);
        let trace = cpg.taint_trace_nodes_with_mode(&[root], Some(&order), ReturnFlowMode::On);
        assert!(trace
            .parents_by_root
            .values()
            .any(|(_, relation)| *relation == Relation::ReturnInput));
        assert!(trace
            .parents_by_root
            .values()
            .any(|(_, relation)| *relation == Relation::ReturnFlow));
        return;
    }
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "cpg::trace_descent_tests::crossing_trace_separates_traversed_return_input_from_crossings", "--nocapture"])
        .env(CHILD, "1")
        .env("PRISM_TAINT_CROSSING_TRACE", "1")
        .output().unwrap();
    assert!(
        child.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    let stderr = String::from_utf8(child.stderr).unwrap();
    let crossings = stderr
        .lines()
        .filter(|line| line.starts_with("taint-crossing "))
        .collect::<Vec<_>>();
    assert_eq!(crossings.len(), 2, "{stderr}");
    assert!(
        crossings[0].contains("relation=CallDescent") && crossings[0].contains(" slot=0 "),
        "{stderr}"
    );
    assert!(
        crossings[1].contains("relation=ReturnFlow") && crossings[1].contains(" slot=0 "),
        "{stderr}"
    );
    let inputs = stderr
        .lines()
        .filter(|line| line.starts_with("taint-return-input "))
        .collect::<Vec<_>>();
    assert_eq!(inputs.len(), 1, "{stderr}");
    for field in ["relation=ReturnInput", "component=0", "return=", "depth=1"] {
        assert!(inputs[0].contains(field), "{stderr}");
    }
}

#[test]
fn loss_reason_display_is_its_wire_name() {
    use LossReason::*;
    let all = [
        SelfParam,
        SelfReturn,
        ReturnNotEnteredSite,
        DepthCap,
        TargetNotBinding,
        NoMatchingSite,
        AmbiguousSite,
        NotExact,
        UnprovenSlot,
        PolicyCeiling,
        UnprovenReturnInput,
        ShortcutSuppressed,
        CfgPruned,
        Unmodelled,
        NoGraphEdge,
        ReviewNoReturnFlow,
        ReviewNoRecoveredDefUse,
        ReviewCfgFilter,
        ReviewContextPolicy,
        SanitizerModel,
        OperandGranularity,
        UncertifiedFlow,
        MatcherAdmission,
    ];
    // The match is exhaustive (no wildcard arm), so a new variant forces this list to be extended.
    for reason in all {
        let _: () = match reason {
            SelfParam
            | SelfReturn
            | ReturnNotEnteredSite
            | DepthCap
            | TargetNotBinding
            | NoMatchingSite
            | AmbiguousSite
            | NotExact
            | UnprovenSlot
            | PolicyCeiling
            | UnprovenReturnInput
            | ShortcutSuppressed
            | CfgPruned
            | Unmodelled
            | NoGraphEdge
            | ReviewNoReturnFlow
            | ReviewNoRecoveredDefUse
            | ReviewCfgFilter
            | ReviewContextPolicy
            | SanitizerModel
            | OperandGranularity
            | UncertifiedFlow
            | MatcherAdmission => (),
        };
        let wire = serde_json::to_value(reason).unwrap();
        assert_eq!(wire.as_str().unwrap(), reason.to_string(), "{reason:?}");
    }
    let names: std::collections::BTreeSet<_> = all.iter().map(ToString::to_string).collect();
    assert_eq!(names.len(), all.len(), "display names must be distinct");
}

const ASCENT_EXACT: &str = "function f(p) {\n  const v = p;\n  return v;\n}\nfunction entry(y) {\n  const out = f(y);\n  exec(out);\n}\n";
const ASCENT_NAMEONLY: &str = "class K {\n  f(p) {\n    const v = p;\n    return v;\n  }\n}\nfunction entry(obj, y) {\n  const out = obj.f(y);\n  exec(out);\n}\n";

/// The ascent is judged from the call site alone: the same return edge ascends when its site is
/// a singleton Exact and is refused as `not_exact` when the site only demotes to the owner.
#[test]
fn return_ascent_through_a_nameonly_site_is_refused_as_not_exact() {
    let ascend = |source: &str, drawn: bool| {
        let mut cpg = build(Language::JavaScript, &[("a", source)], "js");
        let from = occurrence(&cpg, source.find("return v").unwrap() + 7, VarAccess::Use);
        let to = occurrence(&cpg, source.find("out =").unwrap(), VarAccess::Def);
        let has_edge = cpg
            .graph
            .edges_connecting(from, to)
            .any(|edge| matches!(edge.weight(), CpgEdge::ReturnFlow { .. }));
        assert_eq!(has_edge, drawn, "only an Exact site has a drawn edge");
        if !has_edge {
            cpg.graph.add_edge(
                from,
                to,
                CpgEdge::ReturnFlow {
                    suppress_shortcut: true,
                },
            );
        }
        let trace = cpg.taint_trace_nodes_with_mode(&[from], None, ReturnFlowMode::On);
        (from, to, trace)
    };

    let (from, to, exact) = ascend(ASCENT_EXACT, true);
    assert!(exact.frontier_by_root[&from].contains(&to));
    assert!(!exact.boundary.iter().any(|edge| edge.to == to));

    let (from, to, demoted) = ascend(ASCENT_NAMEONLY, false);
    assert!(!demoted.frontier_by_root[&from].contains(&to));
    assert!(
        demoted.boundary.iter().any(
            |edge| (edge.root, edge.from, edge.to, edge.kind, edge.reason)
                == (
                    from,
                    from,
                    to,
                    BoundaryKind::CrossFunction,
                    LossReason::NotExact
                )
        ),
        "{:?}",
        demoted.boundary
    );
}

/// A hop whose origin is not a variable occurrence has no byte span to place in a call site, so it
/// is refused as `target_not_binding`; the same hop from a variable is judged by the site proof.
#[test]
fn descent_from_a_return_value_is_refused_as_target_not_binding() {
    let source = include_str!("../../tests/taint_parity/w05/a.js");
    let refused = |from_return_value: bool| {
        let mut cpg = build(Language::JavaScript, &[("a", source)], "js");
        let root = occurrence(&cpg, source.find("f(y)").unwrap() + 2, VarAccess::Use);
        let returned = occurrence(&cpg, source.find("return v").unwrap() + 7, VarAccess::Use);
        let ret = cpg
            .graph
            .node_indices()
            .find(|&id| matches!(cpg.node(id), CpgNode::ReturnValue { .. }))
            .expect("composite return has a return value node");
        let to = occurrence(&cpg, source.find("out =").unwrap(), VarAccess::Def);
        let from = if from_return_value { ret } else { returned };
        add_binding(&mut cpg, from, to, FlowConfidence::Exact);
        let trace = cpg.taint_trace_nodes_with_mode(&[root], None, ReturnFlowMode::On);
        let reasons: Vec<_> = trace
            .boundary
            .iter()
            .filter(|edge| (edge.from, edge.to) == (from, to))
            .map(|edge| (edge.kind, edge.reason))
            .collect();
        (trace, ret, reasons)
    };

    let (trace, ret, reasons) = refused(true);
    let root = *trace.frontier_by_root.keys().next().unwrap();
    assert!(
        trace.frontier_by_root[&root].contains(&ret),
        "the return value must be walked for its hop to be judged"
    );
    assert_eq!(
        reasons,
        [(BoundaryKind::CrossFunction, LossReason::TargetNotBinding)]
    );

    let (_, _, reasons) = refused(false);
    assert_eq!(
        reasons,
        [(BoundaryKind::CrossFunction, LossReason::NoMatchingSite)]
    );
}

#[test]
fn crossing_slot_needs_a_view_that_proves_it_and_a_site_of_the_same_file() {
    use super::{OrderingDecision, SameLineOrderView};
    struct Unproven;
    impl SameLineOrderView for Unproven {
        fn admit_same_line_recovered_def_use(
            &self,
            _def: NodeIndex,
            _use: NodeIndex,
        ) -> OrderingDecision {
            OrderingDecision::Admit
        }
    }

    let source = include_str!("../../tests/taint_parity/w05/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let out = occurrence(&cpg, source.find("out =").unwrap(), VarAccess::Def);
    let site = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "f")
        .expect("f(y) call site")
        .clone();
    let parsed = crate::ast::ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let files = [("a.js".to_string(), parsed)].into();
    let proven = crate::reasoning::order::AstOrderView::new(&cpg, &files);

    assert_eq!(
        proven.crossing_slot(out, &site, Relation::ReturnFlow),
        Some(0)
    );
    assert_eq!(
        Unproven.crossing_slot(out, &site, Relation::ReturnFlow),
        None
    );

    let mut elsewhere = site.clone();
    elsewhere.caller.file = "b.js".into();
    assert_eq!(
        proven.crossing_slot(out, &elsewhere, Relation::ReturnFlow),
        None
    );
}

#[test]
fn excluding_wrapper_skips_the_excluded_hop_and_matches_the_mode_variant() {
    let source = include_str!("../../tests/taint_parity/w05/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let root = occurrence(&cpg, source.find("f(y)").unwrap() + 2, VarAccess::Use);
    let formal = occurrence(&cpg, source.find("f(p)").unwrap() + 2, VarAccess::Def);

    let whole = cpg.taint_trace_nodes(&[root], None);
    assert!(whole.frontier_by_root[&root].contains(&formal));

    let none = BTreeSet::new();
    let unexcluded = cpg.taint_trace_nodes_excluding(&[root], None, &none);
    assert_eq!(unexcluded.frontier_by_root, whole.frontier_by_root);

    let excluded: BTreeSet<_> = [(root, formal)].into();
    let trace = cpg.taint_trace_nodes_excluding(&[root], None, &excluded);
    assert!(!trace.frontier_by_root[&root].contains(&formal));
    assert!(!trace.boundary.iter().any(|edge| edge.to == formal));
    let with_mode =
        cpg.taint_trace_nodes_excluding_with_mode(&[root], None, &excluded, ReturnFlowMode::Off);
    assert_eq!(trace.frontier_by_root, with_mode.frontier_by_root);
}

#[test]
fn byte_span_reads_the_span_of_every_node_kind() {
    let source = include_str!("../../tests/taint_parity/w05/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let mut kinds = BTreeSet::new();
    for id in cpg.graph.node_indices() {
        let (expected, kind) = match cpg.node(id) {
            CpgNode::Function {
                start_byte,
                end_byte,
                ..
            } => ((*start_byte, *end_byte), "function"),
            CpgNode::Statement {
                start_byte,
                end_byte,
                ..
            } => ((*start_byte, *end_byte), "statement"),
            CpgNode::Variable {
                start_byte,
                end_byte,
                ..
            } => ((*start_byte, *end_byte), "variable"),
            CpgNode::ReturnValue {
                start_byte,
                end_byte,
                ..
            } => ((*start_byte, *end_byte), "return_value"),
        };
        assert_eq!(super::trace_cuts::byte_span(cpg.node(id)), expected);
        assert!(expected.0 <= expected.1, "{kind} span is ordered");
        kinds.insert(kind);
    }
    assert_eq!(
        kinds.len(),
        4,
        "fixture must hold every node kind: {kinds:?}"
    );
}

#[test]
fn return_neighbours_list_return_inputs_before_return_flows() {
    let returned_with = |mode| {
        let mut cpg = build(Language::JavaScript, &[("a", ASCENT_EXACT)], "js");
        let from = occurrence(
            &cpg,
            ASCENT_EXACT.find("return v").unwrap() + 7,
            VarAccess::Use,
        );
        let flow = occurrence(&cpg, ASCENT_EXACT.find("out =").unwrap(), VarAccess::Def);
        // A later node than the flow target: only the relation rank can put it first.
        let input = occurrence(&cpg, ASCENT_EXACT.find("f(y)").unwrap() + 2, VarAccess::Use);
        assert!(flow.index() < input.index());
        cpg.graph.add_edge(from, input, CpgEdge::ReturnInput);
        let returned: Vec<_> = cpg
            .taint_neighbors(from, mode)
            .into_iter()
            .filter(|(_, relation, _, _)| {
                matches!(relation, Relation::ReturnInput | Relation::ReturnFlow)
            })
            .map(|(target, relation, _, _)| (target, relation))
            .collect();
        (flow, input, returned)
    };

    let (flow, input, on) = returned_with(ReturnFlowMode::On);
    assert_eq!(
        on,
        [(input, Relation::ReturnInput), (flow, Relation::ReturnFlow)]
    );
    let (_, _, off) = returned_with(ReturnFlowMode::Off);
    assert!(off.is_empty(), "{off:?}");
}

fn call_site_bytes(cpg: &CodePropertyGraph, callee: &str) -> (usize, usize) {
    let site = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == callee)
        .expect("call site");
    (site.start_byte, site.end_byte)
}

/// A refused return hop still names its call site, but only when the syntax view proves which
/// call assigns the target.
#[test]
fn refused_return_hop_names_its_site_only_with_a_proving_view() {
    let source = include_str!("../../tests/taint_parity/w05/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let from = occurrence(&cpg, source.find("return v").unwrap() + 7, VarAccess::Use);
    let to = occurrence(&cpg, source.find("out =").unwrap(), VarAccess::Def);
    let parsed = crate::ast::ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let files = [("a.js".to_string(), parsed)].into();
    let order = crate::reasoning::order::AstOrderView::new(&cpg, &files);
    let gate = || Err(LossReason::NotExact);

    let (start_byte, end_byte) = call_site_bytes(&cpg, "f");
    let evidence = cpg
        .crossing_site(from, to, Relation::ReturnFlow, gate(), Some(&order))
        .expect("an assigned call is named");
    assert_eq!(
        evidence.location,
        super::CrossingSite {
            start_byte,
            end_byte,
            slot: Some(0)
        }
    );
    assert!(cpg
        .crossing_site(from, to, Relation::ReturnFlow, gate(), None)
        .is_none());
}

/// Site evidence for a call hop needs variable endpoints; any other endpoint yields none, never a
/// guess.
#[test]
fn call_hop_site_evidence_needs_variable_endpoints() {
    let source = include_str!("../../tests/taint_parity/w05/a.js");
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let from = occurrence(&cpg, source.find("f(y)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&cpg, source.find("f(p)").unwrap() + 2, VarAccess::Def);
    let ret = cpg
        .graph
        .node_indices()
        .find(|&id| matches!(cpg.node(id), CpgNode::ReturnValue { .. }))
        .expect("composite return has a return value node");
    let gate = || Err(LossReason::TargetNotBinding);

    let (start_byte, end_byte) = call_site_bytes(&cpg, "f");
    let evidence = cpg
        .crossing_site(from, to, Relation::CallDescent, gate(), None)
        .expect("a variable hop is named by its call");
    assert_eq!(
        (evidence.location.start_byte, evidence.location.end_byte),
        (start_byte, end_byte)
    );
    for (origin, target) in [(ret, to), (from, ret)] {
        assert!(
            cpg.crossing_site(origin, target, Relation::CallDescent, gate(), None)
                .is_none(),
            "{origin:?} -> {target:?}"
        );
    }
}

/// A return edge whose source has no owning function cannot witness a suppressed shortcut, and
/// does not hide the one that can.
#[test]
fn shortcut_witness_skips_a_return_edge_without_an_owner() {
    let shortcut = |source: &str| {
        let mut cpg = build(Language::JavaScript, &[("a", source)], "js");
        let use_ = occurrence(&cpg, source.find("f(y)").unwrap() + 2, VarAccess::Use);
        let def = occurrence(&cpg, source.find("out =").unwrap(), VarAccess::Def);
        let statement = cpg
            .graph
            .node_indices()
            .find(|&id| matches!(cpg.node(id), CpgNode::Statement { .. }))
            .expect("statement node");
        cpg.graph.add_edge(
            statement,
            def,
            CpgEdge::ReturnFlow {
                suppress_shortcut: true,
            },
        );
        cpg.taint_neighbors(use_, ReturnFlowMode::On)
            .into_iter()
            .find(|(target, relation, _, _)| {
                *target == def && *relation == Relation::AssignmentPropagation
            })
            .map(|(_, _, _, site)| site.map(|site| (site.start_byte, site.end_byte)))
            .expect("assignment propagation to the call's result")
    };

    let exact = shortcut(ASCENT_EXACT);
    assert!(exact.is_some(), "the real return edge still witnesses");
    assert_eq!(shortcut(ASCENT_NAMEONLY), None);
}
