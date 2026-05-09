//! Mermaid flowchart rendering for SliceGraph.
//! See docs/superpowers/specs/2026-05-09-data-flow-visualization-design.md.

use crate::slice::{EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind, SliceGraph};

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
}
