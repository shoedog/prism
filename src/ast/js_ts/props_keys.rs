//! Shared JS-family property key and erased value syntax.

use crate::ast::ParsedFile;
use tree_sitter::Node;

pub(crate) fn is_transparent_wrapper(kind: &str) -> bool {
    matches!(
        kind,
        "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion"
    )
}

/// An angle assertion carries its value last. The other wrappers carry it first.
pub(crate) fn strip_transparent_wrappers(mut node: Node<'_>) -> Option<Node<'_>> {
    while is_transparent_wrapper(node.kind()) {
        let mut cursor = node.walk();
        let mut inner = node.named_children(&mut cursor).filter(|c| !c.is_extra());
        node = if node.kind() == "type_assertion" {
            inner.last()?
        } else {
            inner.next()?
        };
    }
    Some(node)
}

/// A single key segment from a syntax token. Keep quoted contents intact and
/// refuse computed, escaped, non-ASCII and noncanonical numeric spellings.
pub(crate) fn definite_key(parsed: &ParsedFile, node: Node<'_>) -> Option<String> {
    let text = parsed.node_text(&node);
    if !text.is_ascii() || text.contains('\\') {
        return None;
    }
    let key = match node.kind() {
        "identifier"
        | "property_identifier"
        | "shorthand_property_identifier"
        | "shorthand_property_identifier_pattern" => text,
        "string" => text.get(1..text.len().checked_sub(1)?)?,
        "number" if text == "0" || !text.starts_with('0') => {
            if !text.bytes().all(|byte| byte.is_ascii_digit())
                || text.parse::<u64>().ok()? > 9_007_199_254_740_991
            {
                return None;
            }
            text
        }
        _ => return None,
    };
    Some(key.to_owned())
}
