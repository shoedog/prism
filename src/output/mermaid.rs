//! Mermaid flowchart rendering for SliceGraph.
//! See docs/superpowers/specs/2026-05-09-data-flow-visualization-design.md.

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
}
