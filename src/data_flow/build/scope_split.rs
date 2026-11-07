//! Rule S exemption for split declaration headers (#694 slice A, fold 1).
//!
//! Queries and taint connect a Use to a Def only on one line. A range/receive `:=`,
//! type-switch alias or `var`/`const` spec whose target identifier sits on a different
//! line than some part of its right-hand side has no connected replacement path
//! (a right-hand read on line N never reaches the new def on line M), so the outer
//! def's pair with the body use is the only path left and S must keep it.
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Start bytes of the declared identifiers of every header or spec in `owner` whose right-hand
/// side reads an identifier on a line other than the declared name's.
pub(super) fn split_target_bytes(owner: Node<'_>) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    walk(owner, &mut out);
    out
}

fn walk(node: Node<'_>, out: &mut BTreeSet<usize>) {
    let (target, rhs) = match node.kind() {
        "range_clause" | "receive_statement" => {
            let mut cursor = node.walk();
            let declares = node.children(&mut cursor).any(|c| c.kind() == ":=");
            if declares {
                (
                    node.child_by_field_name("left"),
                    node.child_by_field_name("right"),
                )
            } else {
                (None, None)
            }
        }
        "type_switch_statement" => (
            node.child_by_field_name("alias"),
            node.child_by_field_name("value"),
        ),
        "var_spec" | "const_spec" => {
            let mut cursor = node.walk();
            let name = node.children_by_field_name("name", &mut cursor).next();
            (name, node.child_by_field_name("value"))
        }
        _ => (None, None),
    };
    if let (Some(rhs), true) = (rhs, target.is_some()) {
        let mut ids = Vec::new();
        if node.kind() == "var_spec" || node.kind() == "const_spec" {
            let mut cursor = node.walk();
            ids.extend(node.children_by_field_name("name", &mut cursor));
        } else if let Some(t) = target {
            if t.kind() == "identifier" {
                ids.push(t);
            } else {
                let mut cursor = t.walk();
                ids.extend(t.named_children(&mut cursor));
            }
        }
        // Rows of the identifier reads in the right-hand side (literals read nothing).
        let mut rows = BTreeSet::new();
        read_rows(rhs, &mut rows);
        for id in ids.into_iter().filter(|i| i.kind() == "identifier") {
            if rows.iter().any(|r| *r != id.start_position().row) {
                out.insert(id.start_byte());
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(child, out);
    }
}

/// Rows of every identifier in `node` (a call's callee, a selector's operand, a name).
fn read_rows(node: Node<'_>, rows: &mut BTreeSet<usize>) {
    if node.kind() == "identifier" {
        rows.insert(node.start_position().row);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        read_rows(child, rows);
    }
}
