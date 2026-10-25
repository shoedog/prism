//! Rust-specific lexical scope derivation.

use super::{scope_span, ScopeSpan};
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn closure_parameter_scope(parsed: &ParsedFile, node: Node<'_>) -> Option<ScopeSpan> {
    (parsed.language == Language::Rust && node.kind() == "closure_parameters")
        .then(|| node.parent())
        .flatten()
        .filter(|parent| parent.kind() == "closure_expression")
        .and_then(|closure| closure.child_by_field_name("body"))
        .map(scope_span)
}
