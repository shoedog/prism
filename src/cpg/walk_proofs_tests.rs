use std::collections::BTreeSet;
use std::process::{Command, Output};

use petgraph::graph::NodeIndex;

use super::object_literal_arg_tests::parse_files;
use super::trace_cuts::byte_span;
use super::trace_descent_tests::occurrence;
use super::trace_proofs::Rule;
use super::walk_proofs::{prove_rule, ProofKey, Verdict};
use super::{
    CodePropertyGraph, CpgContext, CpgEdge, CpgNode, FlowConfidence, LossReason, Relation,
    ReturnFlowMode, VarAccess,
};
use crate::languages::Language;
use crate::reasoning::order::AstOrderView;

const FILE: &str = "PRISM_TAINT_WALK_PROOFS_FILE";
const FILE_RULES: &str = "PRISM_TAINT_WALK_PROOFS";
const REAL: &str = "PRISM_TAINT_WALK_PROOFS_REAL";
const COMPARE: &str = "PRISM_TAINT_WALK_PROOFS_COMPARE";
const CASE: &str = "PRISM_IWALK_PR1_TEST_CASE";
const L01: &str = include_str!("../../tests/taint_parity/l01/a.js");
const L04: &str = include_str!("../../tests/taint_parity/l04/a.js");
const L05: &str = include_str!("../../tests/taint_parity/l05/a.py");
const FLOW: &str =
    "function g(p) { return p; }\nfunction f(t) {\n  const out = g(t);\n  sink(out);\n}\n";

fn child(case: &str, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args([
        "--exact",
        "cpg::walk_proofs_tests::walk_case",
        "--nocapture",
    ]);
    for name in [
        FILE,
        FILE_RULES,
        REAL,
        COMPARE,
        "PRISM_TAINT_CROSSING_TRACE",
    ] {
        command.env_remove(name);
    }
    command
        .env(CASE, case)
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

fn succeeded(output: Output) -> (String, String) {
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stdout}\n{stderr}");
    (stdout, stderr)
}

#[test]
fn walk_case() {
    use VarAccess::{Def, Use};
    let Ok(case) = std::env::var(CASE) else {
        return;
    };
    if case == "l01" {
        let files = parse_files(Language::JavaScript, &[("a", L01)], "js");
        let cpg = CodePropertyGraph::build(&files);
        let root = occurrence(&cpg, 166, Use);
        let other = occurrence(&cpg, L01.find("log(t)").unwrap() + 4, Use);
        let formal = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, Def);
        let sink = occurrence(&cpg, 97, Use);
        let view = AstOrderView::new(&cpg, &files);
        let trace = cpg.taint_trace_nodes(&[other, root], Some(&view));
        assert_eq!(byte_span(cpg.node(root)), (166, 167));
        assert_eq!(
            trace.parents_by_root[&(root, formal)],
            (root, Relation::CallDescent)
        );
        assert!(trace.frontier_by_root[&root].contains(&sink));
        assert!(!trace.frontier_by_root[&other].contains(&sink));
        println!("DEBUG {trace:?}");
        println!(
            "DESCENT from={:?} to={:?} root={:?}",
            cpg.trace_key(root),
            cpg.trace_key(formal),
            cpg.trace_key(root)
        );
        return;
    }
    if case == "l04" || case == "l04-compare" {
        let files = parse_files(Language::JavaScript, &[("a", L04)], "js");
        let mut cpg = CodePropertyGraph::build(&files);
        let inner = L04.find("g(t)").unwrap();
        let from = occurrence(&cpg, inner + 2, Use);
        let to = occurrence(&cpg, L04.find("g(p)").unwrap() + 2, Def);
        if case == "l04-compare" {
            // Isolate the proof hook by giving the legacy gate a singleton.
            // Keep the outer call in the caller's calls and in the AST.
            cpg.call_graph
                .callers
                .get_mut("g")
                .unwrap()
                .retain(|site| site.start_byte == inner);
            assert_eq!(cpg.binding_site(from, to).unwrap().start_byte, inner);
        }
        let order = AstOrderView::new(&cpg, &files);
        let trace = cpg.taint_trace_nodes(&[from], Some(&order));
        if case == "l04" {
            assert!(trace.boundary.iter().any(|hop| hop.from == from
                && hop.to == to
                && hop.reason == LossReason::AmbiguousSite));
        } else {
            assert_eq!(
                trace.parents_by_root[&(from, to)],
                (from, Relation::CallDescent)
            );
        }
        println!("DEBUG {trace:?}");
        return;
    }
    if case == "unknown" {
        let files = parse_files(Language::JavaScript, &[("a", L01)], "js");
        let mut cpg = CodePropertyGraph::build(&files);
        let template = occurrence(&cpg, 166, Use);
        let from = with_bounds(&mut cpg, template, (166, 166));
        let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, Def);
        cpg.graph
            .add_edge(from, to, CpgEdge::DataFlow(FlowConfidence::Exact));
        let view = AstOrderView::new(&cpg, &files);
        let trace = cpg.taint_trace_nodes(&[from], Some(&view));
        if std::env::var(REAL).unwrap().ends_with("+unknown") {
            assert!(trace.boundary.iter().any(|hop| hop.from == from
                && hop.to == to
                && hop.reason == LossReason::NoMatchingSite));
            assert!(!trace.frontier_by_root[&from].contains(&to));
        } else {
            assert_eq!(
                trace.parents_by_root[&(from, to)],
                (from, Relation::CallDescent)
            );
        }
        return;
    }

    let files = parse_files(Language::JavaScript, &[("a", FLOW)], "js");
    let context = CpgContext::build(&files, None);
    let cpg = &context.cpg;
    let def = occurrence(cpg, FLOW.find("f(t)").unwrap() + 2, Def);
    let actual = occurrence(cpg, FLOW.find("g(t)").unwrap() + 2, Use);
    let formal = occurrence(cpg, FLOW.find("g(p)").unwrap() + 2, Def);
    let returned = occurrence(cpg, FLOW.find("return p").unwrap() + 7, Use);
    let out = occurrence(cpg, FLOW.find("out =").unwrap(), Def);
    let order = AstOrderView::new(cpg, &files);
    if case == "clients" {
        let review =
            context.taint_forward_cfg_with_return_flow_labeled(&[("a.js".into(), 3)], None);
        let formal_loc = cpg.to_var_location(formal).unwrap();
        assert!(review.1.keys().any(|(_, target)| target == &formal_loc));
        let trace = cpg.taint_trace_nodes_with_mode(&[actual], Some(&order), ReturnFlowMode::On);
        assert!(trace.frontier_by_root[&actual].contains(&formal));
        let rewalk = cpg.taint_trace_nodes_excluding_with_mode(
            &[actual],
            Some(&order),
            &BTreeSet::new(),
            ReturnFlowMode::On,
        );
        assert_eq!(format!("{trace:?}"), format!("{rewalk:?}"));
        println!(
            "DESCENT from={:?} to={:?} root={:?}",
            cpg.trace_key(actual),
            cpg.trace_key(formal),
            cpg.trace_key(actual)
        );
        return;
    }
    assert!(["graph-only", "missing-file"].contains(&case.as_str()));
    let no_files = Default::default();
    let missing = AstOrderView::new(cpg, &no_files);
    let trace = cpg.taint_trace_nodes_with_mode(
        &[def, returned],
        (case == "missing-file").then_some(&missing),
        ReturnFlowMode::On,
    );
    assert!(
        trace.frontier_by_root[&def].contains(&actual),
        "lost local Def→Use"
    );
    for (root, from, to, relation) in [
        (def, actual, formal, Relation::CallDescent),
        (returned, returned, out, Relation::ReturnFlow),
    ] {
        assert!(
            !trace.frontier_by_root[&root].contains(&to),
            "crossed {relation:?}"
        );
        assert!(
            trace
                .boundary
                .iter()
                .any(|cut| (cut.root, cut.from, cut.to, cut.relation, cut.reason)
                    == (root, from, to, relation, LossReason::NoMatchingSite)),
            "{trace:?}"
        );
    }
}

#[test]
fn both_clients_pass_ast_site_view() {
    let (stdout, stderr) = succeeded(child("clients", &[(REAL, "i,ii"), (COMPARE, "1")]));
    let keys = stdout
        .lines()
        .find_map(|line| line.strip_prefix("DESCENT "))
        .unwrap();
    for rule in ["i", "ii"] {
        let prefix = format!(
            "taint-walk-proof-compare rule={rule} relation=CallDescent {keys} verdict=proved "
        );
        assert_eq!(
            stderr
                .lines()
                .filter(|line| line.starts_with(&prefix))
                .count(),
            3,
            "{stderr}"
        );
    }
}

#[test]
fn graph_only_refuses_both_crossing_directions_and_keeps_local_flow() {
    succeeded(child("graph-only", &[(REAL, "i")]));
}

#[test]
fn missing_endpoint_file_refuses_both_crossing_directions_and_keeps_local_flow() {
    succeeded(child("missing-file", &[(REAL, "ii")]));
}

#[test]
fn unknown_only_refuses_with_the_fail_closed_spelling() {
    for rule in ["i", "ii"] {
        succeeded(child("unknown", &[(REAL, rule)]));
        succeeded(child("unknown", &[(REAL, &format!("{rule}+unknown"))]));
    }
}

#[test]
fn l01_second_occurrence_is_proved_and_compare_is_observational() {
    let (off, _) = succeeded(child("l01", &[]));
    let (on, _) = succeeded(child("l01", &[(REAL, "i,ii")]));
    let (compared, stderr) = succeeded(child("l01", &[(COMPARE, "1")]));
    let trace = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("DEBUG "))
            .unwrap()
            .to_owned()
    };
    assert_eq!(trace(&off), trace(&on));
    assert_eq!(trace(&off), trace(&compared));
    let keys = off
        .lines()
        .find_map(|line| line.strip_prefix("DESCENT "))
        .unwrap();
    for rule in ["i", "ii"] {
        let prefix = format!(
            "taint-walk-proof-compare rule={rule} relation=CallDescent {keys} verdict=proved "
        );
        assert_eq!(
            stderr
                .lines()
                .filter(|line| line.starts_with(&prefix))
                .count(),
            1,
            "{stderr}"
        );
    }
}

#[test]
fn l04_compare_reports_ii_ambiguity_without_changing_the_walk() {
    let (_, guarded) = succeeded(child("l04", &[(COMPARE, "1")]));
    assert!(!guarded.contains("taint-walk-proof-compare"), "{guarded}");
    let (off, _) = succeeded(child("l04-compare", &[]));
    let (observed, stderr) = succeeded(child("l04-compare", &[(COMPARE, "1")]));
    let debug = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("DEBUG "))
            .unwrap()
            .to_owned()
    };
    assert_eq!(debug(&off), debug(&observed));
    assert!(
        stderr.lines().any(|line| line
            .starts_with("taint-walk-proof-compare rule=ii relation=CallDescent ")
            && line.contains(" verdict=refused reason=ambiguous_site family=nested_same_spelling")),
        "{stderr}"
    );
}

#[test]
fn unsupported_and_overlapping_real_rules_panic_by_name() {
    for (list, rule) in [
        ("iii", "iii"),
        ("iv", "iv"),
        ("v", "v"),
        ("return_flow", "return_flow"),
        ("return_input+unknown", "return_input"),
        ("all", "iii"),
        ("all+unknown", "iii"),
        ("i-v+unknown", "iii"),
    ] {
        let output = child("l01", &[(REAL, list)]);
        assert!(!output.status.success(), "{list}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(REAL) && stderr.contains(&format!("`{rule}`")),
            "{stderr}"
        );
    }
    let exclusions = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(exclusions.path(), r#"{"version":1,"rules":{"i":[]}}"#).unwrap();
    let output = child(
        "l01",
        &[
            (REAL, "i+unknown"),
            (FILE_RULES, "i"),
            (FILE, exclusions.path().to_str().unwrap()),
        ],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(REAL) && stderr.contains("`i`"), "{stderr}");
}

#[derive(Debug, PartialEq, Eq)]
enum Proof {
    Proved { site: (usize, usize), slot: usize },
    Refused(LossReason),
    Unknown,
}

fn prove(
    rule: &str,
    cpg: &CodePropertyGraph,
    view: &AstOrderView<'_>,
    from: NodeIndex,
    to: NodeIndex,
) -> Proof {
    let rule = match rule {
        "i" => Rule::I,
        "ii" => Rule::Ii,
        other => panic!("rule {other} has no proof in this PR"),
    };
    match prove_rule(rule, cpg, view, from, to) {
        Verdict::Proved(cert) => Proof::Proved {
            site: cert.site,
            slot: cert.slot,
        },
        Verdict::Refused(reason, _) => Proof::Refused(reason),
        Verdict::Unknown(_) => Proof::Unknown,
    }
}

fn seam(
    language: Language,
    ext: &str,
    source: &str,
    actual: usize,
    formal: usize,
    edit: impl FnOnce(&mut CodePropertyGraph, NodeIndex) -> NodeIndex,
    check: impl FnOnce(&CodePropertyGraph, &AstOrderView<'_>, NodeIndex, NodeIndex),
) {
    let files = parse_files(language, &[("a", source)], ext);
    let mut cpg = CodePropertyGraph::build(&files);
    let from = occurrence(&cpg, actual, VarAccess::Use);
    let to = occurrence(&cpg, formal, VarAccess::Def);
    let from = edit(&mut cpg, from);
    let view = AstOrderView::new(&cpg, &files);
    check(&cpg, &view, from, to);
}

fn unchanged(_: &mut CodePropertyGraph, from: NodeIndex) -> NodeIndex {
    from
}

fn with_bounds(
    cpg: &mut CodePropertyGraph,
    template: NodeIndex,
    span: (usize, usize),
) -> NodeIndex {
    let mut node = cpg.node(template).clone();
    let CpgNode::Variable {
        start_byte,
        end_byte,
        ..
    } = &mut node
    else {
        unreachable!()
    };
    (*start_byte, *end_byte) = span;
    cpg.graph.add_node(node)
}

#[test]
fn l05_first_actual_has_its_own_emission() {
    seam(
        Language::Python,
        "py",
        L05,
        74,
        L05.find("g(p)").unwrap() + 2,
        |cpg, from| {
            // Model a gate that has nominated the first site. The two calls
            // remain present for the emission proof.
            cpg.call_graph
                .callers
                .get_mut("g")
                .unwrap()
                .retain(|site| site.start_byte == 72);
            from
        },
        |cpg, view, from, to| {
            assert_eq!(byte_span(cpg.node(from)), (74, 78));
            assert!(
                !cpg.graph
                    .node_indices()
                    .any(|node| byte_span(cpg.node(node)) == (83, 87)),
                "unexpected second-site occurrence"
            );
            for rule in ["i", "ii"] {
                assert_eq!(
                    prove(rule, cpg, view, from, to),
                    Proof::Proved {
                        site: (72, 79),
                        slot: 0
                    },
                    "{rule}"
                );
            }
            let trace = cpg.taint_trace_nodes(&[from], Some(view));
            let sink = occurrence(cpg, 19, VarAccess::Use);
            assert_eq!(
                trace.parents_by_root[&(from, to)],
                (from, Relation::CallDescent)
            );
            assert!(trace.frontier_by_root[&from].contains(&sink));
        },
    );
}

#[test]
fn l04_nested_same_target_is_ambiguous_under_ii() {
    seam(
        Language::JavaScript,
        "js",
        L04,
        L04.find("g(t)").unwrap() + 2,
        L04.find("g(p)").unwrap() + 2,
        unchanged,
        |cpg, view, from, to| {
            let trace = cpg.taint_trace_nodes(&[from], Some(view));
            assert!(trace
                .boundary
                .iter()
                .any(|b| b.to == to && b.reason == LossReason::AmbiguousSite));
            assert_eq!(
                prove("ii", cpg, view, from, to),
                Proof::Refused(LossReason::AmbiguousSite)
            );
        },
    );
}

#[test]
fn receiver_span_cannot_supply_the_actual() {
    let source = "function g(p) { sink(p); }\nfunction f(t, x) { g(t.run(x)); }";
    let receiver = source.find("t.run").unwrap();
    let template = source.find("run(x)").unwrap() + 4;
    seam(
        Language::JavaScript,
        "js",
        source,
        template,
        11,
        |cpg, node| with_bounds(cpg, node, (receiver, receiver + 1)),
        |cpg, view, from, to| {
            for rule in ["i", "ii"] {
                assert_eq!(
                    prove(rule, cpg, view, from, to),
                    Proof::Refused(LossReason::NoMatchingSite),
                    "{rule}"
                );
            }
        },
    );
}

#[test]
fn intervening_function_literal_cannot_supply_the_outer_actual() {
    let source =
        "package p\nfunc g(p func()) { sink(p) }\nfunc f(t string) { g(func() { sink(t) }) }\n";
    let actual = source.find("sink(t)").unwrap() + 5;
    let formal = source.find("p func()").unwrap();
    // A producer may carry an enclosing caller id on these bytes.
    seam(
        Language::Go,
        "go",
        source,
        actual,
        formal,
        |cpg, from| {
            if let CpgNode::Variable {
                function,
                function_start_line,
                ..
            } = &mut cpg.graph[from]
            {
                *function = "f".into();
                *function_start_line = 3;
            }
            from
        },
        |cpg, view, from, to| {
            assert_eq!(
                prove("i", cpg, view, from, to),
                Proof::Refused(LossReason::NoMatchingSite)
            );
        },
    );
}

#[test]
fn keyword_name_is_distinct_from_keyword_value() {
    let source = "def g(p):\n    sink(p)\ndef f(t):\n    g(p=t)\n";
    let value = source.find("p=t").unwrap() + 2;
    seam(
        Language::Python,
        "py",
        source,
        value,
        6,
        |cpg, node| with_bounds(cpg, node, (value - 2, value - 1)),
        |cpg, view, from, to| {
            assert_eq!(
                prove("i", cpg, view, from, to),
                Proof::Refused(LossReason::NoMatchingSite)
            );
        },
    );
}

#[test]
fn zero_width_occurrence_has_no_value_evidence() {
    seam(
        Language::JavaScript,
        "js",
        L01,
        166,
        L01.find("g(x)").unwrap() + 2,
        |cpg, node| with_bounds(cpg, node, (166, 166)),
        |cpg, view, from, to| {
            assert_eq!(prove("i", cpg, view, from, to), Proof::Unknown);
        },
    );
}

#[test]
fn f_of_g_of_t_supplies_only_the_inner_actual() {
    let source =
        "function f(a) { sink(a); }\nfunction g(b) { return b; }\nfunction h(t) { f(g(t)); }\n";
    let site = source.find("g(t)").unwrap();
    seam(
        Language::JavaScript,
        "js",
        source,
        site + 2,
        11,
        unchanged,
        |cpg, view, from, outer| {
            let inner = occurrence(cpg, source.find("g(b)").unwrap() + 2, VarAccess::Def);
            assert_eq!(
                prove("i", cpg, view, from, inner),
                Proof::Proved {
                    site: (site, site + 4),
                    slot: 0
                }
            );
            assert_eq!(
                prove("i", cpg, view, from, outer),
                Proof::Refused(LossReason::NoMatchingSite)
            );
        },
    );
}

#[test]
fn disjoint_same_line_calls_have_two_distinct_byte_certificates() {
    let source = "function g(p) { sink(p); }\nfunction f(a, b) { g(a); g(b); }\n";
    let formal = source.find("g(p)").unwrap() + 2;
    for call in ["g(a)", "g(b)"] {
        let start = source.find(call).unwrap();
        seam(
            Language::JavaScript,
            "js",
            source,
            start + 2,
            formal,
            unchanged,
            |cpg, view, from, to| {
                assert_eq!(
                    prove("ii", cpg, view, from, to),
                    Proof::Proved {
                        site: (start, start + 4),
                        slot: 0
                    }
                );
            },
        );
    }
}

#[derive(Clone)]
struct CacheSite {
    target: crate::call_graph::FunctionId,
    site: (usize, usize),
    value: (usize, usize),
    slot: usize,
}

fn proof_cache_key(site: &CacheSite) -> ProofKey {
    ProofKey::new(
        Rule::I,
        Relation::CallDescent,
        ("a.js".into(), "f".into(), 1),
        site.site,
        site.value,
        (NodeIndex::new(1), NodeIndex::new(2)),
        (
            site.target.file.clone(),
            site.target.name.clone(),
            site.target.start_line,
        ),
        site.slot,
        None,
    )
}

#[test]
fn site_cache_key_includes_target_slot_and_both_span_ends() {
    let baseline = CacheSite {
        target: crate::call_graph::FunctionId {
            file: "lib.js".into(),
            name: "g".into(),
            start_line: 1,
            end_line: 3,
        },
        site: (20, 30),
        value: (24, 25),
        slot: 0,
    };
    let other_target = crate::call_graph::FunctionId {
        file: "other.js".into(),
        ..baseline.target.clone()
    };
    let collisions: Vec<_> = [
        (
            "target",
            CacheSite {
                target: other_target,
                ..baseline.clone()
            },
        ),
        (
            "site end",
            CacheSite {
                site: (20, 31),
                ..baseline.clone()
            },
        ),
        (
            "value end",
            CacheSite {
                value: (24, 26),
                ..baseline.clone()
            },
        ),
        (
            "slot",
            CacheSite {
                slot: 1,
                ..baseline.clone()
            },
        ),
    ]
    .into_iter()
    .filter_map(|(field, candidate)| {
        (proof_cache_key(&baseline) == proof_cache_key(&candidate)).then_some(field)
    })
    .collect();
    assert!(
        collisions.is_empty(),
        "colliding cache dimensions: {collisions:?}"
    );
}
