//! File-bound module identity and the deliberately narrow evaluation-region policy.
use super::{Language, Node, ParsedFile};

/// Reserved non-positional member of the shared anonymous-owner namespace.
/// `FunctionId.file` supplies file identity; no source position is encoded here.
/// Keeping the A2 prefix makes every S1 guard and the Exact barrier apply.
pub const MODULE_OWNER_NAME: &str = "<anon@module>";

pub fn is_module_owner_name(name: &str) -> bool {
    name == MODULE_OWNER_NAME
}

pub(super) fn is_module_node(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) && node.kind() == "program"
        && node.id() == parsed.tree.root_node().id()
}

/// Callable ownership and legacy named claims are checked by the shared caller.
/// Class evaluation and computed keys await an explicit evaluation-region policy.
pub(super) fn ordinary_region(parsed: &ParsedFile, start: usize, end: usize) -> bool {
    let mut current = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, end);
    while let Some(node) = current {
        if matches!(node.kind(), "class_body" | "computed_property_name") {
            return false;
        }
        current = node.parent();
    }
    true
}
