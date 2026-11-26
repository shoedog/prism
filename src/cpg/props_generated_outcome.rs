//! Compare the selected pattern and all its Defs and incident data-flow edges.

use super::{endpoint, owner, Access, CodePropertyGraph, CpgEdge, Endpoint, Evidence, Label, Span};
use crate::ast::ParsedFile;
use petgraph::{graph::NodeIndex, visit::EdgeRef};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    from: Endpoint,
    to: Endpoint,
    label: Label,
}

#[derive(Debug, Deserialize)]
struct Path {
    from: Endpoint,
    to: Endpoint,
}

#[derive(Debug, Deserialize)]
pub(super) struct Outcome {
    pub(super) disposition: String,
    pattern: Pattern,
    defs: Vec<Endpoint>,
    edges: Vec<Edge>,
    paths: Vec<Path>,
    counts: Counts,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Pattern {
    position: usize,
    alias_skipped: bool,
    levels: Vec<Level>,
    elements: Vec<Element>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Level {
    path: Option<Vec<String>>,
    bytes: Span,
    consumed: BTreeSet<String>,
    unknown_sibling: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Element {
    name: String,
    bytes: Span,
    path: Option<Vec<String>>,
    rest_level: Option<usize>,
    defaulted: bool,
    admitted: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Counts {
    local_defs: usize,
    edges: usize,
    paths: usize,
    early_error: usize,
}

fn observe_pattern(file: &ParsedFile, target: Node<'_>, position: usize) -> Option<Pattern> {
    file.js_ts_pattern_params(&target)
        .into_iter()
        .find(|p| p.position == position)
        .map(|p| Pattern {
            position: p.position,
            alias_skipped: p.alias_skipped,
            levels: p
                .levels
                .into_iter()
                .map(|level| Level {
                    path: level.path,
                    bytes: [level.bytes.start, level.bytes.end],
                    consumed: level.consumed,
                    unknown_sibling: level.unknown_sibling,
                })
                .collect(),
            elements: p
                .elements
                .into_iter()
                .map(|element| Element {
                    name: element.name,
                    bytes: [element.bytes.start, element.bytes.end],
                    path: element.path,
                    rest_level: element.rest_level,
                    defaulted: element.defaulted,
                    admitted: element.admitted,
                })
                .collect(),
        })
}

pub(super) fn compare(
    file: &ParsedFile,
    graph: &CodePropertyGraph,
    evidence: &Evidence,
    want: &Outcome,
) {
    let target = owner(file, &evidence.target);
    let observed_pattern = observe_pattern(file, target, evidence.formal.position);
    let nodes: BTreeMap<NodeIndex, Endpoint> = graph
        .graph
        .node_indices()
        .filter_map(|index| Some((index, endpoint(graph.node(index))?)))
        .collect();
    let [start, end] = evidence.formal.bytes;
    let local_defs: BTreeSet<_> = nodes
        .iter()
        .filter_map(|(&index, at)| {
            (at.owner == evidence.target.name
                && at.access == Access::Def
                && ((start <= at.bytes[0] && at.bytes[1] <= end)
                    || evidence.reads.iter().any(|read| read.name == at.name)))
            .then_some(index)
        })
        .collect();
    let mut defs: Vec<_> = local_defs.iter().map(|index| &nodes[index]).collect();
    let mut wanted_defs: Vec<_> = want.defs.iter().collect();
    defs.sort();
    wanted_defs.sort();

    let mut edges: Vec<_> = graph
        .graph
        .edge_references()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(label) = *edge.weight() else {
                return None;
            };
            let from = nodes.get(&edge.source());
            let to = nodes.get(&edge.target());
            let into_read = to.is_some_and(|at| {
                evidence.reads.iter().any(|read| {
                    at.owner == read.owner
                        && at.name == read.name
                        && at.line == read.line
                        && at.access == read.access
                })
            });
            (local_defs.contains(&edge.source())
                || local_defs.contains(&edge.target())
                || into_read)
                .then(|| Edge {
                    from: from.expect("pattern flow source variable").clone(),
                    to: to.expect("pattern flow target variable").clone(),
                    label: label.into(),
                })
        })
        .collect();
    edges.sort();
    let mut wanted_edges: Vec<_> = want.edges.iter().collect();
    wanted_edges.sort();
    let paths = want
        .paths
        .iter()
        .filter(|path| {
            let from = nodes.iter().find(|(_, at)| **at == path.from);
            let to = nodes.iter().find(|(_, at)| **at == path.to);
            from.zip(to).is_some_and(|((&from, _), (&to, _))| {
                graph.has_path(from, to, &CpgEdge::is_data_flow)
            })
        })
        .count();
    let counts = Counts {
        local_defs: defs.len(),
        edges: edges.len(),
        paths,
        early_error: usize::from(file.dfg_owner_name(&target).is_none()),
    };
    assert!(
        observed_pattern.as_ref() == Some(&want.pattern) && defs == wanted_defs
            && edges.iter().collect::<Vec<_>>() == wanted_edges && counts == want.counts,
        "pattern expected {:?}\nobserved {:?}\nDefs expected {wanted_defs:?}\nobserved {defs:?}\nedges expected {wanted_edges:?}\nobserved {edges:?}\ncounts expected {:?}\nobserved {counts:?}",
        want.pattern, observed_pattern, want.counts,
    );
}
