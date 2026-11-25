//! Runtime operands and owner-relative lexical environments for S1.

use crate::ast::ParsedFile;
use crate::js_write_census::tables::Tables;
use tree_sitter::Node;

pub(super) fn runtime_node(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if node.is_error()
        || node.is_missing()
        || parsed.language.is_erased_type_boundary(node)
        || Tables::shared().order_class(node.kind()) == Some("none")
    {
        return false;
    }
    if matches!(node.kind(), "field_definition" | "public_field_definition") {
        let mut cursor = node.walk();
        return !node
            .children(&mut cursor)
            .any(|child| matches!(child.kind(), "declare" | "abstract"));
    }
    // Angle assertions use a type_arguments subtree before the operand.
    true
}

pub(super) fn decorated_callable(decorator: Node<'_>) -> Option<Node<'_>> {
    if decorator.kind() != "decorator" {
        return None;
    }
    let parameter = decorator.parent()?;
    let list = parameter.parent()?;
    (matches!(
        parameter.kind(),
        "required_parameter" | "optional_parameter"
    ) && list.kind() == "formal_parameters")
        .then(|| list.parent())
        .flatten()
}

pub(super) fn binding_scope(
    parsed: &ParsedFile,
    owner: Node<'_>,
    operand: Node<'_>,
    name: &str,
) -> Option<usize> {
    if parsed.js_ts_span_in_with_body(operand.start_byte(), operand.end_byte()) {
        return None;
    }
    let mut ancestor = operand.parent();
    let mut ignored = Vec::new();
    while let Some(node) = ancestor {
        if let Some(callable) = decorated_callable(node) {
            ignored.push(callable);
        }
        if node == owner {
            break;
        }
        if name == "arguments"
            && !ignored.contains(&node)
            && node.kind() != "arrow_function"
            && parsed
                .language
                .callable_boundary_node_types()
                .contains(&node.kind())
            && ["body", "parameters"].iter().any(|field| {
                node.child_by_field_name(field)
                    .is_some_and(|part| part.byte_range().contains(&operand.start_byte()))
            })
        {
            // Even an implicit arguments binding proves a different binding.
            return Some(node.id());
        }
        ancestor = node.parent();
    }
    if ignored.is_empty() {
        parsed.js_ts_def_scope_id(operand.start_byte(), name)
    } else {
        parsed.js_ts_pattern_decorator_scope(operand, name, &ignored)
    }
}
