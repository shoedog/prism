//! JS/TS/TSX facts for rules U and D (#630 slice 5). Removal-only, lexical, named owners only.
//!
//! Synthetic `<anon@..>` owners carry exact read spans and stay off. Hazards:
//! * `with` and a direct `eval` in the owner switch U and D off (object-backed or dynamic names).
//! * `arguments` anywhere in the owner switches D off (sloppy mode aliases formals to it).
//! * Hoisted function declarations (Annex B, same-name), `var` redeclarations and labels need no
//!   switch: D only drops defs that a dominating write provably overwrote, so an extra
//!   unmodelled writer before the dominating write cannot resurrect them, and a label only
//!   targets enclosing statements (`break L` leaves forward, `continue L` re-enters at a top).
//! * Generators and `async`: `yield`/`await` resume in place, so a region keeps one entry.
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Scopes that run later or per instance: a def or use inside one is never proven.
pub(super) const EXTRA_BOUNDARIES: [&str; 2] = ["public_field_definition", "field_definition"];

/// Owner-level switches: `(enabled, d_off)`.
pub(super) fn owner_switches(parsed: &ParsedFile, owner: Node<'_>) -> (bool, bool) {
    if owner.has_error() || parsed.language.function_name(&owner).is_none() {
        return (false, false);
    }
    let mut d_off = false;
    let mut stack = vec![owner];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "with_statement" => return (false, false),
            "call_expression" => {
                if let Some(f) = node.child_by_field_name("function") {
                    if f.kind() == "identifier" && parsed.node_text(&f) == "eval" {
                        return (false, false);
                    }
                }
            }
            "identifier" if parsed.node_text(&node) == "arguments" => d_off = true,
            _ => {}
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    (true, d_off)
}

/// Single-entry region: a block, or one `switch` case (fallthrough enters at its top).
/// A namespace body runs as its own scope and is not one.
pub(super) fn is_region(node: Node<'_>) -> bool {
    match node.kind() {
        "statement_block" => !node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "internal_module" | "module")),
        "switch_case" | "switch_default" => true,
        _ => false,
    }
}

pub(super) fn pure_path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    match node.kind() {
        "identifier" => Some(AccessPath::simple(parsed.node_text(&node))),
        "this" => Some(AccessPath::simple("this")),
        "member_expression" => {
            if node.child_by_field_name("optional_chain").is_some() {
                return None;
            }
            let mut path = pure_path(parsed, node.child_by_field_name("object")?)?;
            let field = node.child_by_field_name("property")?;
            if field.kind() != "property_identifier" {
                return None;
            }
            path.fields.push(parsed.node_text(&field).to_owned());
            Some(path)
        }
        _ => None,
    }
}

/// The write statement of an lvalue occurrence and whether it is a plain `=`/declaration.
/// A node that is a direct child of a region is an unconditional direct write statement
/// (rule D): an `expression_statement` holding the assignment, or a declaration with exactly
/// one initialised declarator. Anything else is returned as its own inner node, which is
/// never a region child, so it is a write for rule U only.
pub(super) fn classify_write<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<(Node<'a>, bool)> {
    let parent = node.parent()?;
    match parent.kind() {
        "assignment_expression" | "augmented_assignment_expression" => {
            if parent.child_by_field_name("left")? != node {
                return None;
            }
            let augmented = parent.kind() == "augmented_assignment_expression";
            let op = parent
                .child_by_field_name("operator")
                .map(|o| parsed.node_text(&o).to_owned())
                .unwrap_or_default();
            // Logical assignment is conditional: never a dominating write.
            if augmented && matches!(op.as_str(), "||=" | "&&=" | "??=") {
                return Some((parent, false));
            }
            let statement = parent
                .parent()
                .filter(|p| p.kind() == "expression_statement")
                .unwrap_or(parent);
            Some((statement, !augmented))
        }
        "variable_declarator" => {
            if parent.child_by_field_name("name")? != node {
                return None;
            }
            let decl = parent
                .parent()
                .filter(|d| matches!(d.kind(), "lexical_declaration" | "variable_declaration"));
            let mut cursor = decl?.walk();
            let declarators = decl?
                .named_children(&mut cursor)
                .filter(|c| c.kind() == "variable_declarator")
                .count();
            let statement = match decl {
                Some(d) if declarators == 1 && parent.child_by_field_name("value").is_some() => d,
                _ => parent,
            };
            Some((statement, true))
        }
        _ => None,
    }
}

/// Bases whose member paths D proves: `this` (never rebound inside the owner, nested callables
/// are boundaries) and names bound exactly once in the owner, as one of its own formals or an
/// initialised declarator, and never written anywhere in it (nested callables included).
pub(super) fn stable_bases(parsed: &ParsedFile, owner: Node<'_>) -> BTreeSet<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut candidates = BTreeSet::new();
    let mut stack = vec![owner];
    while let Some(node) = stack.pop() {
        if matches!(
            node.kind(),
            "identifier" | "shorthand_property_identifier_pattern"
        ) && is_binding_or_write(node)
        {
            let name = parsed.node_text(&node).to_owned();
            if is_own_binding(owner, node) {
                candidates.insert(name.clone());
            }
            *counts.entry(name).or_default() += 1;
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    let mut stable: BTreeSet<String> = candidates
        .into_iter()
        .filter(|n| counts.get(n) == Some(&1))
        .collect();
    stable.insert("this".to_owned());
    stable
}

/// A formal of the owner (plain or with a default), or an initialised plain declarator.
fn is_own_binding(owner: Node<'_>, ident: Node<'_>) -> bool {
    let Some(parent) = ident.parent() else {
        return false;
    };
    match parent.kind() {
        "variable_declarator" => parent.child_by_field_name("value").is_some(),
        "required_parameter" | "optional_parameter" | "formal_parameters" => {
            owner_formals(owner, parent)
        }
        "assignment_pattern" => parent
            .parent()
            .is_some_and(|g| owner_formals(owner, g) || g.kind() == "formal_parameters"),
        _ => false,
    }
}

fn owner_formals(owner: Node<'_>, node: Node<'_>) -> bool {
    let params = if node.kind() == "formal_parameters" {
        Some(node)
    } else {
        node.parent()
    };
    params.is_some_and(|p| {
        p.kind() == "formal_parameters" && p.parent().is_some_and(|o| o.id() == owner.id())
    })
}

/// Conservative: any position that can bind or write the identifier counts.
fn is_binding_or_write(ident: Node<'_>) -> bool {
    let mut node = ident;
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "array_pattern"
            | "object_pattern"
            | "rest_pattern"
            | "parenthesized_expression"
            | "as_expression"
            | "non_null_expression"
            | "satisfies_expression"
            | "type_assertion"
            | "object_assignment_pattern" => {}
            "pair_pattern" => {
                if parent.child_by_field_name("value") != Some(node) {
                    return false;
                }
            }
            "assignment_pattern" => {
                if parent.child_by_field_name("left") != Some(node) {
                    return false;
                }
            }
            "assignment_expression" | "augmented_assignment_expression" => {
                return parent.child_by_field_name("left") == Some(node)
            }
            "update_expression" => return true,
            "for_in_statement" => return parent.child_by_field_name("left") == Some(node),
            "variable_declarator" => return parent.child_by_field_name("name") == Some(node),
            "required_parameter" | "optional_parameter" => {
                return parent.child_by_field_name("pattern") == Some(node)
            }
            "formal_parameters" | "catch_clause" | "import_specifier" | "import_clause"
            | "namespace_import" | "rest_parameter" => return true,
            "function_declaration"
            | "generator_function_declaration"
            | "function_expression"
            | "generator_function"
            | "class_declaration"
            | "class"
            | "enum_declaration"
            | "internal_module"
            | "arrow_function" => {
                return parent.child_by_field_name("name") == Some(node)
                    || parent.child_by_field_name("parameter") == Some(node)
            }
            _ => return false,
        }
        node = parent;
    }
    false
}
