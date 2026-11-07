//! Generic tree-sitter helpers shared by core analyses and the algorithms.
//!
//! Moved verbatim out of `algorithms/taint.rs` (#702) so core modules do not
//! import the review layer.

use super::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

/// `pub(crate)` (P9): reused by `framework_entries` for the same
/// parenthesized-expression unwrapping this module already does.
pub(crate) fn unwrap_parenthesized(mut node: Node<'_>) -> Node<'_> {
    loop {
        if node.kind() != "parenthesized_expression" {
            return node;
        }
        node = match node.named_child(0) {
            Some(child) => child,
            None => return node,
        };
    }
}

pub(crate) fn call_arg_node<'a>(call: &Node<'a>, arg_idx: usize) -> Option<Node<'a>> {
    let arguments = call.child_by_field_name("arguments").or_else(|| {
        let mut cursor = call.walk();
        let found = call
            .named_children(&mut cursor)
            .find(|child| child.kind() == "arguments");
        found
    })?;
    let mut cursor = arguments.walk();
    let arg = arguments.named_children(&mut cursor).nth(arg_idx);
    arg
}

/// Compute the textual call path for a call expression. For `r.URL.Query()`,
/// returns `Some("r.URL.Query")`; for Python `request.args.get()`, returns
/// `Some("request.args.get")`.
pub(crate) fn call_path_text(parsed: &ParsedFile, call_node: &Node<'_>) -> Option<String> {
    let func = parsed.call_function_node(call_node)?;
    Some(parsed.node_text(&func).to_string())
}

pub(crate) fn call_path_matches(parsed: &ParsedFile, actual: &str, expected: &str) -> bool {
    actual == expected
        || (parsed.language == Language::Python
            && !expected.contains('.')
            && actual
                .rsplit('.')
                .next()
                .is_some_and(|tail| tail == expected))
        || (is_js_ts_language(parsed.language)
            && !expected.contains('.')
            && actual
                .rsplit('.')
                .next()
                .is_some_and(|tail| tail == expected))
}

pub(crate) fn is_js_ts_language(language: Language) -> bool {
    matches!(
        language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}

/// Walk `root` collecting every call node for the file's language.
pub(crate) fn collect_calls<'a>(parsed: &ParsedFile, node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if parsed.language.is_call_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls(parsed, child, out);
    }
}
