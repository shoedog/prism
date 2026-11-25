use super::census::{Binding, Kind};
use crate::targets::inventory::python::source::children;
use tree_sitter::Node;

/// A class namespace is a mapping updated as the suite executes. Keep
/// absence as a separate result so lookup can apply the correct fallback.
pub(super) fn states<'a, 'tree>(
    body: Node<'tree>,
    bindings: &'a [Binding<'tree>],
    at: Node<'tree>,
) -> Vec<Option<&'a Binding<'tree>>> {
    let implicit = bindings.iter().find(|binding| binding.node == body);
    suite(body, bindings, vec![implicit], at.start_byte())
}

fn suite<'a, 'tree>(
    suite_node: Node<'tree>,
    bindings: &'a [Binding<'tree>],
    mut values: Vec<Option<&'a Binding<'tree>>>,
    limit: usize,
) -> Vec<Option<&'a Binding<'tree>>> {
    for stmt in children(suite_node).take_while(|stmt| stmt.start_byte() < limit) {
        if stmt.kind() == "if_statement" {
            values = conditional(stmt, bindings, values, limit);
        } else {
            let conditional = matches!(
                stmt.kind(),
                "for_statement" | "while_statement" | "try_statement" | "match_statement"
            );
            let mut possible = values.clone();
            for binding in bindings.iter().filter(|binding| {
                stmt.start_byte() <= binding.node.start_byte()
                    && binding.node.end_byte() <= stmt.end_byte()
                    && binding.position <= limit
            }) {
                match binding.kind {
                    Kind::Annotation => {}
                    Kind::Delete => possible = vec![None],
                    _ => possible = vec![Some(binding)],
                }
                if conditional {
                    join(&mut values, &possible);
                }
            }
            if !conditional {
                values = possible;
            }
        }
    }
    values
}

fn conditional<'a, 'tree>(
    stmt: Node<'tree>,
    bindings: &'a [Binding<'tree>],
    entry: Vec<Option<&'a Binding<'tree>>>,
    limit: usize,
) -> Vec<Option<&'a Binding<'tree>>> {
    let arms = std::iter::once(stmt)
        .chain(children(stmt).filter(|arm| matches!(arm.kind(), "elif_clause" | "else_clause")));
    let mut exits = Vec::new();
    for arm in arms {
        let is_else = arm.kind() == "else_clause";
        let Some(body) = arm.child_by_field_name(if is_else { "body" } else { "consequence" })
        else {
            continue;
        };
        // Only the path which skipped the preceding suites evaluates this test.
        if arm.start_byte() <= limit && limit < body.start_byte() {
            return entry;
        }
        if body.start_byte() <= limit && limit < body.end_byte() {
            return suite(body, bindings, entry, limit);
        }
        join(&mut exits, &suite(body, bindings, entry.clone(), limit));
        if is_else {
            return exits;
        }
    }
    join(&mut exits, &entry);
    exits
}

fn join<'a, 'tree>(
    target: &mut Vec<Option<&'a Binding<'tree>>>,
    values: &[Option<&'a Binding<'tree>>],
) {
    for &value in values {
        if !target.iter().any(|existing| match (*existing, value) {
            (Some(left), Some(right)) => std::ptr::eq(left, right),
            (None, None) => true,
            _ => false,
        }) {
            target.push(value);
        }
    }
}
