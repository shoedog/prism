//! The S3 matrix evidence helpers and the live-lookup replay must fail on evidence that does not
//! fit (lane 3 S3, codecov patch). Each case perturbs one input of a control that is accepted.

use super::props_generated_tests::parse;
use super::props_s2_matrix_evidence::{check, guard_counterfactual};
use super::props_s2_test_support::{capture, Assembly, PreBinding};
use super::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, FlowDoubt, VarAccess};
use crate::ast::ParsedFile;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::BTreeMap;

fn built(files: &[(&str, &str)]) -> (CodePropertyGraph, Assembly) {
    let inputs = parse(
        &files
            .iter()
            .map(|(name, source)| ((*name).to_owned(), (*source).to_owned()))
            .collect(),
    );
    capture(&inputs, || CodePropertyGraph::build(&inputs))
}

fn variable(
    graph: &petgraph::graph::DiGraph<CpgNode, CpgEdge>,
    owner: &str,
    name: &str,
    access: VarAccess,
    line: Option<usize>,
) -> NodeIndex {
    let found: Vec<_> = graph
        .node_indices()
        .filter(|&i| {
            matches!(&graph[i], CpgNode::Variable { function, path, access: a, line: l, .. }
                if function == owner && path.to_string() == name && *a == access
                    && line.is_none_or(|want| want == *l))
        })
        .collect();
    assert_eq!(found.len(), 1, "{owner}:{name} {access:?} {line:?}");
    found[0]
}

const FOREIGN: [(&str, &str); 2] = [
    (
        "m.jsx",
        "export const App = x => {\n  return <D a={x} />;\n};\nfunction D({a}) {\n  Ghost(a);\n}\n",
    ),
    (
        "other.jsx",
        "export function Ghost(g) {\n  return observe(g);\n}\n",
    ),
];
const GUARD: &str = "d2-guard-bare-arrow-jsx";

fn guard_error(mutate: &dyn Fn(&mut Assembly, NodeIndex, NodeIndex)) -> String {
    let (g, mut asm) = built(&FOREIGN);
    let source = variable(&g.graph, "App", "x", VarAccess::Use, None);
    let dest = variable(&g.graph, "D", "a", VarAccess::Def, None);
    mutate(&mut asm, source, dest);
    guard_counterfactual(GUARD, "foreign", &g, Some(&asm))
        .expect("a guard row")
        .expect_err("counterfactual must fail")
}

#[test]
fn guard_counterfactual_accepts_the_jsx_guard_row_and_rejects_a_misflagged_pending_hop() {
    let (g, asm) = built(&FOREIGN);
    assert_eq!(
        guard_counterfactual(GUARD, "foreign", &g, Some(&asm)),
        Some(Ok(()))
    );
    // The recorded hop carries the foreign-seed flag the final removal depends on.
    let seeded = guard_error(&|asm, from, to| {
        for ((f, t, _), seed, _) in &mut asm.pending {
            if (*f, *t) == (from, to) {
                *seed = true;
            }
        }
    });
    assert!(
        seeded.contains("unexpected pending props hop flags"),
        "{seeded}"
    );
    // A bare-literal flag is the literal lane's: a JSX hop never carries it.
    let bare = guard_error(&|asm, from, to| {
        for ((f, t, _), _, bare) in &mut asm.pending {
            if (*f, *t) == (from, to) {
                *bare = true;
            }
        }
    });
    assert!(
        bare.contains("unexpected pending props hop flags"),
        "{bare}"
    );
    // Two pending records for one hop are not one hop.
    let twice = guard_error(&|asm, from, to| {
        let hop = asm
            .pending
            .iter()
            .find(|((f, t, _), _, _)| (*f, *t) == (from, to))
            .cloned()
            .unwrap();
        asm.pending.push(hop);
    });
    assert!(
        twice.contains("unexpected pending props hop flags"),
        "{twice}"
    );
}

#[test]
fn guard_counterfactual_rejects_a_pending_hop_the_before_graph_does_not_hold() {
    // The pending record and the pre-guard edge must be the same edge.
    let relabeled = guard_error(&|asm, from, to| {
        for ((f, t, edge), _, _) in &mut asm.pending {
            if (*f, *t) == (from, to) {
                *edge = CpgEdge::DataFlow(FlowConfidence::NameOnly(FlowDoubt::SameLine));
            }
        }
    });
    assert_eq!(relabeled, "pending hop and pre-guard graph disagree");
}

const FORWARD: &str = "function Page(x) {\n  return <Header a={x} />;\n}\nconst Header = ({a}) => {\n  return observe(a);\n};\nPage(1);\n";

#[test]
fn forward_jsx_evidence_needs_the_page_formal_to_reach_its_source() {
    let (g, asm) = built(&[("m.jsx", FORWARD)]);
    let accepted = check("d2-forward-jsx", "forward", "jsx", &g, Some(&asm));
    assert!(accepted.expect("a registered row").is_ok());
    // Without the formal's reaching edge the JSX source is unreached and the row is unmet.
    let (g, mut asm) = built(&[("m.jsx", FORWARD)]);
    let pre = &mut asm.pre_binding.as_mut().unwrap().graph;
    let formal = variable(pre, "Page", "x", VarAccess::Def, None);
    let edges: Vec<_> = pre
        .edges(formal)
        .filter(|e| e.weight().is_data_flow())
        .map(|e| e.id())
        .collect();
    assert!(!edges.is_empty());
    for edge in edges {
        pre.remove_edge(edge);
    }
    let rejected = check("d2-forward-jsx", "forward", "jsx", &g, Some(&asm))
        .expect("a registered row")
        .expect_err("the source is unreached");
    assert_eq!(rejected, "Page formal does not reach the JSX source");
}

// The operand starts on line 6 and its chain ends on line 7, where `see(box)` also reads `box`.
const MULTILINE: &str = "function D({a}) {\n  return use(a);\n}\nfunction App(y) {\n  const box = make();\n  const out = <D a={box\n    .value} />; TAIL\n  return out;\n}\n";

struct Replay {
    graph: CodePropertyGraph,
    pre: PreBinding,
    file: ParsedFile,
    opening: std::ops::Range<usize>,
}

fn replay_of(source: &str) -> Replay {
    let inputs = parse(&BTreeMap::from([("m.jsx".to_owned(), source.to_owned())]));
    let (graph, asm) = capture(&inputs, || CodePropertyGraph::build(&inputs));
    let start = source.find("<D").or_else(|| source.find("<Q")).unwrap();
    let end = start + source[start..].find("/>").unwrap() + 2;
    Replay {
        graph,
        pre: asm.pre_binding.unwrap(),
        file: inputs.into_values().next().unwrap(),
        opening: start..end,
    }
}

impl Replay {
    fn edges(
        &self,
        opening: std::ops::Range<usize>,
        hidden: &[NodeIndex],
    ) -> Vec<(NodeIndex, NodeIndex)> {
        let mut lookup = self.pre.live_lookup.clone();
        lookup.hide(hidden.iter().copied());
        let dest = self
            .pre
            .graph
            .node_indices()
            .find(|&i| {
                matches!(&self.pre.graph[i], CpgNode::Variable { function, path, access, .. }
                    if function == "D" && path.to_string() == "a" && *access == VarAccess::Def)
            })
            .unwrap();
        super::jsx_props_replay::replay(
            &self.file,
            opening,
            &self.graph,
            &self.pre.graph,
            &lookup,
            dest,
            false,
        )
        .into_iter()
        .map(|(from, to, _)| (from, to))
        .collect()
    }
}

#[test]
fn the_base_use_on_a_later_operand_line_binds_when_the_root_line_has_none() {
    let clean = replay_of(&MULTILINE.replace("TAIL", "see(box);"));
    let root = variable(&clean.pre.graph, "App", "box", VarAccess::Use, Some(6));
    let later = variable(&clean.pre.graph, "App", "box", VarAccess::Use, Some(7));
    let dest = variable(&clean.pre.graph, "D", "a", VarAccess::Def, None);
    // Control: the live lookup finds the root-line Use first.
    assert_eq!(clean.edges(clean.opening.clone(), &[]), vec![(root, dest)]);
    // With the root-line Use masked the fallback reads the `see(box)` Use on the chain's last line.
    assert_eq!(
        clean.edges(clean.opening.clone(), &[root]),
        vec![(later, dest)]
    );
    // The root-line anchor and the later read are both representatives.
    let written = replay_of(&MULTILINE.replace("TAIL", "box = other(); see(box);"));
    let root = variable(&written.pre.graph, "App", "box", VarAccess::Use, Some(6));
    assert!(
        matches!(written.pre.graph[root], CpgNode::Variable { start_byte, end_byte, .. }
        if start_byte == end_byte)
    );
    assert_eq!(written.edges(written.opening.clone(), &[]), vec![]);
    assert_eq!(written.edges(written.opening.clone(), &[root]), vec![]);
}

#[test]
fn replay_returns_nothing_for_a_site_that_is_not_a_recorded_opening_with_resolvable_props() {
    let good = replay_of(&MULTILINE.replace("TAIL", "see(box);"));
    assert_eq!(good.edges(good.opening.clone(), &[]).len(), 1);
    // A span that is no recorded JSX call site.
    let shifted = good.opening.start + 1..good.opening.end;
    assert_eq!(good.edges(shifted, &[]), vec![]);
    // A braced attribute that is not a spread has no entries to bind.
    let braced = replay_of(
        "function D({a}) {\n  return use(a);\n}\nfunction App(x) {\n  return <D {x} />;\n}\n",
    );
    assert_eq!(braced.edges(braced.opening.clone(), &[]), vec![]);
    // A callee that resolves to no definition is not a single target.
    let unresolved = replay_of(
        "function D({a}) {\n  return use(a);\n}\nfunction App(x) {\n  return <Q a={x} />;\n}\n",
    );
    assert_eq!(unresolved.edges(unresolved.opening.clone(), &[]), vec![]);
    // Control: the same braced element with a named attribute binds.
    let named = replay_of(
        "function D({a}) {\n  return use(a);\n}\nfunction App(x) {\n  return <D a={x} />;\n}\n",
    );
    assert_eq!(named.edges(named.opening.clone(), &[]).len(), 1);
}
