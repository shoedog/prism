//! Split declaration headers keep main's behaviour (#694 slice A, fold 2; owner decision
//! 2026-11-07, the real fix is #728).
//!
//! Queries and taint connect a Use to a Def only on one line. A range/receive `:=`, a
//! type-switch alias or a `var`/`const` spec is *split* when any token of its right-hand side
//! (the range expression, the received channel expression, the asserted expression, the spec's
//! values) is on a different row from any of its declared names: a right-hand read on row N
//! never reaches the new def on row M, so no replacement path exists. Such a header keeps
//! main's categorical exemption: rule S and rule O keep every pair of a name it declares
//! (`address_taken_names`), and rule D never takes a split spec as a dominating write.
//! Rule U keeps its declare-only anchor handling: the anchor line holds only the declaration,
//! and the kept S pair carries the outer value to the body use.
use tree_sitter::Node;

/// The declared identifiers of `node` when it is a split header or spec, else empty.
pub(super) fn split_declared(node: Node<'_>) -> Vec<Node<'_>> {
    let (names, rhs) = match node.kind() {
        "range_clause" | "receive_statement" => {
            let mut cursor = node.walk();
            let declares = node.children(&mut cursor).any(|c| c.kind() == ":=");
            (
                node.child_by_field_name("left").filter(|_| declares),
                node.child_by_field_name("right"),
            )
        }
        "type_switch_statement" => (
            node.child_by_field_name("alias"),
            node.child_by_field_name("value"),
        ),
        "var_spec" | "const_spec" => {
            let mut cursor = node.walk();
            let first = node.children_by_field_name("name", &mut cursor).next();
            return match (first, node.child_by_field_name("value")) {
                (Some(_), Some(rhs)) => {
                    let mut cursor = node.walk();
                    let ids: Vec<_> = node
                        .children_by_field_name("name", &mut cursor)
                        .filter(|n| n.kind() == "identifier")
                        .collect();
                    keep_split(ids, rhs)
                }
                _ => Vec::new(),
            };
        }
        _ => (None, None),
    };
    let (Some(names), Some(rhs)) = (names, rhs) else {
        return Vec::new();
    };
    let ids: Vec<_> = if names.kind() == "identifier" {
        vec![names]
    } else {
        let mut cursor = names.walk();
        names
            .named_children(&mut cursor)
            .filter(|n| n.kind() == "identifier")
            .collect()
    };
    keep_split(ids, rhs)
}

/// All of `ids` when some token of `rhs` sits on a row other than some name's row. The right-hand
/// side's first and last tokens bound its rows, so comparing its first and last rows is exact.
fn keep_split<'a>(ids: Vec<Node<'a>>, rhs: Node<'_>) -> Vec<Node<'a>> {
    let (first, last) = (rhs.start_position().row, rhs.end_position().row);
    if ids
        .iter()
        .any(|id| id.start_position().row != first || id.start_position().row != last)
    {
        ids
    } else {
        Vec::new()
    }
}
