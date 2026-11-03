//! Lexical binding lookup for JS/TS/TSX sink rules (#622 S1).
//!
//! The sink rules ask what a receiver or a bare callee *is* (a `child_process`
//! module, a regex, a function declared in this file). That is answered from the
//! nearest enclosing declaration, so a parameter or inner `const` that shadows a
//! file-level binding wins, instead of a file-wide name table.

use super::callee::{peel, Src};
use tree_sitter::Node;

/// What the nearest declaration of a name is.
pub(super) enum Binding<'t> {
    /// No declaration in this file (a global, or an import the grammar hides).
    Unresolved,
    /// A function parameter, with its TS type annotation when it has one.
    Param { annotation: Option<Node<'t>> },
    /// `let/const/var name = init`.
    Var { init: Option<Node<'t>> },
    /// `const { key: name } = init` (`key` is the property name read).
    Destructured {
        init: Node<'t>,
        key: Option<Node<'t>>,
    },
    /// A function declaration, or a variable initialised with a function.
    Function,
    /// `import name from 'module'` / `import * as name` / `import { imported as name }`.
    Import {
        module: Node<'t>,
        imported: Option<Node<'t>>,
    },
    /// Anything else (class, catch parameter, nested pattern).
    Other,
}

pub(super) fn is_function_kind(kind: &str) -> bool {
    matches!(
        kind,
        "arrow_function" | "function_expression" | "function" | "generator_function"
    )
}

/// Resolve `name` as seen from `from`, innermost scope first.
pub(super) fn resolve<'t>(p: Src<'_>, from: Node<'t>, name: &str) -> Binding<'t> {
    let mut cur = from.parent();
    while let Some(scope) = cur {
        if let Some(b) = declared_in(p, scope, name) {
            return b;
        }
        cur = scope.parent();
    }
    Binding::Unresolved
}

fn declared_in<'t>(p: Src<'_>, scope: Node<'t>, name: &str) -> Option<Binding<'t>> {
    match scope.kind() {
        "function_declaration"
        | "function_expression"
        | "function"
        | "arrow_function"
        | "method_definition"
        | "generator_function"
        | "generator_function_declaration" => in_params(p, scope, name),
        "statement_block" | "program" => in_block(p, scope, name),
        // `catch (q)` binds the error object for the whole clause.
        "catch_clause" => scope
            .child_by_field_name("parameter")
            .filter(|param| contains_ident(p, *param, name))
            .map(|_| Binding::Other),
        _ => None,
    }
}

/// True when `name` is assigned anywhere in the file after its declaration (`name = v`,
/// `name += v`, `name ||= v`, `[name] = ..`, `for (name of ..)`): a fact proved from the declaration alone
/// no longer holds, so the caller treats the name as unknown. Conservative by design
/// (any same-named write in the file counts).
pub(super) fn is_reassigned(p: Src<'_>, from: Node<'_>, name: &str) -> bool {
    let mut root = from;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let mut found = false;
    super::callee::walk(root, &mut |n| {
        if matches!(
            n.kind(),
            "assignment_expression" | "augmented_assignment_expression" | "for_in_statement"
        ) {
            if let Some(left) = n.child_by_field_name("left") {
                if (left.kind() == "identifier" && p.node_text(&left) == name)
                    || (left.kind() != "identifier"
                        && left.kind() != "member_expression"
                        && contains_ident(p, left, name))
                {
                    found = true;
                }
            }
        }
    });
    found
}

fn contains_ident(p: Src<'_>, node: Node<'_>, name: &str) -> bool {
    let mut found = false;
    super::callee::walk(node, &mut |n| {
        if matches!(
            n.kind(),
            "identifier" | "shorthand_property_identifier_pattern"
        ) && p.node_text(&n) == name
        {
            found = true;
        }
    });
    found
}

fn in_params<'t>(p: Src<'_>, func: Node<'t>, name: &str) -> Option<Binding<'t>> {
    if let Some(single) = func.child_by_field_name("parameter") {
        if single.kind() == "identifier" && p.node_text(&single) == name {
            return Some(Binding::Param { annotation: None });
        }
    }
    let params = func.child_by_field_name("parameters")?;
    let mut cursor = params.walk();
    for param in params.named_children(&mut cursor) {
        match param.kind() {
            "identifier" if p.node_text(&param) == name => {
                return Some(Binding::Param { annotation: None })
            }
            "required_parameter" | "optional_parameter" => {
                let pattern = param.child_by_field_name("pattern");
                match pattern {
                    Some(pat) if pat.kind() == "identifier" => {
                        if p.node_text(&pat) == name {
                            return Some(Binding::Param {
                                annotation: param.child_by_field_name("type"),
                            });
                        }
                    }
                    Some(pat) if contains_ident(p, pat, name) => return Some(Binding::Other),
                    _ => {}
                }
            }
            "assignment_pattern" => {
                if let Some(left) = param.child_by_field_name("left") {
                    if left.kind() == "identifier" && p.node_text(&left) == name {
                        return Some(Binding::Param { annotation: None });
                    }
                }
            }
            "rest_pattern" | "object_pattern" | "array_pattern"
                if contains_ident(p, param, name) =>
            {
                return Some(Binding::Other)
            }
            _ => {}
        }
    }
    None
}

fn in_block<'t>(p: Src<'_>, block: Node<'t>, name: &str) -> Option<Binding<'t>> {
    let mut cursor = block.walk();
    for stmt in block.named_children(&mut cursor) {
        let stmt = if stmt.kind() == "export_statement" {
            stmt.child_by_field_name("declaration").unwrap_or(stmt)
        } else {
            stmt
        };
        match stmt.kind() {
            "lexical_declaration" | "variable_declaration" => {
                let mut inner = stmt.walk();
                for d in stmt.named_children(&mut inner) {
                    if d.kind() == "variable_declarator" {
                        if let Some(b) = in_declarator(p, d, name) {
                            return Some(b);
                        }
                    }
                }
            }
            "function_declaration" | "generator_function_declaration" => {
                if stmt
                    .child_by_field_name("name")
                    .is_some_and(|n| p.node_text(&n) == name)
                {
                    return Some(Binding::Function);
                }
            }
            "class_declaration" => {
                if stmt
                    .child_by_field_name("name")
                    .is_some_and(|n| p.node_text(&n) == name)
                {
                    return Some(Binding::Other);
                }
            }
            "import_statement" => {
                if let Some(b) = in_import(p, stmt, name) {
                    return Some(b);
                }
            }
            _ => {}
        }
    }
    None
}

fn in_declarator<'t>(p: Src<'_>, decl: Node<'t>, name: &str) -> Option<Binding<'t>> {
    let target = decl.child_by_field_name("name")?;
    let value = decl.child_by_field_name("value");
    match target.kind() {
        "identifier" if p.node_text(&target) == name => Some(match value {
            Some(v) if is_function_kind(peel(v).kind()) => Binding::Function,
            init => Binding::Var { init },
        }),
        "object_pattern" if contains_ident(p, target, name) => {
            let mut cursor = target.walk();
            for child in target.named_children(&mut cursor) {
                let key = match child.kind() {
                    "shorthand_property_identifier_pattern" if p.node_text(&child) == name => {
                        Some(child)
                    }
                    "pair_pattern" => {
                        let v = child.child_by_field_name("value");
                        if v.is_some_and(|v| v.kind() == "identifier" && p.node_text(&v) == name) {
                            child.child_by_field_name("key")
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };
                return Some(match value {
                    Some(init) => Binding::Destructured { init, key },
                    None => Binding::Other,
                });
            }
            Some(Binding::Other)
        }
        "array_pattern" if contains_ident(p, target, name) => Some(Binding::Other),
        _ => None,
    }
}

fn in_import<'t>(p: Src<'_>, stmt: Node<'t>, name: &str) -> Option<Binding<'t>> {
    let mut cursor = stmt.walk();
    let children: Vec<Node<'t>> = stmt.named_children(&mut cursor).collect();
    let source = stmt
        .child_by_field_name("source")
        .or_else(|| children.iter().find(|c| c.kind() == "string").copied());
    for child in &children {
        match child.kind() {
            // TS `import cp = require('x')`
            "import_require_clause" => {
                let id = child.named_child(0)?;
                let module = child
                    .child_by_field_name("source")
                    .or_else(|| child.named_child(1))?;
                if p.node_text(&id) == name {
                    return Some(Binding::Import {
                        module,
                        imported: None,
                    });
                }
            }
            "import_clause" => {
                let module = source?;
                let mut inner = child.walk();
                for part in child.named_children(&mut inner) {
                    match part.kind() {
                        "identifier" if p.node_text(&part) == name => {
                            return Some(Binding::Import {
                                module,
                                imported: None,
                            })
                        }
                        "namespace_import" => {
                            if part.named_child(0).is_some_and(|n| p.node_text(&n) == name) {
                                return Some(Binding::Import {
                                    module,
                                    imported: None,
                                });
                            }
                        }
                        "named_imports" => {
                            let mut specs = part.walk();
                            for spec in part.named_children(&mut specs) {
                                let orig = spec.child_by_field_name("name");
                                let local = spec.child_by_field_name("alias").or(orig);
                                if local.is_some_and(|l| p.node_text(&l) == name) {
                                    return Some(Binding::Import {
                                        module,
                                        imported: orig,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    None
}
