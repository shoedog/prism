//! JS/TS/TSX facts for rules U and D (#630 slice 5). Removal-only, lexical, named owners only.
//!
//! Synthetic `<anon@..>` owners carry exact read spans and stay off. Policy:
//! * Owner switch (`owner_enabled`): U and D are BOTH off for an owner with a direct eval, possibly
//!   mapped `arguments` (sloppy, simple formals, of the owner or of an enclosing non-arrow
//!   callable whose `arguments` the owner can reach), a `with` statement, an enclosing `with` body, an
//!   escaped identifier spelling or a parse error, each decided by the repo's existing analysis.
//!   Mapped `arguments` is a read hazard for U (`a = arguments[0]`) and a write hazard for D on a
//!   stable base, so neither rule runs. Strict code, modules and non-simple parameter lists that
//!   mention `arguments` keep both rules.
//! * A dominating write (rule D) is a direct child of a block or `switch` case that is an
//!   `expression_statement` or a declaration; a `case x = 5:` label assignment is not one (the
//!   label is skipped on fallthrough), nor is a logical or compound assignment inside any other
//!   expression.
//! * Field paths: D only for a single-field path `base.f` on a stable base (a formal of the owner,
//!   or an initialised declarator whose scope is the owner itself, bound once and never written;
//!   or `this`). Paths of two or more fields keep: an intermediate object is a reference the
//!   owner can swap and restore (`o.a = {}; ..; o.a = old`).
//! * Out of model, as for Exact: getters, setters, Proxies and runtime mutation.
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

/// Owner-level switch: whether U and D are enabled for this owner. Both rules go off (the pairs
/// keep) for an owner the repo's own analyses mark as hazardous, no approximation of ours:
/// * a parse error, or an anonymous owner;
/// * a direct eval (`js_ts_direct_eval_anywhere`: parentheses, TS wrappers, escaped spelling);
/// * mapped `arguments` reachable from the owner (`refusals::mapped_arguments_reachable`: the
///   owner or an enclosing non-arrow callable with formals whose `arguments` is mapped, reached
///   directly, through nested arrows, an alias or a computed key, escaped spellings decoded; an
///   arrow owner is judged by the callables around it), or by the owner's own
///   `js_ts_mapped_arguments_possible` (a program-level arrow that spells `arguments`, which a
///   CommonJS module wrapper maps); strict code and non-simple parameter lists are unmapped;
/// * a `with` statement in the owner, or the owner inside a `with` body (`inside_with_body`);
/// * an escaped identifier spelling anywhere in the owner (S1b-1 census `precondition`).
pub(super) fn owner_enabled(parsed: &ParsedFile, owner: Node<'_>) -> bool {
    if owner.has_error() || parsed.language.function_name(&owner).is_none() {
        return false;
    }
    let tables = crate::js_write_census::tables::Tables::shared();
    if parsed.js_ts_direct_eval_anywhere(&owner)
        || parsed.js_ts_mapped_arguments_possible(&owner)
        || crate::js_write_census::refusals::mapped_arguments_reachable(parsed, tables, owner)
        || crate::js_write_census::refusals::inside_with_body(owner)
        || crate::js_write_census::walk::precondition(tables, owner, parsed.source.as_bytes())
            .is_some()
    {
        return false;
    }
    let mut stack = vec![owner];
    while let Some(node) = stack.pop() {
        if node.kind() == "with_statement" {
            return false;
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    true
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
                return Some((node, false));
            }
            // An assignment that is not a whole statement (a `case x = 5:` label is a direct
            // child of its `switch_case`) is returned as the identifier, never a region child.
            let statement = parent
                .parent()
                .filter(|p| p.kind() == "expression_statement")
                .unwrap_or(node);
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
            if is_own_binding(parsed, owner, node) {
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

/// A formal of the owner (plain or with a default), or an initialised plain declarator whose
/// scope is the owner itself.
fn is_own_binding(parsed: &ParsedFile, owner: Node<'_>, ident: Node<'_>) -> bool {
    let Some(parent) = ident.parent() else {
        return false;
    };
    match parent.kind() {
        "variable_declarator" => {
            parent.child_by_field_name("value").is_some()
                && declarator_in_owner_scope(parsed, owner, parent)
        }
        "required_parameter" | "optional_parameter" | "formal_parameters" => {
            owner_formals(owner, parent)
        }
        "assignment_pattern" => parent.parent().is_some_and(|g| owner_formals(owner, g)),
        _ => false,
    }
}

/// The declarator binds in the owner's own scope: no callable or class boundary between it and the
/// owner, and a `let`/`const` sits directly in the owner's body block (a block-scoped one in a
/// nested block, loop head or `switch` is a different binding scope; `var` is function-scoped).
fn declarator_in_owner_scope(parsed: &ParsedFile, owner: Node<'_>, decl: Node<'_>) -> bool {
    let mut boundaries = parsed.language.callable_boundary_node_types();
    boundaries.extend(EXTRA_BOUNDARIES);
    boundaries.push("class_static_block");
    let lexical = decl
        .parent()
        .is_some_and(|d| d.kind() == "lexical_declaration");
    let body = owner.child_by_field_name("body");
    let mut at = decl.parent();
    let mut first_container = true;
    while let Some(n) = at {
        if n.id() == owner.id() {
            return true;
        }
        if boundaries.contains(&n.kind()) {
            return false;
        }
        if lexical && first_container && !matches!(n.kind(), "lexical_declaration") {
            // The container of the declaration must be the owner's body block.
            if body.is_none_or(|b| b.id() != n.id()) {
                return false;
            }
            first_container = false;
        }
        at = n.parent();
    }
    false
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
