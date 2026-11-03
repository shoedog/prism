//! Lexical binding lookup for Python sink rules (#622 S1): what a receiver or a
//! bare callee is (a collection, a file object, a function defined in this file),
//! taken from the nearest enclosing function or module scope.

use super::callee::Src;
use tree_sitter::Node;

pub(super) enum PyBind<'t> {
    Unresolved,
    /// A parameter, with its annotation when it has one.
    Param {
        annotation: Option<Node<'t>>,
    },
    /// `name = value`.
    Assigned(Node<'t>),
    /// `with value as name`.
    WithAs(Node<'t>),
    /// `def name` / `class name`.
    Def,
    Other,
}

pub(super) fn resolve<'t>(p: Src<'_>, from: Node<'t>, name: &str) -> PyBind<'t> {
    let mut cur = from.parent();
    while let Some(scope) = cur {
        match scope.kind() {
            "function_definition" => {
                if let Some(b) = in_params(p, scope, name) {
                    return b;
                }
                if let Some(body) = scope.child_by_field_name("body") {
                    if let Some(b) = scan(p, body, name) {
                        return b;
                    }
                }
            }
            "module" => {
                if let Some(b) = scan(p, scope, name) {
                    return b;
                }
            }
            _ => {}
        }
        cur = scope.parent();
    }
    PyBind::Unresolved
}

/// True when `name` is bound more than `allowed` times in the file. Every binding form counts
/// toward one total: plain, annotated and tuple/list-unpacking assignment targets, `with .. as
/// name`, `for` targets, walrus and augmented assignment. A fact proved from the first binding
/// alone no longer holds once the total passes `allowed`, so the caller treats the name as
/// unknown. Conservative by design: the count is file-wide, by spelling.
pub(super) fn is_rebound(p: Src<'_>, from: Node<'_>, name: &str, allowed: usize) -> bool {
    let mut root = from;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let mut count = 0usize;
    super::callee::walk(root, &mut |n| {
        let target = match n.kind() {
            "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
                n.child_by_field_name("left")
            }
            "named_expression" => n.child_by_field_name("name"),
            "as_pattern_target" => n.named_child(0),
            _ => None,
        };
        if target.is_some_and(|t| binds_name(p, t, name)) {
            count += 1;
        }
    });
    count > allowed
}

/// True when the assignment target `t` binds `name` (directly or inside an unpacking pattern;
/// attribute and subscript targets bind nothing).
fn binds_name(p: Src<'_>, t: Node<'_>, name: &str) -> bool {
    match t.kind() {
        "identifier" => p.node_text(&t) == name,
        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "list_splat_pattern"
        | "parenthesized_expression"
        | "tuple"
        | "list" => {
            let mut cursor = t.walk();
            let mut kids = t.named_children(&mut cursor);
            kids.any(|c| binds_name(p, c, name))
        }
        _ => false,
    }
}

fn in_params<'t>(p: Src<'_>, func: Node<'t>, name: &str) -> Option<PyBind<'t>> {
    let params = func.child_by_field_name("parameters")?;
    let mut cursor = params.walk();
    for param in params.named_children(&mut cursor) {
        match param.kind() {
            "identifier" if p.node_text(&param) == name => {
                return Some(PyBind::Param { annotation: None })
            }
            "typed_parameter" => {
                let ident = param.named_child(0).filter(|n| n.kind() == "identifier");
                if ident.is_some_and(|n| p.node_text(&n) == name) {
                    return Some(PyBind::Param {
                        annotation: param.child_by_field_name("type"),
                    });
                }
            }
            "default_parameter" | "typed_default_parameter" => {
                let n = param.child_by_field_name("name");
                if n.is_some_and(|n| p.node_text(&n) == name) {
                    return Some(PyBind::Param {
                        annotation: param.child_by_field_name("type"),
                    });
                }
            }
            _ => {}
        }
    }
    None
}

/// First binding of `name` in source order inside `node`, not entering nested scopes.
fn scan<'t>(p: Src<'_>, node: Node<'t>, name: &str) -> Option<PyBind<'t>> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "function_definition" | "class_definition" => {
                if child
                    .child_by_field_name("name")
                    .is_some_and(|n| p.node_text(&n) == name)
                {
                    return Some(PyBind::Def);
                }
                continue;
            }
            "lambda" => continue,
            "assignment" => {
                let left = child.child_by_field_name("left");
                if left.is_some_and(|l| l.kind() == "identifier" && p.node_text(&l) == name) {
                    return Some(match child.child_by_field_name("right") {
                        Some(v) => PyBind::Assigned(v),
                        None => PyBind::Other,
                    });
                }
            }
            "with_item" => {
                if let Some(b) = with_alias(p, child, name) {
                    return Some(b);
                }
            }
            "for_statement" => {
                let left = child.child_by_field_name("left");
                if left.is_some_and(|l| l.kind() == "identifier" && p.node_text(&l) == name) {
                    return Some(PyBind::Other);
                }
            }
            _ => {}
        }
        if let Some(b) = scan(p, child, name) {
            return Some(b);
        }
    }
    None
}

fn with_alias<'t>(p: Src<'_>, item: Node<'t>, name: &str) -> Option<PyBind<'t>> {
    let value = item.child_by_field_name("value")?;
    if value.kind() != "as_pattern" {
        return None;
    }
    let mut cursor = value.walk();
    let aliased = value
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "as_pattern_target")
        .any(|t| {
            t.named_child(0)
                .is_some_and(|i| i.kind() == "identifier" && p.node_text(&i) == name)
        });
    if aliased {
        Some(PyBind::WithAs(value.named_child(0)?))
    } else {
        None
    }
}
