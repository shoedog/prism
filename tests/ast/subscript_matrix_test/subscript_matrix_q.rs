use super::{at, taint_reaches, Bytes, Fixture, Graph, Label, Row, SeedSpec, Truth};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use prism::access_path::AccessPath;
use prism::cpg::{CodePropertyGraph, CpgEdge, CpgNode, ReturnFlowMode, VarAccess};
use prism::navigation::types::{GraphPayload, SymbolRef};
use prism::reasoning::order::AstOrderView;
use prism::reasoning::shape::{node_to_graph_node, reachability_for_node_from_ordered};
use prism::reasoning::types::Reachability;
use serde::Deserialize;
use std::collections::BTreeSet;

fn one() -> usize {
    1
}

#[derive(Debug, Deserialize)]
pub(super) struct Site {
    line: usize,
    bytes: Bytes,
    owner: String,
    owner_line: usize,
    path: Option<AccessPath>,
    #[serde(default = "one")]
    node_count: usize,
}

#[derive(Deserialize)]
struct Pair {
    write: Site,
    read: Site,
    label: Option<Label>,
}

#[derive(Deserialize)]
pub(super) struct Marker {
    marker: String,
    source: Site,
    present: bool,
    verdict: String,
    selected_pair: bool,
    pairs: Vec<Pair>,
}

pub(super) fn check_truth(row: &Row, truth: &Truth) -> Result<(), String> {
    let bits: &[bool] = match truth {
        Truth::Reaches => &[true],
        Truth::Not => &[false],
        Truth::Mixed(bits) => bits,
        _ => return Err(format!("invalid Q truth {truth:?}")),
    };
    let markers: Vec<_> = row.markers.iter().map(|m| m.marker.as_str()).collect();
    let expected: &[&str] = match row.truth_protocol.as_deref() {
        None if !matches!(truth, Truth::Mixed(_)) => &["A"],
        Some("ab") if matches!(truth, Truth::Mixed(_)) => &["A", "B"],
        _ => return Err(format!("invalid Q truth protocol {:?}", row.truth_protocol)),
    };
    if markers.as_slice() != expected || bits.len() != markers.len() {
        return Err(format!("invalid marker order or count {markers:?}"));
    }
    if row
        .markers
        .iter()
        .map(|m| m.present)
        .ne(bits.iter().copied())
    {
        return Err(format!("unexpected executed marker bits {bits:?}"));
    }
    for marker in &row.markers {
        if marker
            .pairs
            .iter()
            .filter_map(|pair| pair.label)
            .any(|label| label != Label::NameonlyCfgIncomplete)
        {
            return Err(format!("{} invalid marked addition label", marker.marker));
        }
        if marker.selected_pair
            && (!marker.present && row.accepted_loss.is_none()
                || marker.verdict != "Reached"
                || marker.pairs.iter().all(|pair| pair.label.is_none()))
        {
            return Err(format!(
                "{} invalid selected-pair expectation",
                marker.marker
            ));
        }
    }
    if row.markers.len() == 2 && row.markers[0].source.line == row.markers[1].source.line {
        return Err("A and B share a source line".into());
    }
    Ok(())
}

fn bind(
    graph: &Graph,
    file: &str,
    site: &Site,
    access: VarAccess,
) -> Result<Vec<NodeIndex>, String> {
    let nodes: Vec<_> = graph
        .node_indices()
        .filter(|&node| {
            at(&graph[node], file, site.bytes, access)
                && matches!(&graph[node], CpgNode::Variable { line, .. } if *line == site.line)
        })
        .collect();
    if nodes.len() != site.node_count {
        return Err(format!(
            "{access:?} at {}:{:?}: expected {} nodes, found {}",
            site.line,
            site.bytes,
            site.node_count,
            nodes.len()
        ));
    }
    if !nodes.is_empty()
        && !nodes.iter().any(|&node| {
            matches!(&graph[node], CpgNode::Variable { function, function_start_line, path, .. }
            if function == &site.owner && *function_start_line == site.owner_line
                && site.path.as_ref().is_none_or(|expected| expected == path))
        })
    {
        return Err(format!("{access:?} missing typed node for {site:?}"));
    }
    Ok(nodes)
}

fn check_pair(
    graph: &Graph,
    file: &str,
    pair: &Pair,
) -> Result<Option<(NodeIndex, NodeIndex)>, String> {
    let writes = bind(graph, file, &pair.write, VarAccess::Def)?;
    let reads = bind(graph, file, &pair.read, VarAccess::Use)?;
    let labels: Vec<_> = writes
        .iter()
        .flat_map(|&write| {
            reads
                .iter()
                .flat_map(move |&read| graph.edges_connecting(write, read))
        })
        .filter_map(|edge| match edge.weight() {
            CpgEdge::DataFlow(label) => Some(*label),
            _ => None,
        })
        .collect();
    match (
        pair.label,
        labels.as_slice(),
        writes.as_slice(),
        reads.as_slice(),
    ) {
        (None, [], _, _) => Ok(None),
        (Some(expected), [label], [write], [read]) if expected.matches(*label) => {
            Ok(Some((*write, *read)))
        }
        _ => Err(format!(
            "W {:?} to R {:?}: expected {:?}, got DataFlow labels {labels:?}",
            pair.write, pair.read, pair.label
        )),
    }
}

fn at_symbol(symbol: &SymbolRef, file: &str, site: &Site, expected_access: &str) -> bool {
    matches!(
        symbol,
        SymbolRef::Variable { file: name, function, line, start_byte, end_byte, access, .. }
        if name == file && *line == site.line && [*start_byte, *end_byte] == site.bytes
            && access == expected_access && function == &site.owner
    )
}

struct QueryResult {
    verdict: Reachability,
    witness: Option<(GraphPayload, usize)>,
}

fn query(fixture: &Fixture, file: &str, source: &Site, sink: &Site) -> Result<QueryResult, String> {
    let seed = |line| SeedSpec::Loc {
        file: file.into(),
        line,
    };
    let evidence = taint_reaches(
        &fixture.session,
        &[seed(source.line)],
        Some(&[seed(sink.line)]),
    )
    .map_err(|error| format!("unbound production query: {error:?}"))?;
    let reasoning = evidence
        .reasoning
        .ok_or("production query returned no reasoning")?;
    if reasoning.source_count != 1 || reasoning.sinks_omitted != 0 {
        return Err("production query did not isolate the marked source and sink".into());
    }
    let mut results = reasoning
        .per_sink
        .iter()
        .filter(|result| at_symbol(&result.sink, file, sink, "use"))
        .flat_map(|result| &result.sources)
        .filter(|result| at_symbol(&result.source, file, source, "def"));
    let result = results
        .next()
        .ok_or("production query omitted the marked source/sink")?;
    if results.next().is_some() {
        return Err("production query duplicated the marked source/sink".into());
    }
    let witness = match result.graph_node {
        Some(sink_node) => {
            let graph = evidence
                .graph
                .ok_or("production query omitted the witness graph")?;
            let symbol = graph
                .nodes
                .get(sink_node)
                .and_then(|node| node.symbol.as_ref());
            if !symbol.is_some_and(|symbol| at_symbol(symbol, file, sink, "use")) {
                return Err("production query returned the wrong witness terminal".into());
            }
            Some((graph, sink_node))
        }
        None => None,
    };
    Ok(QueryResult {
        verdict: result.reachability,
        witness,
    })
}

fn selected_hop(
    cpg: &CodePropertyGraph,
    source: NodeIndex,
    graph: &GraphPayload,
    mut node: usize,
    pairs: &BTreeSet<(NodeIndex, NodeIndex)>,
) -> Option<(NodeIndex, NodeIndex)> {
    let root = node_to_graph_node(cpg, source).symbol;
    let mut visited = BTreeSet::new();
    let mut selected = None;
    while visited.insert(node) {
        let target = graph.nodes.get(node)?;
        if target.symbol == root {
            return selected;
        }
        let mut incoming = graph.edges.iter().filter(|edge| edge.to == node);
        let edge = incoming.next()?;
        if incoming.next().is_some() {
            // A union of witnesses cannot certify a selected chain at this node.
            return None;
        }
        let from = graph.nodes.get(edge.from)?;
        if selected.is_none() && edge.kind == "DataFlow" {
            selected = pairs.iter().copied().find(|&(write, read)| {
                node_to_graph_node(cpg, write).symbol == from.symbol
                    && node_to_graph_node(cpg, read).symbol == target.symbol
            });
        }
        node = edge.from;
    }
    None
}

pub(super) fn check(fixture: &Fixture, file: &str, row: &Row) -> Result<(), String> {
    let cpg = fixture.session.index.cpg();
    let order = AstOrderView::new(cpg, &fixture.session.repo.files);
    let sink = row.sink.as_ref().ok_or("missing Q sink")?;
    let sinks = bind(&cpg.graph, file, sink, VarAccess::Use)?;
    let mut failures = Vec::new();
    let mut selected = Vec::new();
    for marker in &row.markers {
        let roots = bind(&cpg.graph, file, &marker.source, VarAccess::Def)?;
        let [source] = roots.as_slice() else {
            return Err(format!("{} source Def is not unique", marker.marker));
        };
        let source = *source;
        let mut pairs = BTreeSet::new();
        for pair in &marker.pairs {
            match check_pair(&cpg.graph, file, pair) {
                Ok(Some(pair)) => {
                    pairs.insert(pair);
                }
                Ok(None) => {}
                Err(error) => failures.push(format!("{} {error}", marker.marker)),
            }
        }
        let witness = match query(fixture, file, &marker.source, sink) {
            Ok(QueryResult {
                verdict: actual,
                witness,
            }) => {
                if format!("{actual:?}") != marker.verdict {
                    failures.push(format!(
                        "{} expected {}, got {actual:?}",
                        marker.marker, marker.verdict
                    ));
                }
                if witness.is_some() != (actual == Reachability::Reached) {
                    failures.push(format!(
                        "{} unexpected public witness for {actual:?}",
                        marker.marker
                    ));
                }
                witness
            }
            Err(error) => {
                failures.push(format!("{} {error}", marker.marker));
                continue;
            }
        };
        if !marker.selected_pair {
            continue;
        }
        let hop = witness
            .as_ref()
            .and_then(|(graph, terminal)| selected_hop(cpg, source, graph, *terminal, &pairs));
        if let Some(hop) = hop {
            selected.push((source, hop));
        } else {
            failures.push(format!(
                "{} selected witness misses the marked W→R DataFlow",
                marker.marker
            ));
        }
    }
    if row.id == "Q02.both.py" {
        // The masking oracle must run for both roots; a missing selection is a failure, not a skip.
        let mut ran = 0;
        if let [(a, hop), (b, _)] = selected.as_slice() {
            let excluded = BTreeSet::from([*hop]);
            for (root, should_reach) in [(*a, false), (*b, true)] {
                let trace = cpg.taint_trace_nodes_excluding_with_mode(
                    &[root],
                    Some(&order),
                    &excluded,
                    ReturnFlowMode::On,
                );
                let result = reachability_for_node_from_ordered(
                    cpg,
                    &trace,
                    root,
                    sinks[0],
                    Some(&order),
                    &mut Vec::new(),
                );
                ran += 1;
                if (result == Reachability::Reached) != should_reach {
                    failures.push(format!("masking A's W→R: root {root:?} got {result:?}, expected reaches={should_reach}"));
                }
            }
        }
        if ran != 2 {
            let found = selected.len();
            failures.push(format!(
                "Q02.both masking oracle did not run: {ran} of 2 roots checked, {found} selected pairs"
            ));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join(" | "))
    }
}

#[test]
fn q04_owner_redirect_changes_typed_edge_bag() {
    use prism::cpg::FlowConfidence;

    let node = |owner: &str, owner_line, access, line, start| CpgNode::Variable {
        file: "case.py".into(),
        function: owner.into(),
        function_start_line: owner_line,
        path: AccessPath::simple("d"),
        line,
        start_byte: start,
        end_byte: start + 1,
        access,
    };
    let mut parent = Graph::new();
    let source = parent.add_node(node("outer", 1, VarAccess::Def, 3, 20));
    let first = parent.add_node(node("outer", 1, VarAccess::Use, 8, 70));
    let second = parent.add_node(node("inner", 6, VarAccess::Use, 8, 70));
    let exact = CpgEdge::DataFlow(FlowConfidence::Exact);
    let original = parent.add_edge(source, first, exact.clone());

    let edge_bag = |graph: &Graph, typed: bool| {
        let endpoint = |index: NodeIndex| {
            let mut key = serde_json::to_value(&graph[index]).unwrap();
            if !typed {
                let variable = key["Variable"].as_object_mut().unwrap();
                variable.remove("function");
                variable.remove("function_start_line");
            }
            serde_json::to_string(&key).unwrap()
        };
        let mut records: Vec<_> = graph
            .edge_references()
            .map(|edge| {
                (
                    endpoint(edge.source()),
                    endpoint(edge.target()),
                    serde_json::to_string(edge.weight()).unwrap(),
                )
            })
            .collect();
        records.sort_unstable();
        records
    };

    let mut redirected = parent.clone();
    assert_eq!(edge_bag(&parent, true), edge_bag(&redirected, true));
    redirected.remove_edge(original).unwrap();
    redirected.add_edge(source, second, exact);
    assert_eq!(parent.node_count(), redirected.node_count());
    assert_eq!(edge_bag(&parent, false), edge_bag(&redirected, false));
    assert_ne!(edge_bag(&parent, true), edge_bag(&redirected, true));
}

#[test]
fn tsx_value_case_uses_tsx_graph() {
    use prism::languages::Language;

    let file = "case.tsx";
    let source = "function caseFn() {\n  const t: string = src();\n  sink(t);\n}\n";
    let fixture = super::fixture(file, source);
    let parsed = &fixture.session.repo.files[file];
    assert_eq!(parsed.language, Language::Tsx);
    assert_eq!(parsed.parse_error_count, 0);
    let start = source.find("sink(t)").unwrap() + "sink(".len();
    super::uses_at(&fixture.session.index.cpg().graph, file, [start, start + 1]).unwrap();
}
