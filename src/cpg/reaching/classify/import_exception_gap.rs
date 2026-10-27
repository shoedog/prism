use super::super::binding_table::select_binding_row;
use super::super::Line;
use super::edge_use_byte;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn unplaced(parsed: &ParsedFile, edge: &FlowEdge) -> Option<Line> {
    if parsed.language != Language::Python {
        return None;
    }
    let use_byte = edge_use_byte(parsed, edge)?;
    let function = parsed
        .all_functions()
        .into_iter()
        .find(|node| parsed.node_line_range(node).0 == edge.to.function_start_line)?;
    let mut imports = Vec::new();
    collect_import_statements(function, &mut imports);
    imports
        .into_iter()
        .filter(|import| {
            let line = import.start_position().row + 1;
            edge.from.line < line && import.end_byte() <= use_byte
        })
        .filter(|import| {
            select_binding_row(parsed, *import).is_some_and(|row| {
                super::super::transfer::row_introduced_identifiers(parsed, *import, row.fields)
                    .into_iter()
                    .any(|identifier| parsed.node_text(&identifier) == edge.to.path.base)
            })
        })
        .filter(|import| exception_bypass_has_no_target(parsed, *import))
        .map(|import| import.start_position().row + 1)
        .min()
}

fn collect_import_statements<'a>(node: Node<'a>, imports: &mut Vec<Node<'a>>) {
    if matches!(node.kind(), "import_statement" | "import_from_statement") {
        imports.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_import_statements(child, imports);
    }
}

fn exception_bypass_has_no_target(parsed: &ParsedFile, import: Node<'_>) -> bool {
    let Some(body) = import.parent().filter(|parent| parent.kind() == "block") else {
        return false;
    };
    let Some(try_statement) = body
        .parent()
        .filter(|parent| parent.kind() == "try_statement")
    else {
        return false;
    };
    if try_statement
        .child_by_field_name("body")
        .is_none_or(|try_body| try_body.id() != body.id())
    {
        return false;
    }
    let mut body_cursor = body.walk();
    let statements: Vec<_> = body
        .named_children(&mut body_cursor)
        .filter(|child| parsed.language.is_statement_node(child.kind()))
        .collect();
    if !matches!(statements.as_slice(), [only] if only.id() == import.id()) {
        return false;
    }

    let mut cursor = try_statement.walk();
    let clauses: Vec<_> = try_statement.named_children(&mut cursor).collect();
    let has_untracked_except = clauses.iter().any(|clause| {
        matches!(clause.kind(), "except_clause" | "except_group_clause")
            && first_tracked_statement_line(*clause, parsed).is_none()
    });
    let finally_first = clauses
        .iter()
        .find(|clause| clause.kind() == "finally_clause")
        .and_then(|clause| first_tracked_statement_line(*clause, parsed));
    has_untracked_except
        && finally_first
            .or_else(|| crate::cfg::python_structural_successor(try_statement, parsed))
            .is_none()
}

fn first_tracked_statement_line(node: Node<'_>, parsed: &ParsedFile) -> Option<Line> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if parsed.language.is_statement_node(child.kind()) {
            return Some(child.start_position().row + 1);
        }
        if child.kind() == "block" {
            if let Some(line) = first_tracked_statement_line(child, parsed) {
                return Some(line);
            }
        }
    }
    None
}
