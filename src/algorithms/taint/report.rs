//! Finding evidence ranking and the taint-chain diagram.

use crate::finding_confidence::{EvidenceHop, EvidencePath};
use crate::output::mermaid::safe_node_id;
use crate::slice::{EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind, SliceGraph};
use std::collections::BTreeMap;

/// Build a Chain-shaped `SliceGraph` representing one source-to-sink taint path.
///
/// `intermediate` contains any intermediate step nodes (file, line, line-text)
/// ordered from source to sink. For Plan 1 the caller passes `&[]`; later plans
/// can populate it from `FlowPath` edges.
pub(super) fn build_taint_chain_diagram(
    source_file: &str,
    source_line: usize,
    source_text: &str,
    sink_file: &str,
    sink_line: usize,
    sink_text: &str,
    intermediate: &[(String, usize, String)],
) -> SliceGraph {
    // Degenerate case: source == sink with no intermediates. Emit a single
    // Sink node rather than a duplicate-id pair.
    if intermediate.is_empty() && source_file == sink_file && source_line == sink_line {
        let node = GraphNode {
            id: safe_node_id(sink_file, sink_line),
            label: format!("{}:{}\n{}", sink_file, sink_line, sink_text),
            kind: NodeKind::Sink,
            file: Some(sink_file.to_string()),
            line: Some(sink_line),
        };
        return SliceGraph {
            title: Some("Data flow".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![node],
            edges: vec![],
            clusters: vec![],
            mermaid: String::new(),
        };
    }
    // Normal path: build [source, ...intermediate, sink] chain.
    let mut nodes = vec![GraphNode {
        id: safe_node_id(source_file, source_line),
        label: format!("{}:{}\n{}", source_file, source_line, source_text),
        kind: NodeKind::Source,
        file: Some(source_file.to_string()),
        line: Some(source_line),
    }];
    for (f, l, t) in intermediate {
        nodes.push(GraphNode {
            id: safe_node_id(f, *l),
            label: format!("{}:{}\n{}", f, l, t),
            kind: NodeKind::Step,
            file: Some(f.clone()),
            line: Some(*l),
        });
    }
    nodes.push(GraphNode {
        id: safe_node_id(sink_file, sink_line),
        label: format!("{}:{}\n{}", sink_file, sink_line, sink_text),
        kind: NodeKind::Sink,
        file: Some(sink_file.to_string()),
        line: Some(sink_line),
    });
    let edges: Vec<GraphEdge> = nodes
        .windows(2)
        .map(|pair| GraphEdge {
            from: pair[0].id.clone(),
            to: pair[1].id.clone(),
            label: Some("tainted".to_string()),
            style: EdgeStyle::Solid,
        })
        .collect();
    SliceGraph {
        title: Some("Data flow".to_string()),
        shape: GraphShape::Chain,
        nodes,
        edges,
        clusters: vec![],
        mermaid: String::new(),
    }
}

/// Extract the trimmed text of a 1-indexed line from a source string.
/// Returns an empty string if the line number is out of range.
pub(super) fn source_line_text(source: &str, line: usize) -> String {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .to_string()
}

fn evidence_rank(evidence: &EvidencePath) -> u8 {
    if evidence.crossed_unlabeled || evidence.hops.is_empty() {
        0
    } else if evidence.hops.iter().all(EvidenceHop::is_exact) {
        2
    } else {
        1
    }
}

pub(super) fn keep_best_evidence<K: Ord>(
    map: &mut BTreeMap<K, EvidencePath>,
    key: K,
    evidence: EvidencePath,
) {
    match map.get(&key) {
        Some(current) if evidence_rank(current) >= evidence_rank(&evidence) => {}
        _ => {
            map.insert(key, evidence);
        }
    }
}
