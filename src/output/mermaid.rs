//! Mermaid flowchart rendering for SliceGraph.
//! See docs/superpowers/specs/2026-05-09-data-flow-visualization-design.md.

use std::collections::{BTreeMap, BTreeSet};

use crate::slice::{
    DiagramWarning, DiagramWarningKind, EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind,
    SliceGraph,
};

/// Build a Mermaid-safe stable node id from a file path and line number.
/// Non-alphanumeric chars in the file path collapse to `_`.
pub(crate) fn safe_node_id(file: &str, line: usize) -> String {
    let slug: String = file
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("n_{}_{}", slug, line)
}

const CLASS_DEFS: &str = "\
    classDef origin fill:#cdf,stroke:#06c,stroke-width:2px;\n\
    classDef source fill:#fed68a;\n\
    classDef sink fill:#f88;\n\
    classDef caller fill:#dfe;\n\
    classDef callee fill:#fff3c4;";

fn class_for(kind: NodeKind) -> Option<&'static str> {
    match kind {
        NodeKind::Origin => Some("origin"),
        NodeKind::Source => Some("source"),
        NodeKind::Sink => Some("sink"),
        NodeKind::Caller => Some("caller"),
        NodeKind::Callee => Some("callee"),
        NodeKind::Step => None,
    }
}

fn arrow_for(style: EdgeStyle, label: &Option<String>) -> String {
    let arrow = match style {
        EdgeStyle::Solid => "-->",
        EdgeStyle::Bold => "==>",
        EdgeStyle::Dotted => "-.->",
    };
    match label {
        Some(l) if !l.is_empty() => {
            let (escaped, _trunc) = escape_label_inner(l);
            // Wrap in quotes inside |...| to handle | and other specials uniformly.
            format!("{}|\"{}\"|", arrow, escaped)
        }
        _ => arrow.to_string(),
    }
}

/// Inner escape — performs only character substitution and truncation.
/// Used by shape templates that wrap labels themselves with `"…"`.
/// Returns (escaped_label_inner, was_truncated).
pub(crate) fn escape_label_inner(s: &str) -> (String, bool) {
    const MAX: usize = 80;
    let mut out = s.replace('"', "&quot;").replace('\n', "<br/>");
    let truncated = out.chars().count() > MAX;
    if truncated {
        let take: String = out.chars().take(MAX - 1).collect();
        out = format!("{}…", take);
    }
    (out, truncated)
}

/// Render a `Chain`-shaped `SliceGraph` as a Mermaid `flowchart TD` string.
/// Nodes are always emitted as `id["label"]` (unconditional quoting) so that
/// labels containing `.`, `:`, spaces, etc. are safe without special-casing.
/// Styled classDef blocks for Source/Sink/etc. are appended at the end.
pub(crate) fn render_chain(g: &SliceGraph) -> String {
    let mut out = String::from("flowchart TD\n");
    for node in &g.nodes {
        let (label, _trunc) = escape_label_inner(&node.label);
        let class_suffix = class_for(node.kind)
            .map(|c| format!(":::{}", c))
            .unwrap_or_default();
        out.push_str(&format!("    {}[\"{}\"]{}\n", node.id, label, class_suffix));
    }
    for edge in &g.edges {
        let arrow = arrow_for(edge.style, &edge.label);
        out.push_str(&format!("    {} {} {}\n", edge.from, arrow, edge.to));
    }
    out.push_str(CLASS_DEFS);
    out
}

/// Render a `Cycle`-shaped `SliceGraph` as a Mermaid `flowchart LR` string.
/// Uses left-right orientation to naturally expose back-edges. Bold (`==>`)
/// edges visually distinguish the cycle back-edge from normal flow edges.
pub(crate) fn render_cycle(g: &SliceGraph) -> String {
    let mut out = String::from("flowchart LR\n");
    for node in &g.nodes {
        let (label, _trunc) = escape_label_inner(&node.label);
        let class_suffix = class_for(node.kind)
            .map(|c| format!(":::{}", c))
            .unwrap_or_default();
        out.push_str(&format!("    {}[\"{}\"]{}\n", node.id, label, class_suffix));
    }
    for edge in &g.edges {
        let arrow = arrow_for(edge.style, &edge.label);
        out.push_str(&format!("    {} {} {}\n", edge.from, arrow, edge.to));
    }
    out.push_str(CLASS_DEFS);
    out
}

/// Render a `Layered`-shaped `SliceGraph` as a Mermaid `flowchart TD` string.
/// Each `NodeCluster` becomes a `subgraph` block. Nodes not in any cluster
/// render at the top level after all subgraphs. Edges render last.
pub(crate) fn render_layered(g: &SliceGraph) -> String {
    let mut out = String::from("flowchart TD\n");
    let mut clustered: BTreeSet<&str> = BTreeSet::new();
    for (cluster_idx, cluster) in g.clusters.iter().enumerate() {
        let subgraph_id = format!("cluster_{}", cluster_idx);
        let (escaped_label, _trunc) = escape_label_inner(&cluster.label);
        out.push_str(&format!(
            "    subgraph {}[\"{}\"]\n",
            subgraph_id, escaped_label
        ));
        for nid in &cluster.node_ids {
            clustered.insert(nid.as_str());
            if let Some(node) = g.nodes.iter().find(|n| &n.id == nid) {
                let (label, _trunc) = escape_label_inner(&node.label);
                let class_suffix = class_for(node.kind)
                    .map(|c| format!(":::{}", c))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "        {}[\"{}\"]{}\n",
                    node.id, label, class_suffix
                ));
            }
        }
        out.push_str("    end\n");
    }
    // Orphan nodes (not in any cluster) emit at top level.
    for node in &g.nodes {
        if !clustered.contains(node.id.as_str()) {
            let (label, _trunc) = escape_label_inner(&node.label);
            let class_suffix = class_for(node.kind)
                .map(|c| format!(":::{}", c))
                .unwrap_or_default();
            out.push_str(&format!("    {}[\"{}\"]{}\n", node.id, label, class_suffix));
        }
    }
    for edge in &g.edges {
        let arrow = arrow_for(edge.style, &edge.label);
        out.push_str(&format!("    {} {} {}\n", edge.from, arrow, edge.to));
    }
    out.push_str(CLASS_DEFS);
    out
}

/// Render a `Fanout`-shaped `SliceGraph` as a Mermaid `flowchart LR` string.
/// `NodeKind::Origin` nodes use the stadium shape `id[("label")]` to visually
/// distinguish the change point that callers fan out from. All other node kinds
/// use the standard rectangular shape `id["label"]`.
pub(crate) fn render_fanout(g: &SliceGraph) -> String {
    let mut out = String::from("flowchart LR\n");
    for node in &g.nodes {
        let (label, _trunc) = escape_label_inner(&node.label);
        let class_suffix = class_for(node.kind)
            .map(|c| format!(":::{}", c))
            .unwrap_or_default();
        // Origin nodes use the rounded-rectangle "stadium" shape: id[("label")]
        let (open, close) = if node.kind == NodeKind::Origin {
            ("[(", ")]")
        } else {
            ("[", "]")
        };
        out.push_str(&format!(
            "    {}{}\"{}\"{}{}\n",
            node.id, open, label, close, class_suffix
        ));
    }
    for edge in &g.edges {
        let arrow = arrow_for(edge.style, &edge.label);
        out.push_str(&format!("    {} {} {}\n", edge.from, arrow, edge.to));
    }
    out.push_str(CLASS_DEFS);
    out
}

/// Apply size cap to a graph. Returns the truncated graph and any
/// `NodeCapExceeded` warnings if elision happened.
///
/// **Cycle shape special case:** For `GraphShape::Cycle`, the bold back-edge
/// endpoints are always preserved (they define the cycle's visual identity).
/// Pass 2 (head/tail elision) is skipped entirely for cycles — the size cap is
/// best-effort when structural integrity requires more nodes than `cap` allows.
/// A `NodeCapExceeded` warning is still emitted, but with a note that the
/// diagram size was preserved for structural integrity.
pub(crate) fn truncate_to_cap(g: &SliceGraph, cap: usize) -> (SliceGraph, Vec<DiagramWarning>) {
    // Enforce a minimum cap of 4: that is the smallest value that can
    // accommodate head + ghost + tail with at least one node on each side.
    // Below 4, head_n and tail_n + ghost always exceed the budget.
    const MIN_CAP: usize = 4;
    let cap = cap.max(MIN_CAP);
    if g.nodes.len() <= cap {
        return (g.clone(), vec![]);
    }
    let original_count = g.nodes.len();
    let mut working = g.clone();

    // Identify cycle back-edge endpoints when shape is Cycle.
    // These nodes are treated like Source/Sink/Origin — never removed — so
    // that the bold back-edge (which visually identifies the cycle) survives.
    let cycle_back_edge_endpoints: BTreeSet<String> = if g.shape == GraphShape::Cycle {
        g.edges
            .iter()
            .filter(|e| matches!(e.style, EdgeStyle::Bold))
            .flat_map(|e| [e.from.clone(), e.to.clone()])
            .collect()
    } else {
        BTreeSet::new()
    };

    // Pass 1: collapse linear chains. A node B with indegree=outdegree=1 and
    // not Source/Sink/Origin (or a cycle back-edge endpoint) can be removed;
    // its single in-edge and single out-edge fuse into one edge labeled with
    // hop count.
    loop {
        if working.nodes.len() <= cap {
            break;
        }
        let mut indeg: BTreeMap<&str, usize> = BTreeMap::new();
        let mut outdeg: BTreeMap<&str, usize> = BTreeMap::new();
        for e in &working.edges {
            *outdeg.entry(e.from.as_str()).or_default() += 1;
            *indeg.entry(e.to.as_str()).or_default() += 1;
        }
        // Find first removable node.
        let target = working.nodes.iter().find(|n| {
            let preserve = matches!(n.kind, NodeKind::Source | NodeKind::Sink | NodeKind::Origin)
                || cycle_back_edge_endpoints.contains(&n.id);
            !preserve
                && indeg.get(n.id.as_str()).copied().unwrap_or(0) == 1
                && outdeg.get(n.id.as_str()).copied().unwrap_or(0) == 1
        });
        let target_id = match target {
            Some(n) => n.id.clone(),
            None => break, // no further linear collapse possible
        };
        // Remove the node and fuse edges.
        let in_edge = working.edges.iter().find(|e| e.to == target_id).cloned();
        let out_edge = working.edges.iter().find(|e| e.from == target_id).cloned();
        if let (Some(i_e), Some(o_e)) = (in_edge, out_edge) {
            working
                .edges
                .retain(|e| e.from != target_id && e.to != target_id);
            let hops_in = parse_hops(&i_e.label).unwrap_or(1);
            let hops_out = parse_hops(&o_e.label).unwrap_or(1);
            let total = hops_in + hops_out;
            working.edges.push(GraphEdge {
                from: i_e.from,
                to: o_e.to,
                label: Some(format!("{} hops", total)),
                style: i_e.style,
            });
        }
        working.nodes.retain(|n| n.id != target_id);
    }

    // Pass 2: head + tail elision with a ghost node.
    // Skip for Cycle shape — the back-edge identity is more important than
    // the size cap. If Pass 1's preservation rules left us above cap, accept
    // the larger diagram.
    if working.nodes.len() > cap && g.shape != GraphShape::Cycle {
        let head_n = (cap + 1) / 2;
        let tail_n = cap.saturating_sub(head_n + 1).max(1);
        let head: Vec<GraphNode> = working.nodes.iter().take(head_n).cloned().collect();
        let tail: Vec<GraphNode> = working
            .nodes
            .iter()
            .rev()
            .take(tail_n)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let elided = working.nodes.len() - head.len() - tail.len();
        let ghost_id = "n_ellipsis".to_string();
        let ghost = GraphNode {
            id: ghost_id.clone(),
            label: format!("[{} more nodes elided]", elided),
            kind: NodeKind::Step,
            file: None,
            line: None,
        };
        let head_ids: BTreeSet<&str> = head.iter().map(|n| n.id.as_str()).collect();
        let tail_ids: BTreeSet<&str> = tail.iter().map(|n| n.id.as_str()).collect();
        let mut new_edges: Vec<GraphEdge> = vec![];
        // Per-endpoint dedup sets: track which specific endpoints have already had a
        // bridge edge added in each direction. Any one preserved endpoint gets at most
        // one bridge edge to/from ghost per direction.
        let mut head_endpoints_to_ghost: BTreeSet<String> = BTreeSet::new();
        let mut ghost_to_tail_endpoints: BTreeSet<String> = BTreeSet::new();
        let mut tail_endpoints_to_ghost: BTreeSet<String> = BTreeSet::new();
        let mut ghost_to_head_endpoints: BTreeSet<String> = BTreeSet::new();
        for e in &working.edges {
            let from_in_head = head_ids.contains(e.from.as_str());
            let to_in_head = head_ids.contains(e.to.as_str());
            let from_in_tail = tail_ids.contains(e.from.as_str());
            let to_in_tail = tail_ids.contains(e.to.as_str());

            let in_head = from_in_head && to_in_head;
            let in_tail = from_in_tail && to_in_tail;

            if in_head || in_tail {
                new_edges.push(e.clone());
                continue;
            }

            // Cross-region or interior-touching edge. For each endpoint that lives in
            // head or tail, ensure a bridge to/from the ghost exists exactly once per
            // endpoint+direction. Edges with neither endpoint preserved are dropped
            // (their nodes were elided).

            // Outgoing direction: from head/tail node to ghost (when the other end is
            // elided/in the opposite region).
            if from_in_head && head_endpoints_to_ghost.insert(e.from.clone()) {
                new_edges.push(GraphEdge {
                    from: e.from.clone(),
                    to: ghost_id.clone(),
                    label: None,
                    style: EdgeStyle::Dotted,
                });
            }
            if from_in_tail && tail_endpoints_to_ghost.insert(e.from.clone()) {
                new_edges.push(GraphEdge {
                    from: e.from.clone(),
                    to: ghost_id.clone(),
                    label: None,
                    style: EdgeStyle::Dotted,
                });
            }

            // Incoming direction: from ghost to head/tail node.
            if to_in_head && ghost_to_head_endpoints.insert(e.to.clone()) {
                new_edges.push(GraphEdge {
                    from: ghost_id.clone(),
                    to: e.to.clone(),
                    label: None,
                    style: EdgeStyle::Dotted,
                });
            }
            if to_in_tail && ghost_to_tail_endpoints.insert(e.to.clone()) {
                new_edges.push(GraphEdge {
                    from: ghost_id.clone(),
                    to: e.to.clone(),
                    label: None,
                    style: EdgeStyle::Dotted,
                });
            }
        }
        let mut all_nodes: Vec<GraphNode> = head;
        all_nodes.push(ghost);
        all_nodes.extend(tail);
        working.nodes = all_nodes;
        working.edges = new_edges;
    }

    let warns = if working.nodes.len() < original_count {
        vec![DiagramWarning {
            algorithm: String::new(),
            graph_title: g.title.clone(),
            kind: DiagramWarningKind::NodeCapExceeded,
            detail: format!(
                "elided to {} nodes from original {}",
                working.nodes.len(),
                original_count
            ),
        }]
    } else {
        // Cycle preservation kept us at original size despite being over cap.
        vec![DiagramWarning {
            algorithm: String::new(),
            graph_title: g.title.clone(),
            kind: DiagramWarningKind::NodeCapExceeded,
            detail: format!(
                "diagram size {} exceeds cap {} but structural integrity preserved",
                working.nodes.len(),
                cap
            ),
        }]
    };
    (working, warns)
}

/// Render a SliceGraph to Mermaid. Returns the rendered string and any warnings
/// (validation issues, cap exceedance). Caller is responsible for prefixing
/// warnings with the algorithm name.
pub fn render(g: &SliceGraph, cap: usize) -> (String, Vec<DiagramWarning>) {
    let mut warnings: Vec<DiagramWarning> = vec![];

    // Validation pass: empty graph
    if g.nodes.is_empty() {
        warnings.push(DiagramWarning {
            algorithm: String::new(),
            graph_title: g.title.clone(),
            kind: DiagramWarningKind::EmptyGraph,
            detail: "no nodes".to_string(),
        });
        return (String::new(), warnings);
    }

    // Validation pass: duplicate node ids
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut duplicates: BTreeSet<String> = BTreeSet::new();
    for n in &g.nodes {
        if !seen.insert(n.id.as_str()) {
            duplicates.insert(n.id.clone());
        }
    }
    for d in &duplicates {
        warnings.push(DiagramWarning {
            algorithm: String::new(),
            graph_title: g.title.clone(),
            kind: DiagramWarningKind::DuplicateNodeId,
            detail: format!("node id '{}' appears multiple times", d),
        });
    }

    // Validation pass: dangling edges
    let node_ids: BTreeSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    for e in &g.edges {
        if !node_ids.contains(e.from.as_str()) {
            warnings.push(DiagramWarning {
                algorithm: String::new(),
                graph_title: g.title.clone(),
                kind: DiagramWarningKind::DanglingEdge,
                detail: format!(
                    "edge from '{}' to '{}' — '{}' missing from nodes",
                    e.from, e.to, e.from
                ),
            });
        }
        if !node_ids.contains(e.to.as_str()) {
            warnings.push(DiagramWarning {
                algorithm: String::new(),
                graph_title: g.title.clone(),
                kind: DiagramWarningKind::DanglingEdge,
                detail: format!(
                    "edge from '{}' to '{}' — '{}' missing from nodes",
                    e.from, e.to, e.to
                ),
            });
        }
    }

    // Build a sanitized graph (drop dangling edges, dedup nodes by id keeping first).
    let mut sanitized_nodes: Vec<GraphNode> = vec![];
    let mut keep: BTreeSet<&str> = BTreeSet::new();
    for n in &g.nodes {
        if keep.insert(n.id.as_str()) {
            sanitized_nodes.push(n.clone());
        }
    }
    let sanitized_node_ids: BTreeSet<&str> =
        sanitized_nodes.iter().map(|n| n.id.as_str()).collect();
    let sanitized_edges: Vec<GraphEdge> = g
        .edges
        .iter()
        .filter(|e| {
            sanitized_node_ids.contains(e.from.as_str())
                && sanitized_node_ids.contains(e.to.as_str())
        })
        .cloned()
        .collect();
    let sanitized = SliceGraph {
        nodes: sanitized_nodes,
        edges: sanitized_edges,
        ..g.clone()
    };

    // Apply size cap.
    let (capped, cap_warns) = truncate_to_cap(&sanitized, cap);
    warnings.extend(cap_warns);

    // Detect label truncation on the capped graph — escape_label_inner returns the flag.
    for n in &capped.nodes {
        let (_label, trunc) = escape_label_inner(&n.label);
        if trunc {
            warnings.push(DiagramWarning {
                algorithm: String::new(),
                graph_title: g.title.clone(),
                kind: DiagramWarningKind::LabelTruncated,
                detail: format!("label on node '{}' exceeded 80 chars", n.id),
            });
        }
    }

    // Edge label truncation detection.
    for e in &capped.edges {
        if let Some(label) = &e.label {
            let (_escaped, trunc) = escape_label_inner(label);
            if trunc {
                warnings.push(DiagramWarning {
                    algorithm: String::new(),
                    graph_title: g.title.clone(),
                    kind: DiagramWarningKind::LabelTruncated,
                    detail: format!("edge label on '{}->{}' exceeded 80 chars", e.from, e.to),
                });
            }
        }
    }

    // Cluster label truncation detection.
    for c in &capped.clusters {
        let (_escaped, trunc) = escape_label_inner(&c.label);
        if trunc {
            warnings.push(DiagramWarning {
                algorithm: String::new(),
                graph_title: g.title.clone(),
                kind: DiagramWarningKind::LabelTruncated,
                detail: format!(
                    "cluster label '{}' exceeded 80 chars",
                    c.label.chars().take(20).collect::<String>()
                ),
            });
        }
    }

    let out = match capped.shape {
        GraphShape::Chain => render_chain(&capped),
        GraphShape::Layered => render_layered(&capped),
        GraphShape::Cycle => render_cycle(&capped),
        GraphShape::Fanout => render_fanout(&capped),
    };
    (out, warnings)
}

/// Render a complete `--format mermaid` markdown report from a multi-result.
pub fn format_mermaid_report(multi: &crate::slice::MultiSliceResult) -> String {
    let any_diagram = multi
        .results
        .iter()
        .any(|r| !r.diagrams.is_empty() || r.findings.iter().any(|f| !f.diagrams.is_empty()));
    let mut out = String::from("# Prism diagram report\n\n");
    if !any_diagram {
        out.push_str("_No flow-shaped findings produced for this run._\n");
    } else {
        for r in &multi.results {
            if r.diagrams.is_empty() && r.findings.iter().all(|f| f.diagrams.is_empty()) {
                continue;
            }
            out.push_str(&format!("## {}\n\n", r.algorithm.name()));
            for (idx, g) in r.diagrams.iter().enumerate() {
                let title = g
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("Diagram {}", idx + 1));
                out.push_str(&format!(
                    "### {}\n\n```mermaid\n{}\n```\n\n",
                    title, g.mermaid
                ));
            }
            let findings_with_diagrams: Vec<_> = r
                .findings
                .iter()
                .enumerate()
                .filter(|(_, f)| !f.diagrams.is_empty())
                .collect();
            for (idx, f) in findings_with_diagrams {
                let label = format!(
                    "Finding {}: {}{}",
                    idx + 1,
                    if f.description.is_empty() {
                        String::new()
                    } else {
                        format!("{} at ", f.description)
                    },
                    f.file
                );
                for g in &f.diagrams {
                    out.push_str(&format!(
                        "### {}\n\n```mermaid\n{}\n```\n\n",
                        label, g.mermaid
                    ));
                }
            }
        }
    }
    let warnings = multi.aggregate_diagram_warnings();
    if !warnings.is_empty() {
        out.push_str("## Diagnostics\n\n");
        for w in &warnings {
            let title = w.graph_title.as_deref().unwrap_or("(no title)");
            out.push_str(&format!(
                "- {}/{}: {:?}: {}\n",
                w.algorithm, title, w.kind, w.detail
            ));
        }
    }
    out
}

fn parse_hops(label: &Option<String>) -> Option<usize> {
    let l = label.as_ref()?;
    l.strip_suffix(" hops").and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::NodeCluster;

    #[test]
    fn safe_node_id_alphanumeric_unchanged() {
        assert_eq!(safe_node_id("foo", 42), "n_foo_42");
    }

    #[test]
    fn safe_node_id_dots_and_slashes_collapse() {
        assert_eq!(safe_node_id("src/foo/bar.c", 42), "n_src_foo_bar_c_42");
    }

    #[test]
    fn safe_node_id_non_ascii_collapses() {
        assert_eq!(safe_node_id("héllo.c", 1), "n_h_llo_c_1");
    }

    fn chain_fixture() -> SliceGraph {
        SliceGraph {
            title: Some("Data flow".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![
                GraphNode {
                    id: "a".to_string(),
                    label: "foo.c:42 read_input".to_string(),
                    kind: NodeKind::Source,
                    file: Some("foo.c".to_string()),
                    line: Some(42),
                },
                GraphNode {
                    id: "b".to_string(),
                    label: "foo.c:51 name".to_string(),
                    kind: NodeKind::Step,
                    file: Some("foo.c".to_string()),
                    line: Some(51),
                },
                GraphNode {
                    id: "c".to_string(),
                    label: "foo.c:67 strcpy".to_string(),
                    kind: NodeKind::Sink,
                    file: Some("foo.c".to_string()),
                    line: Some(67),
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "a".to_string(),
                    to: "b".to_string(),
                    label: Some("tainted".to_string()),
                    style: EdgeStyle::Solid,
                },
                GraphEdge {
                    from: "b".to_string(),
                    to: "c".to_string(),
                    label: Some("tainted".to_string()),
                    style: EdgeStyle::Solid,
                },
            ],
            clusters: vec![],
            mermaid: String::new(),
        }
    }

    #[test]
    fn render_chain_emits_flowchart_with_classes_and_arrows() {
        let g = chain_fixture();
        let out = render_chain(&g);
        assert!(out.starts_with("flowchart TD"));
        assert!(out.contains("a[\"foo.c:42 read_input\"]:::source"));
        assert!(out.contains("b[\"foo.c:51 name\"]"));
        assert!(out.contains("c[\"foo.c:67 strcpy\"]:::sink"));
        assert!(out.contains("a -->|\"tainted\"| b"));
        assert!(out.contains("b -->|\"tainted\"| c"));
        assert!(out.contains("classDef source"));
        assert!(out.contains("classDef sink"));
    }

    #[test]
    fn render_chain_unlabeled_edges_use_plain_arrow() {
        let mut g = chain_fixture();
        for e in &mut g.edges {
            e.label = None;
        }
        let out = render_chain(&g);
        assert!(out.contains("a --> b"));
        assert!(!out.contains("|tainted|"));
    }

    fn layered_fixture() -> SliceGraph {
        SliceGraph {
            title: Some("Layered call graph".to_string()),
            shape: GraphShape::Layered,
            nodes: vec![
                GraphNode {
                    id: "h".to_string(),
                    label: "handler.c:10".to_string(),
                    kind: NodeKind::Step,
                    file: None,
                    line: None,
                },
                GraphNode {
                    id: "s".to_string(),
                    label: "service.c:22".to_string(),
                    kind: NodeKind::Step,
                    file: None,
                    line: None,
                },
                GraphNode {
                    id: "r".to_string(),
                    label: "repo.c:55".to_string(),
                    kind: NodeKind::Step,
                    file: None,
                    line: None,
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "h".to_string(),
                    to: "s".to_string(),
                    label: None,
                    style: EdgeStyle::Solid,
                },
                GraphEdge {
                    from: "s".to_string(),
                    to: "r".to_string(),
                    label: None,
                    style: EdgeStyle::Solid,
                },
            ],
            clusters: vec![
                NodeCluster {
                    label: "UI".to_string(),
                    node_ids: vec!["h".to_string()],
                },
                NodeCluster {
                    label: "Business".to_string(),
                    node_ids: vec!["s".to_string()],
                },
                NodeCluster {
                    label: "Data".to_string(),
                    node_ids: vec!["r".to_string()],
                },
            ],
            mermaid: String::new(),
        }
    }

    #[test]
    fn render_layered_emits_subgraphs_in_order() {
        let g = layered_fixture();
        let out = render_layered(&g);
        assert!(out.starts_with("flowchart TD\n"));
        assert!(out.contains("[\"UI\"]"));
        assert!(out.contains("[\"Business\"]"));
        assert!(out.contains("[\"Data\"]"));
        // Subgraph order matches cluster order:
        let ui_pos = out.find("[\"UI\"]").unwrap();
        let bz_pos = out.find("[\"Business\"]").unwrap();
        let dt_pos = out.find("[\"Data\"]").unwrap();
        assert!(ui_pos < bz_pos && bz_pos < dt_pos);
        // Cross-layer edge present:
        assert!(out.contains("h --> s"));
        assert!(out.contains("s --> r"));
    }

    #[test]
    fn render_layered_orphan_nodes_render_outside_subgraphs() {
        let mut g = layered_fixture();
        g.nodes.push(GraphNode {
            id: "x".to_string(),
            label: "loose".to_string(),
            kind: NodeKind::Step,
            file: None,
            line: None,
        });
        let out = render_layered(&g);
        assert!(out.contains("x[\"loose\"]"));
    }

    fn cycle_fixture() -> SliceGraph {
        SliceGraph {
            title: Some("Cycle".to_string()),
            shape: GraphShape::Cycle,
            nodes: vec![
                GraphNode {
                    id: "a".to_string(),
                    label: "a.c:1".to_string(),
                    kind: NodeKind::Step,
                    file: None,
                    line: None,
                },
                GraphNode {
                    id: "b".to_string(),
                    label: "b.c:1".to_string(),
                    kind: NodeKind::Step,
                    file: None,
                    line: None,
                },
                GraphNode {
                    id: "c".to_string(),
                    label: "c.c:1".to_string(),
                    kind: NodeKind::Step,
                    file: None,
                    line: None,
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "a".to_string(),
                    to: "b".to_string(),
                    label: None,
                    style: EdgeStyle::Solid,
                },
                GraphEdge {
                    from: "b".to_string(),
                    to: "c".to_string(),
                    label: None,
                    style: EdgeStyle::Solid,
                },
                GraphEdge {
                    from: "c".to_string(),
                    to: "a".to_string(),
                    label: Some("cycle".to_string()),
                    style: EdgeStyle::Bold,
                },
            ],
            clusters: vec![],
            mermaid: String::new(),
        }
    }

    #[test]
    fn render_cycle_uses_lr_orientation_and_bold_back_edge() {
        let g = cycle_fixture();
        let out = render_cycle(&g);
        assert!(out.starts_with("flowchart LR"));
        assert!(out.contains("a --> b"));
        assert!(out.contains("b --> c"));
        assert!(out.contains("c ==>|\"cycle\"| a"));
    }

    fn fanout_fixture() -> SliceGraph {
        SliceGraph {
            title: Some("Caller fanout".to_string()),
            shape: GraphShape::Fanout,
            nodes: vec![
                GraphNode {
                    id: "x".to_string(),
                    label: "parse".to_string(),
                    kind: NodeKind::Origin,
                    file: None,
                    line: None,
                },
                GraphNode {
                    id: "c1".to_string(),
                    label: "main".to_string(),
                    kind: NodeKind::Caller,
                    file: None,
                    line: None,
                },
                GraphNode {
                    id: "c2".to_string(),
                    label: "worker".to_string(),
                    kind: NodeKind::Caller,
                    file: None,
                    line: None,
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "c1".to_string(),
                    to: "x".to_string(),
                    label: None,
                    style: EdgeStyle::Solid,
                },
                GraphEdge {
                    from: "c2".to_string(),
                    to: "x".to_string(),
                    label: None,
                    style: EdgeStyle::Solid,
                },
            ],
            clusters: vec![],
            mermaid: String::new(),
        }
    }

    #[test]
    fn render_fanout_uses_lr_and_marks_origin() {
        let g = fanout_fixture();
        let out = render_fanout(&g);
        assert!(out.starts_with("flowchart LR"));
        // Origin uses doubled brackets for stadium shape, with quoted label.
        assert!(out.contains("x[(\"parse\")]:::origin"));
        assert!(out.contains("c1[\"main\"]:::caller"));
        assert!(out.contains("c1 --> x"));
    }

    fn linear_chain(n: usize) -> SliceGraph {
        let nodes: Vec<GraphNode> = (0..n)
            .map(|i| GraphNode {
                id: format!("n{}", i),
                label: format!("step {}", i),
                kind: if i == 0 {
                    NodeKind::Source
                } else if i == n - 1 {
                    NodeKind::Sink
                } else {
                    NodeKind::Step
                },
                file: None,
                line: None,
            })
            .collect();
        let edges: Vec<GraphEdge> = (0..n - 1)
            .map(|i| GraphEdge {
                from: format!("n{}", i),
                to: format!("n{}", i + 1),
                label: None,
                style: EdgeStyle::Solid,
            })
            .collect();
        SliceGraph {
            title: None,
            shape: GraphShape::Chain,
            nodes,
            edges,
            clusters: vec![],
            mermaid: String::new(),
        }
    }

    #[test]
    fn truncate_below_cap_unchanged() {
        let g = linear_chain(10);
        let (out, warns) = truncate_to_cap(&g, 40);
        assert_eq!(out.nodes.len(), 10);
        assert!(warns.is_empty());
    }

    #[test]
    fn truncate_collapses_linear_chain_first() {
        // 50-node linear chain. Pass 1 collapses linear pass-through nodes.
        // The endpoints (Source, Sink) should be preserved.
        let g = linear_chain(50);
        let (out, warns) = truncate_to_cap(&g, 40);
        assert!(out.nodes.len() <= 40);
        assert_eq!(out.nodes.first().unwrap().kind, NodeKind::Source);
        assert_eq!(out.nodes.last().unwrap().kind, NodeKind::Sink);
        assert!(warns
            .iter()
            .any(|w| matches!(w.kind, DiagramWarningKind::NodeCapExceeded)));
    }

    #[test]
    fn render_dispatches_by_shape() {
        let mut g = chain_fixture();
        g.shape = GraphShape::Chain;
        let (out, warns) = render(&g, 40);
        assert!(out.starts_with("flowchart TD"));
        assert!(warns.is_empty());

        g.shape = GraphShape::Cycle;
        let (out, _) = render(&g, 40);
        assert!(out.starts_with("flowchart LR"));
    }

    #[test]
    fn render_emits_dangling_edge_warning() {
        let mut g = chain_fixture();
        g.edges.push(GraphEdge {
            from: "a".to_string(),
            to: "missing".to_string(),
            label: None,
            style: EdgeStyle::Solid,
        });
        let (_out, warns) = render(&g, 40);
        assert!(warns
            .iter()
            .any(|w| matches!(w.kind, DiagramWarningKind::DanglingEdge)));
    }

    #[test]
    fn render_emits_duplicate_node_warning() {
        let mut g = chain_fixture();
        let dup = g.nodes[0].clone();
        g.nodes.push(dup);
        let (_out, warns) = render(&g, 40);
        assert!(warns
            .iter()
            .any(|w| matches!(w.kind, DiagramWarningKind::DuplicateNodeId)));
    }

    #[test]
    fn render_emits_empty_graph_warning() {
        let g = SliceGraph {
            title: None,
            shape: GraphShape::Chain,
            nodes: vec![],
            edges: vec![],
            clusters: vec![],
            mermaid: String::new(),
        };
        let (_out, warns) = render(&g, 40);
        assert!(warns
            .iter()
            .any(|w| matches!(w.kind, DiagramWarningKind::EmptyGraph)));
    }

    #[test]
    fn truncate_head_tail_with_ellipsis_when_collapse_insufficient() {
        // 100-leaf star. Pass 1 cannot collapse (each leaf has indeg=0/outdeg=1, center has indeg=100/outdeg=0).
        // Pass 2 takes over with head/tail elision.
        let mut nodes = vec![GraphNode {
            id: "center".to_string(),
            label: "center".to_string(),
            kind: NodeKind::Origin,
            file: None,
            line: None,
        }];
        let mut edges = vec![];
        for i in 0..100 {
            nodes.push(GraphNode {
                id: format!("leaf{}", i),
                label: format!("leaf {}", i),
                kind: NodeKind::Caller,
                file: None,
                line: None,
            });
            edges.push(GraphEdge {
                from: format!("leaf{}", i),
                to: "center".to_string(),
                label: None,
                style: EdgeStyle::Solid,
            });
        }
        let g = SliceGraph {
            title: None,
            shape: GraphShape::Fanout,
            nodes,
            edges,
            clusters: vec![],
            mermaid: String::new(),
        };
        let (out, warns) = truncate_to_cap(&g, 40);
        assert!(out.nodes.len() <= 40);
        assert!(out
            .nodes
            .iter()
            .any(|n| n.label.contains("more nodes elided")));
        assert!(warns
            .iter()
            .any(|w| matches!(w.kind, DiagramWarningKind::NodeCapExceeded)));
    }

    #[test]
    fn format_mermaid_report_groups_by_algorithm() {
        use crate::slice::{MultiSliceResult, SliceFinding, SliceResult, SlicingAlgorithm};
        use std::collections::BTreeMap;
        let mut taint = SliceResult::new(SlicingAlgorithm::Taint);
        taint.findings.push(SliceFinding {
            algorithm: "Taint".to_string(),
            file: "foo.c".to_string(),
            line: 67,
            severity: "concern".to_string(),
            description: "tainted strcpy".to_string(),
            function_name: None,
            related_lines: vec![],
            related_files: vec![],
            category: Some("taint_sink".to_string()),
            parse_quality: None,
            diagrams: vec![SliceGraph {
                title: Some("Data flow".to_string()),
                shape: GraphShape::Chain,
                nodes: vec![GraphNode {
                    id: "n1".to_string(),
                    label: "src".to_string(),
                    kind: NodeKind::Source,
                    file: None,
                    line: None,
                }],
                edges: vec![],
                clusters: vec![],
                mermaid: "flowchart TD\n    n1[\"src\"]:::source\n".to_string(),
            }],
        });
        let multi = MultiSliceResult {
            version: "test".to_string(),
            algorithms_run: vec!["Taint".to_string()],
            results: vec![taint],
            findings: vec![],
            errors: vec![],
            warnings: vec![],
            parse_quality: BTreeMap::new(),
            diagram_warnings: vec![],
        };
        let report = format_mermaid_report(&multi);
        assert!(report.starts_with("# Prism diagram report"));
        assert!(report.contains("## Taint"));
        assert!(report.contains("### Finding 1"));
        assert!(report.contains("flowchart TD"));
    }

    #[test]
    fn format_mermaid_report_empty_run_says_nothing_produced() {
        use crate::slice::MultiSliceResult;
        use std::collections::BTreeMap;
        let multi = MultiSliceResult {
            version: "test".to_string(),
            algorithms_run: vec![],
            results: vec![],
            findings: vec![],
            errors: vec![],
            warnings: vec![],
            parse_quality: BTreeMap::new(),
            diagram_warnings: vec![],
        };
        let report = format_mermaid_report(&multi);
        assert!(report.contains("No flow-shaped findings produced"));
    }

    fn long_cycle(n: usize) -> SliceGraph {
        let nodes: Vec<GraphNode> = (0..n)
            .map(|i| GraphNode {
                id: format!("c{}", i),
                label: format!("step {}", i),
                kind: NodeKind::Step,
                file: None,
                line: None,
            })
            .collect();
        let mut edges: Vec<GraphEdge> = (0..n - 1)
            .map(|i| GraphEdge {
                from: format!("c{}", i),
                to: format!("c{}", i + 1),
                label: None,
                style: EdgeStyle::Solid,
            })
            .collect();
        // Bold back-edge: last → first
        edges.push(GraphEdge {
            from: format!("c{}", n - 1),
            to: "c0".to_string(),
            label: Some("cycle".to_string()),
            style: EdgeStyle::Bold,
        });
        SliceGraph {
            title: Some("Cycle".to_string()),
            shape: GraphShape::Cycle,
            nodes,
            edges,
            clusters: vec![],
            mermaid: String::new(),
        }
    }

    #[test]
    fn truncate_cycle_preserves_back_edge_endpoints() {
        let g = long_cycle(50);
        let (out, warns) = truncate_to_cap(&g, 10);
        // Both endpoints of the bold back-edge survive truncation.
        let has_first = out.nodes.iter().any(|n| n.id == "c0");
        let has_last = out.nodes.iter().any(|n| n.id == "c49");
        assert!(has_first, "back-edge endpoint c0 must survive");
        assert!(has_last, "back-edge endpoint c49 must survive");
        // Bold back-edge edge must still be present and labeled "cycle".
        let bold_edge = out
            .edges
            .iter()
            .find(|e| matches!(e.style, EdgeStyle::Bold));
        assert!(bold_edge.is_some(), "bold back-edge must survive");
        let bold_edge = bold_edge.unwrap();
        assert_eq!(bold_edge.from, "c49");
        assert_eq!(bold_edge.to, "c0");
        // Warning should fire (cycle exceeded cap, even if not fully truncated).
        assert!(warns
            .iter()
            .any(|w| matches!(w.kind, DiagramWarningKind::NodeCapExceeded)));
    }

    #[test]
    fn truncate_small_cycle_unchanged() {
        let g = long_cycle(3);
        let (out, warns) = truncate_to_cap(&g, 40);
        assert_eq!(out.nodes.len(), 3);
        assert!(warns.is_empty());
    }

    #[test]
    fn format_mermaid_report_includes_diagnostics_when_warnings_present() {
        use crate::slice::{DiagramWarningKind, MultiSliceResult, SliceResult, SlicingAlgorithm};
        use std::collections::BTreeMap;
        let mut r = SliceResult::new(SlicingAlgorithm::Taint);
        r.diagram_warnings.push(DiagramWarning {
            algorithm: "Taint".to_string(),
            graph_title: Some("Data flow".to_string()),
            kind: DiagramWarningKind::DanglingEdge,
            detail: "n_a missing".to_string(),
        });
        let multi = MultiSliceResult {
            version: "test".to_string(),
            algorithms_run: vec!["Taint".to_string()],
            results: vec![r],
            findings: vec![],
            errors: vec![],
            warnings: vec![],
            parse_quality: BTreeMap::new(),
            diagram_warnings: vec![],
        };
        let report = format_mermaid_report(&multi);
        assert!(report.contains("## Diagnostics"));
        assert!(report.contains("DanglingEdge"));
    }

    // ── Problem 1: Pass-2 truncation shape-aware tests ────────────────────────

    #[test]
    fn truncate_fanout_preserves_tail_caller_via_ghost() {
        // 100-leaf star: center is the origin, leaves are callers. Edges are leaf -> center.
        // After truncation, EVERY preserved leaf must reach the center: either via a direct
        // edge OR via a ghost path (leaf -> ghost AND ghost -> center).
        let mut nodes = vec![GraphNode {
            id: "center".to_string(),
            label: "center".to_string(),
            kind: NodeKind::Origin,
            file: None,
            line: None,
        }];
        let mut edges = vec![];
        for i in 0..100 {
            nodes.push(GraphNode {
                id: format!("leaf{}", i),
                label: format!("leaf {}", i),
                kind: NodeKind::Caller,
                file: None,
                line: None,
            });
            edges.push(GraphEdge {
                from: format!("leaf{}", i),
                to: "center".to_string(),
                label: None,
                style: EdgeStyle::Solid,
            });
        }
        let g = SliceGraph {
            title: None,
            shape: GraphShape::Fanout,
            nodes,
            edges,
            clusters: vec![],
            mermaid: String::new(),
        };
        let (out, _warns) = truncate_to_cap(&g, 10);
        let ghost_id = out
            .nodes
            .iter()
            .find(|n| n.label.contains("more nodes elided"))
            .map(|n| n.id.clone())
            .expect("ghost node must exist");
        // Whether the ghost has its outgoing edge to center.
        let ghost_reaches_center = out
            .edges
            .iter()
            .any(|e| e.from == ghost_id && e.to == "center");
        // For EVERY preserved leaf, assert it can reach center either directly or via ghost.
        for n in &out.nodes {
            if n.id == "center" || n.id == ghost_id {
                continue;
            }
            let direct = out.edges.iter().any(|e| e.from == n.id && e.to == "center");
            let leaf_to_ghost = out.edges.iter().any(|e| e.from == n.id && e.to == ghost_id);
            let via_ghost = leaf_to_ghost && ghost_reaches_center;
            assert!(
                direct || via_ghost,
                "leaf {} is disconnected (no direct edge to center, no leaf->ghost edge AND/OR ghost has no edge to center)",
                n.id
            );
        }
    }

    #[test]
    fn truncate_chain_crossing_edge_emits_both_ghost_halves() {
        // Build a 50-node chain where every node is Source or Sink (non-collapsible),
        // so Pass 1 cannot reduce any nodes and Pass 2 must run.
        // After truncation the head and tail are bridged by edges through the ghost.
        let nodes: Vec<GraphNode> = (0..50)
            .map(|i| GraphNode {
                id: format!("n{}", i),
                label: format!("step {}", i),
                kind: if i % 2 == 0 {
                    NodeKind::Source
                } else {
                    NodeKind::Sink
                },
                file: None,
                line: None,
            })
            .collect();
        let edges: Vec<GraphEdge> = (0..49)
            .map(|i| GraphEdge {
                from: format!("n{}", i),
                to: format!("n{}", i + 1),
                label: None,
                style: EdgeStyle::Solid,
            })
            .collect();
        let g = SliceGraph {
            title: None,
            shape: GraphShape::Chain,
            nodes,
            edges,
            clusters: vec![],
            mermaid: String::new(),
        };
        let (out, _warns) = truncate_to_cap(&g, 10);
        let ghost_id = out
            .nodes
            .iter()
            .find(|n| n.label.contains("more nodes elided"))
            .map(|n| n.id.clone())
            .expect("ghost node must exist");
        // Both `head_to_ghost` and `ghost_to_tail` directions should have at least one edge.
        let has_head_to_ghost = out.edges.iter().any(|e| e.to == ghost_id);
        let has_ghost_to_tail = out.edges.iter().any(|e| e.from == ghost_id);
        assert!(
            has_head_to_ghost,
            "expected at least one edge ending at ghost"
        );
        assert!(
            has_ghost_to_tail,
            "expected at least one edge starting at ghost"
        );
    }

    // ── MIN_CAP clamp test ────────────────────────────────────────────────────

    #[test]
    fn truncate_clamps_cap_at_minimum() {
        // cap=1 should be clamped to MIN_CAP=4 internally; a 6-node chain
        // stays at exactly 4 nodes after truncation (head=2, ghost=1, tail=1).
        let g = linear_chain(6);
        let (out, _) = truncate_to_cap(&g, 1);
        assert!(
            out.nodes.len() >= 3,
            "must preserve at least head + ghost + tail (got {})",
            out.nodes.len()
        );
        assert!(
            out.nodes.len() <= 4,
            "must not exceed clamped minimum of 4 (got {})",
            out.nodes.len()
        );
    }

    // ── Problem 2: Edge label and cluster label escaping tests ────────────────

    #[test]
    fn render_chain_escapes_edge_label_with_pipe() {
        let mut g = chain_fixture();
        g.edges[0].label = Some("a|b".to_string());
        let out = render_chain(&g);
        // The literal `|a|b|` must NOT appear; the escaped form has the label quoted.
        assert!(!out.contains("|a|b|"));
        assert!(out.contains("|\"a|b\"|"));
    }

    #[test]
    fn render_chain_escapes_edge_label_with_newline() {
        let mut g = chain_fixture();
        g.edges[0].label = Some("a\nb".to_string());
        let out = render_chain(&g);
        // Newlines convert to <br/> via escape_label_inner; the rendered edge
        // label must not contain a raw newline that would break the line.
        assert!(!out.contains("|a\nb|"));
    }

    #[test]
    fn render_layered_uses_quoted_subgraph_id_for_keyword_label() {
        let mut g = layered_fixture();
        g.clusters[0].label = "end".to_string();
        let out = render_layered(&g);
        // Mermaid: `subgraph end` is parsed as the end of a block.
        // After fix, the rendered string must NOT contain `subgraph end\n`.
        assert!(!out.contains("subgraph end\n"));
        // It SHOULD contain a quoted label form, e.g. `subgraph cluster_0["end"]`.
        assert!(out.contains("[\"end\"]"));
    }

    #[test]
    fn render_layered_escapes_cluster_label_with_brackets() {
        let mut g = layered_fixture();
        g.clusters[0].label = "UI [main]".to_string();
        let out = render_layered(&g);
        // The label inside the quotes is preserved character-for-character
        // (escape_label_inner doesn't strip brackets).
        assert!(out.contains("[\"UI [main]\"]"));
    }

    #[test]
    fn render_emits_label_truncated_for_long_edge_label() {
        let mut g = chain_fixture();
        let long: String = "a".repeat(120);
        g.edges[0].label = Some(long);
        let (_out, warns) = render(&g, 40);
        assert!(
            warns
                .iter()
                .any(|w| matches!(w.kind, DiagramWarningKind::LabelTruncated)
                    && w.detail.contains("edge label")),
            "expected LabelTruncated for edge label, got: {:?}",
            warns
        );
    }

    #[test]
    fn render_emits_label_truncated_for_long_cluster_label() {
        let mut g = layered_fixture();
        let long: String = "u".repeat(120);
        g.clusters[0].label = long;
        let (_out, warns) = render(&g, 40);
        assert!(
            warns
                .iter()
                .any(|w| matches!(w.kind, DiagramWarningKind::LabelTruncated)
                    && w.detail.contains("cluster label")),
            "expected LabelTruncated for cluster label, got: {:?}",
            warns
        );
    }
}
