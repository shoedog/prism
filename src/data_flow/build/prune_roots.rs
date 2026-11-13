//! Operand and root finding shared by the C rules (`c_prune.rs`) and the C++ additions
//! (`cpp_prune.rs`), #630 slice 6c fold 5. One definition of "the real operand child" and of
//! which wrappers hand their operand over unchanged, so no finder re-derives it with
//! `named_child(0)` (which is a `comment` when one precedes the operand:
//! `&(/*c*/ x)` was invisible to every root finder, in `.c` files too). Declarator peelers use
//! `operand` as well; a unit test (`no_first_named_child_reads`) fails on a first-named-child
//! read in the prune modules.
use crate::ast::ParsedFile;
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
/// access, subscripts, and the GNU builtins. Standard C has no other lvalue-producing
/// expression, but GNU C does: `__builtin_choose_expr(c, a, b)` yields the lvalue `a` or `b`
/// unchanged, so it hands out the roots of both; any other `__builtin_*` call in this position
/// escapes every identifier of its arguments (a builtin may return its argument's lvalue).
/// Every other unrecognized shape contributes nothing. (C++ has reference-returning shapes and
/// uses `cpp_prune::escape_roots`, whose default escapes every identifier.)
pub(super) fn chain_roots<'a>(parsed: &ParsedFile, node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "identifier" {
        out.push(node);
        return;
    }
    let mut kids = Vec::new();
    if wrapper_operands(node, &mut kids) {
        for k in kids {
            chain_roots(parsed, k, out);
        }
    } else if matches!(node.kind(), "field_expression" | "subscript_expression") {
        if let Some(a) = node.child_by_field_name("argument") {
            chain_roots(parsed, a, out);
        }
    } else if node.kind() == "call_expression" {
        builtin_roots(parsed, node, out);
    }
}

fn builtin_roots<'a>(parsed: &ParsedFile, call: Node<'a>, out: &mut Vec<Node<'a>>) {
    let (Some(f), Some(args)) = (
        call.child_by_field_name("function"),
        call.child_by_field_name("arguments"),
    ) else {
        return;
    };
    let callee = peel(f);
    if callee.kind() != "identifier" || !parsed.node_text(&callee).starts_with("__builtin_") {
        return;
    }
    let mut cursor = args.walk();
    let kids = args
        .named_children(&mut cursor)
        .filter(|c| c.kind() != "comment");
    if parsed.node_text(&callee) == "__builtin_choose_expr" {
        for k in kids.skip(1) {
            chain_roots(parsed, k, out);
        }
    } else {
        for k in kids {
            all_identifiers(k, out);
        }
    }
}

fn all_identifiers<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "identifier" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for c in node.named_children(&mut cursor) {
        all_identifiers(c, out);
    }
}

/// The `parameter_list` of a function declarator chain, or None when the walk cannot positively
/// reach the function's name leaf and its own list (see `descend_params`): the parameters then
/// bind as unknown and every name stays unresolved.
pub(super) fn function_params(declarator: Node<'_>) -> Option<Node<'_>> {
    match descend_params(declarator) {
        Params::Found(p) => Some(p),
        Params::Leaf | Params::Unknown => None,
    }
}

enum Params<'a> {
    Found(Node<'a>),
    /// The walk reached a name leaf (an identifier, a qualified or operator name).
    Leaf,
    /// Anything else: an unknown declarator kind or a wrapper with no operand.
    Unknown,
}

/// `int (*f(int a))(int b)`: the innermost function declarator, the one that reaches the name
/// leaf, holds the function's own parameters; outer ones belong to the returned type, and are
/// never used when the inner chain cannot be walked (never a fallback to an outer list).
fn descend_params(declarator: Node<'_>) -> Params<'_> {
    match declarator.kind() {
        "function_declarator" => {
            let Some(inner) = declarator.child_by_field_name("declarator") else {
                return Params::Unknown;
            };
            match descend_params(inner) {
                Params::Found(p) => Params::Found(p),
                Params::Leaf => match declarator.child_by_field_name("parameters") {
                    Some(p) => Params::Found(p),
                    None => Params::Unknown,
                },
                Params::Unknown => Params::Unknown,
            }
        }
        "pointer_declarator"
        | "parenthesized_declarator"
        | "attributed_declarator"
        | "reference_declarator"
        | "array_declarator" => match declarator
            .child_by_field_name("declarator")
            .or_else(|| operand(declarator))
        {
            Some(n) => descend_params(n),
            None => Params::Unknown,
        },
        "identifier"
        | "field_identifier"
        | "qualified_identifier"
        | "destructor_name"
        | "operator_name"
        | "operator_cast"
        | "template_function"
        | "template_method" => Params::Leaf,
        _ => Params::Unknown,
    }
}

#[cfg(test)]
mod tests {
    /// No prune module reads a first named child directly: a `comment` extra can be that child
    /// (`int (/*c*/ x)`, `&(/*c*/ x)`), so every peeler goes through `operand`.
    #[test]
    fn no_first_named_child_reads() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/data_flow/build");
        for f in [
            "c_prune.rs",
            "cpp_prune.rs",
            "cpp_prune_operand.rs",
            "cpp_prune_names.rs",
            "prune_roots.rs",
        ] {
            let text = std::fs::read_to_string(dir.join(f)).unwrap();
            for (n, line) in text.lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("//") {
                    continue;
                }
                assert!(
                    !line.contains(concat!("named_child", "(0)")),
                    "{f}:{} reads a first named child (use prune_roots::operand)",
                    n + 1
                );
            }
        }
    }
}
