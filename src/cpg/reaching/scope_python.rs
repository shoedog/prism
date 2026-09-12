use super::scope::{scope_span, Binding, BindingId, Declaration, ScopeSpan};
use crate::ast::ParsedFile;
use tree_sitter::Node;

pub(super) fn collect_python_comprehension_declarations(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_function_id: usize,
    declarations: &mut Vec<Declaration>,
) {
    if node.id() != root_function_id
        && matches!(
            node.kind(),
            "function_definition" | "lambda" | "class_definition"
        )
    {
        return;
    }

    if node.kind() == "for_in_clause" {
        let mut parent = node.parent();
        while let Some(candidate) = parent {
            if matches!(
                candidate.kind(),
                "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
            ) {
                if let Some(target) = node.child_by_field_name("left") {
                    let visible_from = node
                        .child_by_field_name("right")
                        .map(|iterable| iterable.end_byte())
                        .unwrap_or_else(|| node.end_byte());
                    collect_python_target_identifiers(
                        parsed,
                        target,
                        scope_span(candidate),
                        visible_from,
                        candidate.child_by_field_name("body").map(scope_span),
                        declarations,
                    );
                }
                break;
            }
            parent = candidate.parent();
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_python_comprehension_declarations(parsed, child, root_function_id, declarations);
    }
}

fn collect_python_target_identifiers(
    parsed: &ParsedFile,
    node: Node<'_>,
    scope: ScopeSpan,
    visible_from: usize,
    body: Option<ScopeSpan>,
    declarations: &mut Vec<Declaration>,
) {
    if node.kind() == "identifier" {
        let declaration_index = declarations.len();
        declarations.push(Declaration {
            binding: Binding {
                id: BindingId::Declaration(declaration_index),
                scope,
                declaration_line: Some(node.start_position().row + 1),
            },
            name: parsed.node_text(&node).to_string(),
            visible_from,
            additional_visibility: body.into_iter().chain([scope_span(node)]).collect(),
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_python_target_identifiers(parsed, child, scope, visible_from, body, declarations);
    }
}
