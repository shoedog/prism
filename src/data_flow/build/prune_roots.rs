//! Operand and root finding shared by the C rules (`c_prune.rs`) and the C++ additions
//! (`cpp_prune.rs`), #630 slice 6c fold 5. One definition of "the real operand child" and of
//! which wrappers hand their operand over unchanged, so no finder re-derives it with
//! `named_child(0)` (which is a `comment` when one precedes the operand:
//! `&(/*c*/ x)` was invisible to every root finder, in `.c` files too).
use tree_sitter::Node;

/// The first named child that is not a comment: the real operand of a wrapper node.
pub(super) fn operand(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|c| c.kind() != "comment");
    found
}

/// A transparent wrapper (`( E )`, GNU `__extension__ E`, `_Generic(c, T: E, ..)`): pushes the
/// expression operands it hands over as lvalues and returns true. A `_Generic` pushes its
/// controlling expression too (never evaluated, so that only costs recall).
pub(super) fn wrapper_operands<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) -> bool {
    match node.kind() {
        "parenthesized_expression" | "extension_expression" => {
            out.extend(operand(node));
            true
        }
        "generic_expression" => {
            let mut cursor = node.walk();
            out.extend(
                node.named_children(&mut cursor)
                    .filter(|c| !matches!(c.kind(), "comment" | "type_descriptor")),
            );
            true
        }
        _ => false,
    }
}

/// `node` with every parenthesis and `__extension__` peeled (comments skipped).
pub(super) fn peel(mut node: Node<'_>) -> Node<'_> {
    while matches!(
        node.kind(),
        "parenthesized_expression" | "extension_expression"
    ) {
        match operand(node) {
            Some(c) => node = c,
            None => break,
        }
    }
    node
}

/// The root identifiers of an address-of or member chain, C rules: the wrappers above, `.`/`->`
/// access and subscripts. In C every other expression is an rvalue, so an unrecognized shape
/// hands out no lvalue and contributes nothing (C++ has reference-returning shapes and uses
/// `cpp_prune::escape_roots`, whose default escapes every identifier).
pub(super) fn chain_roots<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "identifier" {
        out.push(node);
        return;
    }
    let mut kids = Vec::new();
    if wrapper_operands(node, &mut kids) {
        for k in kids {
            chain_roots(k, out);
        }
    } else if matches!(node.kind(), "field_expression" | "subscript_expression") {
        if let Some(a) = node.child_by_field_name("argument") {
            chain_roots(a, out);
        }
    }
}
