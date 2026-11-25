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
    let edge = trace
        .boundary
        .iter()
        .find(|edge| edge.from == from && edge.to == to)
        .unwrap();
    assert_ne!(
        edge.joint_relaxations & super::relaxation(LossReason::NoMatchingSite),
        0
    );
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
fn return_not_entered_site_is_a_cut_not_a_boundary() {
    let mut trace = Trace::default();
    let [root, from, to] = [0, 1, 2].map(NodeIndex::new);
    use super::{BoundaryEdge, CrossingSite};
    let entered = CrossingSite {
        start_byte: 10,
        end_byte: 20,
        slot: Some(0),
    };
    let receiving = CrossingSite {
        start_byte: 30,
        end_byte: 40,
        slot: Some(0),
    };
    let identity = |site: CrossingSite| ("a.js".into(), site.start_byte, site.end_byte);
    let compare = super::ContinuationFrames::check_return;
    let entry = Some(identity(entered));
    let reason = compare(Some(&entry), Some(&identity(receiving))).unwrap_err();
    assert!(compare(Some(&entry), Some(&identity(entered))).is_ok());
    assert_eq!(
        compare(Some(&entry), Some(&("b.js".into(), 10, 20))),
        Err(LossReason::ReturnNotEnteredSite)
    );
    assert_eq!(compare(Some(&entry), None), Err(LossReason::NoMatchingSite));
    assert_eq!(
        compare(Some(&None), Some(&identity(receiving))),
        Err(LossReason::NoMatchingSite)
    );
    assert!(compare(None, Some(&identity(receiving))).is_ok());
    trace.record_refusal(BoundaryEdge {
        root,
        from,
        to,
        kind: BoundaryKind::CrossFunction,
        reason,
        relation: Relation::ReturnFlow,
        depth: 1,
        site: Some(receiving),
        edge_own_not_exact: false,
        joint_relaxations: 0,
    });
    assert!(trace.boundary.is_empty());
    assert_eq!(
        trace
            .cuts
            .iter()
            .map(|cut| (cut.root, cut.from, cut.to, cut.reason))
            .collect::<Vec<_>>(),
        [(root, from, to, LossReason::ReturnNotEnteredSite)]
    );
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
