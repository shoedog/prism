//! Mermaid flowchart rendering for SliceGraph.
//! See docs/superpowers/specs/2026-05-09-data-flow-visualization-design.md.

use std::collections::{BTreeMap, BTreeSet};

use crate::slice::{
    DiagramWarning, DiagramWarningKind, EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeCluster,
    NodeKind, SliceGraph,
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

/// Escape a label for safe inclusion inside `["…"]` in a Mermaid flowchart node.
/// Returns (escaped_label, was_truncated). Caller decides whether to emit a
/// LabelTruncated warning.
pub(crate) fn escape_label(s: &str) -> (String, bool) {
    const MAX: usize = 80;
    let needs_quote = s
        .chars()
        .any(|c| matches!(c, '[' | ']' | '<' | '>' | '|' | '(' | ')' | '"'));
    let mut out = s.replace('"', "&quot;").replace('\n', "<br/>");
    let truncated = out.chars().count() > MAX;
    if truncated {
        let take: String = out.chars().take(MAX - 1).collect();
        out = format!("{}…", take);
    }
    if needs_quote {
        (format!("\"{}\"", out), truncated)
    } else {
        (out, truncated)
    }
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
        Some(l) if !l.is_empty() => format!("{}|{}|", arrow, l),
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
    for cluster in &g.clusters {
        out.push_str(&format!("    subgraph {}\n", cluster.label));
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
pub(crate) fn truncate_to_cap(g: &SliceGraph, cap: usize) -> (SliceGraph, Vec<DiagramWarning>) {
    if g.nodes.len() <= cap {
        return (g.clone(), vec![]);
    }
    let original_count = g.nodes.len();
    let mut working = g.clone();

    // Pass 1: collapse linear chains. A node B with indegree=outdegree=1 and
    // not Source/Sink/Origin can be removed; its single in-edge and single
    // out-edge fuse into one edge labeled with hop count.
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
            let preserve = matches!(n.kind, NodeKind::Source | NodeKind::Sink | NodeKind::Origin);
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
    if working.nodes.len() > cap {
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
        let mut head_to_ghost = false;
        let mut ghost_to_tail = false;
        for e in &working.edges {
            let in_head = head_ids.contains(e.from.as_str()) && head_ids.contains(e.to.as_str());
            let in_tail = tail_ids.contains(e.from.as_str()) && tail_ids.contains(e.to.as_str());
            if in_head || in_tail {
                new_edges.push(e.clone());
            } else if head_ids.contains(e.from.as_str()) && !head_to_ghost {
                new_edges.push(GraphEdge {
                    from: e.from.clone(),
                    to: ghost_id.clone(),
                    label: None,
                    style: EdgeStyle::Dotted,
                });
                head_to_ghost = true;
            } else if tail_ids.contains(e.to.as_str()) && !ghost_to_tail {
                new_edges.push(GraphEdge {
                    from: ghost_id.clone(),
                    to: e.to.clone(),
                    label: None,
                    style: EdgeStyle::Dotted,
                });
                ghost_to_tail = true;
            }
        }
        let mut all_nodes: Vec<GraphNode> = head;
        all_nodes.push(ghost);
        all_nodes.extend(tail);
        working.nodes = all_nodes;
        working.edges = new_edges;
    }

    let warns = vec![DiagramWarning {
        algorithm: String::new(),
        graph_title: g.title.clone(),
        kind: DiagramWarningKind::NodeCapExceeded,
        detail: format!(
            "elided to {} nodes from original {}",
            working.nodes.len(),
            original_count
        ),
    }];
    (working, warns)
}

fn parse_hops(label: &Option<String>) -> Option<usize> {
    let l = label.as_ref()?;
    l.strip_suffix(" hops").and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn escape_label_plain_unchanged() {
        let (out, trunc) = escape_label("hello world");
        assert_eq!(out, "hello world");
        assert!(!trunc);
    }

    #[test]
    fn escape_label_brackets_get_quoted() {
        let (out, trunc) = escape_label("a[b]c");
        assert_eq!(out, "\"a[b]c\"");
        assert!(!trunc);
    }

    #[test]
    fn escape_label_quote_replaced() {
        let (out, _) = escape_label("a\"b");
        // Has special char (the original `"`) so wraps in quotes.
        assert_eq!(out, "\"a&quot;b\"");
    }

    #[test]
    fn escape_label_newline_to_br() {
        let (out, _) = escape_label("a\nb");
        // No bracket-class special chars, so no wrapping quotes.
        assert_eq!(out, "a<br/>b");
    }

    #[test]
    fn escape_label_truncates_at_80() {
        let long: String = "a".repeat(120);
        let (out, trunc) = escape_label(&long);
        assert!(trunc);
        assert!(out.chars().count() <= 80);
        assert!(out.ends_with('…'));
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
        assert!(out.contains("a -->|tainted| b"));
        assert!(out.contains("b -->|tainted| c"));
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
        assert!(out.contains("subgraph UI"));
        assert!(out.contains("subgraph Business"));
        assert!(out.contains("subgraph Data"));
        // Subgraph order matches cluster order:
        let ui_pos = out.find("subgraph UI").unwrap();
        let bz_pos = out.find("subgraph Business").unwrap();
        let dt_pos = out.find("subgraph Data").unwrap();
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
        assert!(out.contains("c ==>|cycle| a"));
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
}
