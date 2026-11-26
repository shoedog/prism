//! The matrix evidence helpers must fail on evidence that does not fit (lane 3 S2b, codecov
//! patch). Each case perturbs one input of a control that the helper accepts.

use super::object_literal_arg_tests::parse_files;
use super::props_s2_matrix_evidence::{check, guard_counterfactual};
use super::props_s2_test_support::{capture, Assembly};
use super::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, VarAccess};
use crate::languages::Language;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

fn built(files: &[(&str, &str)]) -> (CodePropertyGraph, Assembly) {
    let parsed = parse_files(Language::JavaScript, files, "js");
    capture(&parsed, || CodePropertyGraph::build(&parsed))
}

fn err(result: Option<Result<String, String>>) -> String {
    result
        .expect("a registered row")
        .expect_err("evidence must not fit")
}

fn variable(g: &CodePropertyGraph, owner: &str, name: &str, access: VarAccess) -> NodeIndex {
    g.graph
        .node_indices()
        .find(|&i| {
            matches!(&g.graph[i], CpgNode::Variable { function, path, access: a, .. }
                if function == owner && path.to_string() == name && *a == access)
        })
        .expect("variable node")
}

const D: &str = "function D({a}) {\n  return use(a);\n}\n";

#[test]
fn resolved_site_evidence_rejects_missing_ambiguous_and_misplaced_targets() {
    let id = "d1-rewritten-argument";
    let ok = format!("{D}function App(x) {{\n  return D({{a: x}});\n}}\n");
    let (g, _) = built(&[("m", &ok)]);
    assert!(check(id, "", "js", &g, None).unwrap().is_ok());
    // No recorded site, two recorded sites, an unresolved callee and another file extension.
    let none = format!("{D}function App(x) {{\n  return x;\n}}\n");
    let (g, _) = built(&[("m", &none)]);
    assert!(err(check(id, "", "js", &g, None)).contains("expected one recorded site"));
    let twice = format!("{D}function App(x) {{\n  D({{a: x}});\n  return D({{a: x}});\n}}\n");
    let (g, _) = built(&[("m", &twice)]);
    assert!(err(check(id, "", "js", &g, None)).contains("expected one recorded site"));
    let missing = "function App(x) {\n  return D({a: x});\n}\n";
    let (g, _) = built(&[("m", missing)]);
    assert!(err(check(id, "", "js", &g, None)).contains("resolver returned"));
    let (g, _) = built(&[("m", &ok)]);
    assert!(err(check(id, "", "ts", &g, None)).contains("unexpected target"));
}

#[test]
fn namespace_export_evidence_rejects_a_proved_or_unqualified_candidate() {
    let caller = "import * as api from \"./ctx\";\nfunction A(x) {\n  api.send({a: x});\n}\n";
    let unexported = "function send({a}) {\n  return use(a);\n}\n";
    let (g, _) = built(&[("m", caller), ("ctx", unexported)]);
    assert!(check("l22", "", "js", &g, None).unwrap().is_ok());
    // The candidate has the export proof the refusal row says it must lack.
    let exported = "export function send({a}) {\n  return use(a);\n}\n";
    let (g, _) = built(&[("m", caller), ("ctx", exported)]);
    assert!(err(check("l22", "", "js", &g, None)).contains("unexpectedly has export proof"));
    // The row spells its qualifier `api`; another namespace spelling is not the row's shape.
    let renamed = "import * as ns from \"./ctx\";\nfunction A(x) {\n  ns.send({a: x});\n}\n";
    let (g, _) = built(&[("m", renamed), ("ctx", unexported)]);
    assert!(err(check("l22", "", "js", &g, None)).contains("value namespace qualifier missing"));
    // The row's own resolved-target precondition propagates.
    let (g, _) = built(&[
        ("m", "function A(x) {\n  return x;\n}\n"),
        ("ctx", unexported),
    ]);
    assert!(err(check("l22", "", "js", &g, None)).contains("expected one recorded site"));
}

const BARE_FORMAL: &str =
    "function P(p) {\n  observe(p); return wrap(p);\n}\nfunction App(x) {\n  return P({a: x});\n}\n";

#[test]
fn selected_shadow_evidence_rejects_unknown_cases_missing_nodes_and_unreached_uses() {
    let id = "d2-literal-selected-read";
    let (g, asm) = built(&[("m", BARE_FORMAL)]);
    assert!(check(id, "bare-formal", "js", &g, Some(&asm))
        .unwrap()
        .is_ok());
    assert!(err(check(id, "bogus", "js", &g, Some(&asm))).contains("unknown selected shadow case"));
    // The member-formal bytes name no Use in this fixture.
    assert!(err(check(id, "member-formal", "js", &g, Some(&asm))).contains("expected one P:p"));
    // The formal's reaching edge is the evidence: without it the precondition is unmet.
    let (g, mut asm) = built(&[("m", BARE_FORMAL)]);
    let pre = &mut asm.pre_binding.as_mut().unwrap().graph;
    let formal = variable(&g, "P", "p", VarAccess::Def);
    let edges: Vec<_> = pre
        .edges(formal)
        .filter(|e| e.weight().is_data_flow())
        .map(|e| e.id())
        .collect();
    assert!(!edges.is_empty());
    for edge in edges {
        pre.remove_edge(edge);
    }
    assert!(err(check(id, "bare-formal", "js", &g, Some(&asm))).contains("does not reach"));
}

const FOREIGN: [(&str, &str); 2] = [
    (
        "m",
        "export const App = x => {\n  return D({a: x});\n};\nfunction D({a}) {\n  Ghost(a);\n}\n",
    ),
    (
        "other",
        "export function Ghost(g) {\n  return observe(g);\n}\n",
    ),
];
const IMPORTED: [(&str, &str); 2] = [
    (
        "m",
        "import {Ghost} from './other';\nexport const App = x => {\n  return D({a: x});\n};\nfunction D({a}) {\n  Ghost(a);\n}\n",
    ),
    FOREIGN[1],
];
const GUARD: &str = "d2-guard-bare-arrow-literal";

#[test]
fn guard_seed_evidence_rejects_wrong_flags_and_a_broken_literal_source() {
    let (g, asm) = built(&FOREIGN);
    assert!(check(GUARD, "foreign", "js", &g, Some(&asm))
        .unwrap()
        .is_ok());
    let (gi, asm_i) = built(&IMPORTED);
    assert!(check(GUARD, "imported", "js", &gi, Some(&asm_i))
        .unwrap()
        .is_ok());
    // Seeds recorded for the foreign build do not belong to the imported graph's claim.
    assert!(err(check(GUARD, "imported", "js", &gi, Some(&asm))).contains("wrong guard seed flags"));
    // Dropping the pending route also fails the flag check for the foreign case.
    let (g, mut asm) = built(&FOREIGN);
    asm.pending.clear();
    assert!(err(check(GUARD, "foreign", "js", &g, Some(&asm))).contains("wrong guard seed flags"));
    // The App formal must reach the literal source before the guard runs.
    let (g, mut asm) = built(&FOREIGN);
    let input = asm.guard_input.as_mut().unwrap();
    let def = variable(&g, "App", "x", VarAccess::Def);
    let edges: Vec<_> = input
        .graph
        .edges(def)
        .filter(|e| e.weight().is_data_flow())
        .map(|e| e.id())
        .collect();
    for edge in edges {
        input.graph.remove_edge(edge);
    }
    assert!(
        err(check(GUARD, "foreign", "js", &g, Some(&asm))).contains("fails to reach the literal")
    );
}

#[test]
fn guard_counterfactual_rejects_missing_hops_wrong_seed_paths_and_changed_counts() {
    let (g, asm) = built(&FOREIGN);
    assert_eq!(
        guard_counterfactual(GUARD, "foreign", &g, Some(&asm)),
        Some(Ok(()))
    );
    // Not a guard row: no counterfactual applies.
    assert_eq!(guard_counterfactual("l22", "foreign", &g, Some(&asm)), None);
    let fails = |mutate: &dyn Fn(&mut Assembly, &CodePropertyGraph)| {
        let (g, mut asm) = built(&FOREIGN);
        mutate(&mut asm, &g);
        guard_counterfactual(GUARD, "foreign", &g, Some(&asm))
            .expect("a guard row")
            .expect_err("counterfactual must fail")
    };
    // The pending literal hop is the one edge the counterfactual removes.
    let no_hop = fails(&|asm, g| {
        let input = asm.guard_input.as_mut().unwrap();
        let from = variable(g, "App", "x", VarAccess::Use);
        let to = variable(g, "D", "a", VarAccess::Def);
        while let Some(edge) = input.graph.find_edge(from, to) {
            input.graph.remove_edge(edge);
        }
    });
    assert!(no_hop.contains("pending literal hop"), "{no_hop}");
    // Without the seeds the formal cannot reach one, which a foreign case must.
    let no_seed = fails(&|asm, _| asm.guard_input.as_mut().unwrap().seeds.clear());
    assert!(no_seed.contains("single-hop counterfactual"), "{no_seed}");
    // The pending props counts must be the final counts.
    let counts = fails(&|asm, _| {
        asm.guard_input
            .as_mut()
            .unwrap()
            .counters
            .insert("edges_to_def".into(), 2);
    });
    assert!(
        counts.contains("retain the pending props counts"),
        "{counts}"
    );
}

#[test]
fn guard_counterfactual_terminates_on_a_cyclic_before_graph() {
    // The seed search is a graph walk; a cycle in the before graph must not change its answer.
    let (g, mut asm) = built(&FOREIGN);
    let input = asm.guard_input.as_mut().unwrap();
    let def = variable(&g, "App", "x", VarAccess::Def);
    let used = variable(&g, "App", "x", VarAccess::Use);
    input
        .graph
        .add_edge(used, def, CpgEdge::DataFlow(FlowConfidence::Exact));
    assert_eq!(
        guard_counterfactual(GUARD, "foreign", &g, Some(&asm)),
        Some(Ok(()))
    );
}
