//! Lexical binding lookup for Go sink rules (#622 S1): what an identifier names
//! (a constant, a variable, a parameter with its type, a function) from the
//! nearest enclosing declaration, so a local `format := input` shadows a file
//! `const format`, and a local variable shadows an imported package name.

use super::callee::Src;
use tree_sitter::Node;

pub(super) enum GoBind<'t> {
    Unresolved,
    Const,
    /// `name := init` / `var name T`.
    Var {
        init: Option<Node<'t>>,
        /// The declared type of `var name T`.
        ty: Option<Node<'t>>,
    },
    /// A parameter or receiver, with its declared type.
    Param {
        ty: Option<Node<'t>>,
    },
    /// A function declared at package level.
    Func,
}

pub(super) fn resolve<'t>(p: Src<'_>, from: Node<'t>, name: &str) -> GoBind<'t> {
    let use_at = from.start_byte();
    let mut cur = from.parent();
    while let Some(scope) = cur {
        let found = match scope.kind() {
            "function_declaration" | "method_declaration" | "func_literal" => {
                in_params(p, scope, name)
            }
            "block" | "statement_list" => in_statements(p, scope, name, Some(use_at)),
            "source_file" => in_statements(p, scope, name, None),
            // `if v := x; ..`, `switch v := x; ..`, `for v := ..; ..`, `for k, v := range ..`:
            // the init statement or range clause binds for the whole statement.
            "if_statement"
            | "for_statement"
            | "expression_switch_statement"
            | "type_switch_statement" => in_init(p, scope, name),
            _ => None,
        };
        if let Some(b) = found {
            return b;
        }
        cur = scope.parent();
    }
    GoBind::Unresolved
}

fn in_init<'t>(p: Src<'_>, stmt: Node<'t>, name: &str) -> Option<GoBind<'t>> {
    let mut cursor = stmt.walk();
    let children: Vec<Node<'t>> = stmt.named_children(&mut cursor).collect();
    for child in children {
        let (left, right) = match child.kind() {
            // `if`/`switch` initializer is a field; `for` holds the clause as a child.
            "short_var_declaration" | "range_clause" => (
                child.child_by_field_name("left"),
                child.child_by_field_name("right"),
            ),
            "for_clause" => {
                let init = child.child_by_field_name("initializer")?;
                (
                    init.child_by_field_name("left"),
                    init.child_by_field_name("right"),
                )
            }
            _ => continue,
        };
        let Some(left) = left else { continue };
        let mut ids = left.walk();
        for (i, l) in left.named_children(&mut ids).enumerate() {
            if l.kind() == "identifier" && p.node_text(&l) == name {
                // Range clauses bind elements, not the right operand itself.
                let init = if child.kind() == "range_clause" {
                    None
                } else {
                    right.and_then(|r| r.named_child(i))
                };
                return Some(GoBind::Var { init, ty: None });
            }
        }
    }
    if let Some(init) = stmt.child_by_field_name("initializer") {
        if init.kind() == "short_var_declaration" {
            let left = init.child_by_field_name("left")?;
            let right = init.child_by_field_name("right");
            let mut ids = left.walk();
            for (i, l) in left.named_children(&mut ids).enumerate() {
                if l.kind() == "identifier" && p.node_text(&l) == name {
                    return Some(GoBind::Var {
                        init: right.and_then(|r| r.named_child(i)),
                        ty: None,
                    });
                }
            }
        }
    }
    None
}

fn in_params<'t>(p: Src<'_>, func: Node<'t>, name: &str) -> Option<GoBind<'t>> {
    for field in ["receiver", "parameters"] {
        let Some(list) = func.child_by_field_name(field) else {
            continue;
        };
        let mut cursor = list.walk();
        for decl in list.named_children(&mut cursor) {
            let mut names = decl.walk();
            let declares = decl
                .children_by_field_name("name", &mut names)
                .any(|n| p.node_text(&n) == name);
            if declares {
                return Some(GoBind::Param {
                    ty: decl.child_by_field_name("type"),
                });
            }
        }
    }
    None
}

/// Declarations among the direct statements of a block or file. In a block only
/// declarations that end before the use count.
fn in_statements<'t>(
    p: Src<'_>,
    scope: Node<'t>,
    name: &str,
    use_at: Option<usize>,
) -> Option<GoBind<'t>> {
    let mut cursor = scope.walk();
    for stmt in scope.named_children(&mut cursor) {
        if use_at.is_some_and(|at| stmt.end_byte() > at) {
            continue;
        }
        match stmt.kind() {
            "statement_list" => {
                if let Some(b) = in_statements(p, stmt, name, use_at) {
                    return Some(b);
                }
            }
            "short_var_declaration" => {
                let left = stmt.child_by_field_name("left")?;
                let right = stmt.child_by_field_name("right");
                let mut ids = left.walk();
                for (i, l) in left.named_children(&mut ids).enumerate() {
                    if l.kind() == "identifier" && p.node_text(&l) == name {
                        return Some(GoBind::Var {
                            init: right.and_then(|r| r.named_child(i)),
                            ty: None,
                        });
                    }
                }
            }
            "var_declaration" | "const_declaration" => {
                let is_const = stmt.kind() == "const_declaration";
                if let Some(b) = in_specs(p, stmt, name, is_const) {
                    return Some(b);
                }
            }
            "function_declaration" if use_at.is_none() => {
                if stmt
                    .child_by_field_name("name")
                    .is_some_and(|n| p.node_text(&n) == name)
                {
                    return Some(GoBind::Func);
                }
            }
            _ => {}
        }
    }
    None
}

fn in_specs<'t>(p: Src<'_>, decl: Node<'t>, name: &str, is_const: bool) -> Option<GoBind<'t>> {
    let mut cursor = decl.walk();
    for spec in decl.named_children(&mut cursor) {
        let specs: Vec<Node<'t>> = if spec.kind() == "var_spec_list" {
            let mut inner = spec.walk();
            spec.named_children(&mut inner).collect()
        } else {
            vec![spec]
        };
        for s in specs {
            let mut names = s.walk();
            let declares = s
                .children_by_field_name("name", &mut names)
                .any(|n| p.node_text(&n) == name);
            if declares {
                return Some(if is_const {
                    GoBind::Const
                } else {
                    GoBind::Var {
                        init: s
                            .child_by_field_name("value")
                            .and_then(|v| v.named_child(0)),
                        ty: s.child_by_field_name("type"),
                    }
                });
            }
        }
    }
    None
}
