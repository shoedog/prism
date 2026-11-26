//! Independent TypeScript answers for direct-literal collisions and replacement.

use super::props_generated_tests::{
    endpoint, exact_node, owner, parse, span, Access, Endpoint, Label, Owner, Site, Span, CASES,
};
use super::props_s2_test_support::{capture, PreBinding};
use super::{CodePropertyGraph, CpgEdge, FlowConfidence, VarAccess};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use petgraph::{graph::NodeIndex, visit::EdgeRef, Direction};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    from: Endpoint,
    to: Endpoint,
    label: Label,
}

#[derive(Debug, Deserialize)]
struct Leaf {
    path: Vec<String>,
    from: Endpoint,
}

#[derive(Debug, Deserialize)]
pub(super) struct LiteralOutcome {
    edges: Vec<Edge>,
    counters: BTreeMap<String, usize>,
    survivors: Vec<Leaf>,
}

#[derive(Debug, Deserialize)]
struct Evidence {
    caller: Owner,
    target: Owner,
    formal: Span,
    site: Site,
    defs: Vec<Endpoint>,
    source_defs: Vec<Endpoint>,
}

#[derive(Debug, Deserialize)]
struct Cell {
    id: String,
    family: String,
    grammar: String,
    files: BTreeMap<String, String>,
    required_evidence: Evidence,
    expected_outcome: LiteralOutcome,
}

fn candidates(pre: &PreBinding, at: &Endpoint) -> Vec<NodeIndex> {
    let key = (
        at.file.clone(),
        at.owner.clone(),
        at.owner_line,
        at.line,
        AccessPath::simple(&at.name),
        match at.access {
            Access::Def => VarAccess::Def,
            Access::Use => VarAccess::Use,
        },
    );
    pre.exact
        .get(&key)
        .and_then(|by_span| by_span.get(&(at.bytes[0], at.bytes[1])))
        .cloned()
        .unwrap_or_default()
}

pub(super) fn observe_source(pre: &PreBinding, at: &Endpoint, input: &Endpoint) {
    let bases = candidates(pre, at);
    assert_eq!(bases.len(), 1, "masked base candidate at {at:?}");
    let inputs = candidates(pre, input);
    assert_eq!(inputs.len(), 1, "masked source Def at {input:?}");
    let incoming: Vec<_> = pre
        .graph
        .edges_directed(bases[0], Direction::Incoming)
        .filter(|edge| edge.weight().is_data_flow())
        .map(|edge| (edge.source(), edge.weight().clone()))
        .collect();
    assert_eq!(
        incoming,
        [(inputs[0], CpgEdge::DataFlow(FlowConfidence::Exact))]
    );
    let competing: Vec<_> = pre
        .graph
        .node_indices()
        .filter_map(|idx| {
            let other = endpoint(&pre.graph[idx])?;
            (other.file == at.file
                && other.owner == at.owner
                && other.owner_line == at.owner_line
                && other.access == Access::Use
                && at.bytes[0] <= other.bytes[0]
                && other.bytes[1] <= at.bytes[1]
                && other.bytes[0] < other.bytes[1])
                .then_some(other)
        })
        .collect();
    assert_eq!(
        competing,
        [at.clone()],
        "identifier leaf has no separate full tier"
    );
}

fn evidence(file: &ParsedFile, graph: &CodePropertyGraph, pre: &PreBinding, cell: &Cell) {
    let want = &cell.required_evidence;
    let target = owner(file, &want.target);
    let caller = owner(file, &want.caller);
    let formal = exact_node(file, want.formal);
    assert_eq!(formal.kind(), "object_pattern");
    let first = target
        .child_by_field_name("parameters")
        .unwrap()
        .named_child(0)
        .unwrap();
    let binding = first.child_by_field_name("pattern").unwrap_or(first);
    assert_eq!(binding, formal, "runtime position zero");
    let site = &want.site;
    let calls: Vec<_> = graph
        .call_graph
        .calls
        .iter()
        .filter(|(id, _)| {
            id.file == file.path && id.name == want.caller.name && id.start_line == want.caller.line
        })
        .flat_map(|(_, calls)| calls)
        .filter(|call| call.callee_name == want.target.name)
        .collect();
    assert_eq!(calls.len(), 1, "recorded literal call");
    assert_eq!([calls[0].start_byte, calls[0].end_byte], site.bytes);
    let resolved = graph.call_graph.resolve_call_site(calls[0]);
    assert_eq!(resolved.len(), 1, "real resolver singleton");
    let resolved = &resolved[0];
    assert_eq!(resolved.confidence, site.resolution);
    assert_eq!(
        (
            &resolved.target.file,
            &resolved.target.name,
            resolved.target.start_line,
            resolved.target.end_line
        ),
        (
            &file.path,
            &want.target.name,
            want.target.line,
            want.target.end_line
        ),
    );
    let call = exact_node(file, site.bytes);
    assert_eq!(call.kind(), "call_expression");
    let argument = call
        .child_by_field_name("arguments")
        .unwrap()
        .named_child(0)
        .unwrap();
    assert_eq!(span(argument), site.argument);
    assert_eq!(argument.kind(), "object");
    for def in &want.defs {
        assert_eq!(
            candidates(pre, def).len(),
            1,
            "declared binding Def {def:?}"
        );
        let binding = exact_node(file, def.bytes);
        assert!(want.formal[0] <= binding.start_byte() && binding.end_byte() <= want.formal[1]);
        assert_eq!(file.node_text(&binding), def.name);
    }
    for source in &want.source_defs {
        assert_eq!(
            candidates(pre, source).len(),
            1,
            "referenced source Def {source:?}"
        );
    }
    for leaf in &cell.expected_outcome.survivors {
        let operand = exact_node(file, leaf.from.bytes);
        assert!(!leaf.path.is_empty());
        assert!(
            argument.start_byte() < operand.start_byte()
                && operand.end_byte() < argument.end_byte()
        );
        assert_eq!(file.node_text(&operand), leaf.from.name);
        let mut cursor = operand;
        let mut observed_path = Vec::new();
        while cursor != argument {
            let pair = cursor.parent().unwrap();
            assert_eq!(pair.kind(), "pair", "generated identifier value");
            assert_eq!(pair.child_by_field_name("value"), Some(cursor));
            let token = file.node_text(&pair.child_by_field_name("key").unwrap());
            observed_path.push(token.trim_matches(['\'', '"']));
            cursor = pair.parent().unwrap();
            assert_eq!(cursor.kind(), "object");
        }
        observed_path.reverse();
        assert_eq!(observed_path, leaf.path);
        assert!(file.js_ts_occurrence_binds_formal(&caller, &leaf.from.name, operand.start_byte()));
        let input = want
            .source_defs
            .iter()
            .find(|def| def.name == leaf.from.name)
            .unwrap();
        observe_source(pre, &leaf.from, input);
    }
}

pub(super) fn compare_literal(
    graph: &CodePropertyGraph,
    caller: &Owner,
    target: &Owner,
    source_defs: &BTreeSet<Endpoint>,
    want: &LiteralOutcome,
) -> Result<(), String> {
    let nodes: BTreeMap<_, _> = graph
        .graph
        .node_indices()
        .filter_map(|index| Some((index, endpoint(graph.node(index))?)))
        .collect();
    let mut observed: Vec<_> = graph
        .graph
        .edge_references()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(label) = *edge.weight() else {
                return None;
            };
            let from = nodes.get(&edge.source())?;
            let to = nodes.get(&edge.target())?;
            (from.owner == caller.name
                && from.owner_line == caller.line
                && to.owner == target.name
                && to.owner_line == target.line)
                .then(|| Edge {
                    from: from.clone(),
                    to: to.clone(),
                    label: label.into(),
                })
        })
        .collect();
    observed.sort();
    let mut expected = want.edges.clone();
    expected.sort();

    let mut errors = Vec::new();
    if observed != expected {
        errors.push(format!(
            "edges expected {expected:?}, observed {observed:?}"
        ));
    }
    let index = |at: &Endpoint| {
        nodes
            .iter()
            .find(|(_, value)| *value == at)
            .map(|(&id, _)| id)
    };
    for source in source_defs {
        let Some(from) = index(source) else {
            errors.push(format!("missing marked source Def {source:?}"));
            continue;
        };
        for (to, dest) in &nodes {
            if dest.owner != target.name
                || dest.owner_line != target.line
                || dest.access != Access::Def
            {
                continue;
            }
            let permitted = expected
                .iter()
                .any(|edge| edge.from.name == source.name && edge.to == *dest);
            if graph.has_path(from, *to, &CpgEdge::is_data_flow) != permitted {
                errors.push(format!(
                    "path {} -> {dest:?}: expected {permitted}",
                    source.name
                ));
            }
        }
    }
    let counts: BTreeMap<_, _> = graph
        .dfg_label_stats
        .dfg_props
        .iter()
        .filter(|(_, count)| **count != 0)
        .map(|(key, count)| (key.clone(), *count))
        .collect();
    if counts != want.counters {
        errors.push(format!(
            "counters expected {:?}, observed {counts:?}",
            want.counters
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[derive(Debug, Default)]
struct Receipt {
    planned: usize,
    excluded: usize,
    emitted: usize,
    parsed: usize,
    preconditions: usize,
    compared: usize,
    green: usize,
    red: usize,
    expected_edges: BTreeMap<usize, usize>,
}

#[test]
fn props_s2_generated_collision_and_replacement_cells() {
    let mut receipts = BTreeMap::<String, Receipt>::new();
    let mut errors = 0;
    let mut previous = String::new();
    for line in CASES.lines() {
        let id = line
            .strip_prefix(r#"{"id":""#)
            .and_then(|rest| rest.split('"').next())
            .unwrap();
        assert!(id > previous.as_str(), "duplicate or unordered cell {id}");
        previous = id.to_owned();
        if !id.starts_with("literal-") {
            continue;
        }
        let cell: Cell = serde_json::from_str(line).expect("literal cell schema");
        assert_eq!(cell.id, id);
        let receipt = receipts.entry(cell.family.clone()).or_default();
        receipt.planned += 1;
        receipt.emitted += 1;
        let preconditions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let parsed = parse(&cell.files);
            let file = &parsed[&format!("case.{}", cell.grammar)];
            assert!(!file.tree.root_node().has_error() && file.parse_error_count == 0);
            receipt.parsed += 1;
            let (graph, observed) = capture(|| CodePropertyGraph::build(&parsed));
            evidence(file, &graph, &observed.pre_binding.unwrap(), &cell);
            graph
        }));
        let Ok(graph) = preconditions else {
            eprintln!("PROPS_GENERATED {} PRECONDITION {:?}", cell.id, cell.files);
            errors += 1;
            continue;
        };
        receipt.preconditions += 1;
        receipt.compared += 1;
        *receipt
            .expected_edges
            .entry(cell.expected_outcome.edges.len())
            .or_default() += 1;
        let want = &cell.required_evidence;
        let defs = want.source_defs.iter().cloned().collect();
        match compare_literal(
            &graph,
            &want.caller,
            &want.target,
            &defs,
            &cell.expected_outcome,
        ) {
            Ok(()) => receipt.green += 1,
            Err(detail) => {
                receipt.red += 1;
                errors += 1;
                eprintln!("PROPS_GENERATED {} RED {:?}\n{detail}", cell.id, cell.files);
            }
        }
    }
    for (name, planned) in [("literal-collision", 4000), ("literal-replacement", 48)] {
        let got = &receipts[name];
        eprintln!("{name}: {got:?}");
        assert_eq!(
            (got.planned, got.excluded, got.emitted),
            (planned, 0, planned)
        );
    }
    assert_eq!(
        errors, 0,
        "literal cells with failed preconditions or outcomes"
    );
}
