//! R2 (alias-twin lane slice 4c, fold 2): the syntactic may-hold closure.
//!
//! A binding twin of an alias `u` with resolved target base `o` is kept when the owner may write through some other
//! name that can hold `o`'s value. Instead of relating written names to `o` through the producer's alias map (last
//! target per name, plain-identifier initialisers only), `Hold(o)` is the connected component of `o` in the
//! co-occurrence graph of the owner: every UNIT (a leaf statement with its whole subtree including nested callable
//! expressions and class bodies, a statement header, the owner's parameter list, an expression body) unions all the
//! variable names it mentions (identifier references, binding names and `this`; never property names, keys, labels
//! or type names). Flow-insensitive and undirected, so it is independent of alias formation syntax, rebinding
//! order, containers, closures, `this` and callbacks.
//!
//! A non-arrow callable's formals and its own `arguments` object hold the same values, so each such callable that mentions
//! `arguments` adds one unit joining them. The `r != o` exclusion (a write through the target itself is the direct write) applies only when the
//! target spelling has exactly one binding in the owner subtree (census declarations, formals and catch parameters of
//! nested callables included) and, for `this`, no nested non-arrow callable writes through `this`; otherwise a written
//! root spelled like the target may be another binding holding its value and counts like any other held name.
//!
//! The twins of `u` are kept iff some written root of the write census (a non-declaration member or subscript write
//! site) `r != o` is in `Hold(o)`, or a write target whose root is not a plain name (call, sequence, conditional,
//! literal, `new`) mentions a name of `Hold(o)`. A census-refused owner keeps every twin. Out of model, as for `Exact`:
//! a value reaching a written name with no co-occurrence in the owner (module state through calls), a write in another
//! owner through a captured name, a write in a `with` body, names that alias `o` only by the caller's choice.
use super::shared::{field_in_parent, walk_owner};
use crate::ast::ParsedFile;
use crate::js_write_census::walk::{census_with, Form};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Statement-like nodes: units of their own (their child statements are separate units).
fn is_stmt(kind: &str) -> bool {
    kind.ends_with("_statement")
        || matches!(
            kind,
            "lexical_declaration"
                | "variable_declaration"
                | "statement_block"
                | "else_clause"
                | "catch_clause"
                | "finally_clause"
                | "switch_case"
                | "switch_default"
                | "switch_body"
                | "class_static_block"
                | "function_declaration"
                | "generator_function_declaration"
                | "class_declaration"
        )
}

/// Callable and class nodes: everything below them belongs to the enclosing unit.
fn is_whole(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "function_expression"
            | "function"
            | "generator_function"
            | "generator_function_declaration"
            | "arrow_function"
            | "method_definition"
            | "class_declaration"
            | "class"
            | "class_body"
    )
}

fn is_name(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
            | "this"
    )
}

/// The names a unit rooted at `node` mentions. `crossed` is true once a callable or class has
/// been entered: its whole subtree (child statements included) is part of the unit.
fn collect(parsed: &ParsedFile, node: Node<'_>, crossed: bool, out: &mut Vec<String>) {
    if is_name(node.kind()) {
        out.push(parsed.node_text(&node).trim().to_string());
        return;
    }
    let crossed = crossed || is_whole(node.kind());
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !crossed && is_stmt(child.kind()) {
            continue;
        }
        collect(parsed, child, crossed, out);
    }
}

/// True when the identifier `arguments` occurs in the callable: its parameter list, its body and the arrow functions,
/// parameter defaults and class field or static positions that inherit its `arguments`; a nested non-arrow callable has
/// its own and is not entered. (Escaped spellings are refused by the census. A direct `eval("arguments[0].x = 1")`
/// is covered by the refusal classes, which demote every `exact` or `sameline` label of the owner, so the twin's kill
/// is not what keeps it off `exact`.)
fn mentions_arguments(parsed: &ParsedFile, callable: Node<'_>) -> bool {
    let mut stack = vec![callable];
    while let Some(n) = stack.pop() {
        if n.kind() == "identifier" && parsed.node_text(&n).trim() == "arguments" {
            return true;
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            let nested = matches!(
                child.kind(),
                "function_declaration"
                    | "function_expression"
                    | "function"
                    | "generator_function"
                    | "generator_function_declaration"
                    | "method_definition"
            );
            if !nested {
                stack.push(child);
            }
        }
    }
    false
}

fn chain_root_is_name(node: Node<'_>) -> bool {
    let mut cur = node;
    loop {
        match cur.kind() {
            "member_expression" | "subscript_expression" => match cur.child_by_field_name("object")
            {
                Some(o) => cur = o,
                None => return false,
            },
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression" => match cur.named_child(0) {
                Some(c) => cur = c,
                None => return false,
            },
            "type_assertion" => match cur.named_child(cur.named_child_count().saturating_sub(1)) {
                Some(c) => cur = c,
                None => return false,
            },
            "identifier" | "this" => return true,
            _ => return false,
        }
    }
}

/// True when the member or subscript expression is written (assignment, update, `delete`, `for` head, destructuring
/// leaf with or without a default).
fn is_write_target(node: Node<'_>) -> bool {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        match parent.kind() {
            "member_expression" | "subscript_expression" => {
                if parent.child_by_field_name("object").map(|o| o.id()) != Some(cur.id()) {
                    return false;
                }
            }
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion" => {}
            "assignment_expression" | "augmented_assignment_expression" => {
                return field_in_parent(cur) == Some("left");
            }
            "update_expression" => return true,
            "unary_expression" => {
                return parent
                    .child_by_field_name("operator")
                    .is_some_and(|op| op.kind() == "delete");
            }
            "for_in_statement" => return field_in_parent(cur) == Some("left"),
            "assignment_pattern" | "object_assignment_pattern" => {
                if field_in_parent(cur) != Some("left") {
                    return false;
                }
            }
            "pair_pattern" | "array_pattern" | "object_pattern" | "rest_pattern"
            | "spread_element" | "array" | "object" | "pair" => return in_assignment(parent),
            _ => return false,
        }
        cur = parent;
    }
    false
}

fn in_assignment(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "pair_pattern"
            | "array_pattern"
            | "object_pattern"
            | "rest_pattern"
            | "spread_element"
            | "array"
            | "object"
            | "pair"
            | "parenthesized_expression"
            | "assignment_pattern"
            | "object_assignment_pattern" => {}
            "assignment_expression" | "augmented_assignment_expression" => {
                return field_in_parent(node) == Some("left")
            }
            "for_in_statement" => return field_in_parent(node) == Some("left"),
            _ => return false,
        }
        node = parent;
    }
    false
}

/// The owner's may-hold facts, computed once per owner.
pub(super) struct Hold {
    /// `None`: the census refused the owner (keep every twin).
    facts: Option<Facts>,
}

struct Facts {
    roots: BTreeSet<String>,
    /// Names with more than one binding in the owner subtree (a written root spelled like the target may be another
    /// binding that holds its value), plus `this` when a nested non-arrow callable writes through `this`.
    multi: BTreeSet<String>,
    /// The names each unit mentions.
    units: Vec<Vec<String>>,
    /// Names mentioned by each write target whose root is not a plain name.
    nonname: Vec<Vec<String>>,
    cache: BTreeMap<String, BTreeSet<String>>,
}

impl Hold {
    pub(super) fn new(parsed: &ParsedFile, owner: &Node<'_>) -> Self {
        let census = census_with(
            parsed,
            crate::js_write_census::tables::Tables::shared(),
            *owner,
            false,
        );
        if census.refused() {
            return Self { facts: None };
        }
        let roots: BTreeSet<String> = census
            .sites
            .iter()
            .filter(|s| s.form != Form::Declare && !s.path.comps.is_empty())
            .map(|s| s.path.base.clone())
            .collect();
        let mut bindings: BTreeMap<&str, usize> = BTreeMap::new();
        for s in census
            .sites
            .iter()
            .filter(|s| s.form == Form::Declare && s.path.comps.is_empty())
        {
            *bindings.entry(s.path.base.as_str()).or_default() += 1;
        }
        let mut multi: BTreeSet<String> = bindings
            .into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(b, _)| b.to_string())
            .collect();
        let nested_this = |byte: usize| {
            let Some(mut cur) = parsed
                .tree
                .root_node()
                .descendant_for_byte_range(byte, byte + 1)
            else {
                return true;
            };
            while let Some(p) = cur.parent() {
                if p.id() == owner.id() {
                    return false;
                }
                if matches!(
                    p.kind(),
                    "function_declaration"
                        | "function_expression"
                        | "function"
                        | "generator_function"
                        | "generator_function_declaration"
                        | "method_definition"
                        | "class_static_block"
                        | "field_definition"
                        | "public_field_definition"
                ) {
                    return true;
                }
                cur = p;
            }
            false
        };
        if census.sites.iter().any(|s| {
            s.form != Form::Declare
                && !s.path.comps.is_empty()
                && s.path.base == "this"
                && s.region.nested
                && nested_this(s.target_start)
        }) {
            multi.insert("this".to_string());
        }
        let mut nonname = Vec::new();
        let mut units = Vec::new();
        let mut expr_body = false;
        walk_owner(owner, |node| {
            let kind = node.kind();
            // A non-arrow callable's formals and its own `arguments` object hold the same values.
            if matches!(
                kind,
                "function_declaration"
                    | "function_expression"
                    | "function"
                    | "generator_function"
                    | "generator_function_declaration"
                    | "method_definition"
            ) {
                if let (Some(p), true) = (
                    node.child_by_field_name("parameters"),
                    mentions_arguments(parsed, node),
                ) {
                    let mut names = vec!["arguments".to_string()];
                    collect(parsed, p, true, &mut names);
                    units.push(names);
                }
            }
            if is_stmt(kind) && node.id() != owner.id() {
                let mut names = Vec::new();
                collect(parsed, node, false, &mut names);
                units.push(names);
            }
            if node.id() == owner.id() {
                expr_body = owner
                    .child_by_field_name("body")
                    .is_some_and(|b| !is_stmt(b.kind()));
            }
            if matches!(kind, "member_expression" | "subscript_expression")
                && !chain_root_is_name(node)
                && is_write_target(node)
            {
                let mut names = Vec::new();
                collect(parsed, node, true, &mut names);
                nonname.push(names);
            }
        });
        // Each formal of the owner's own parameter list is a unit (a default joins the formal with the names it
        // mentions), as is an expression body.
        if let Some(p) = owner.child_by_field_name("parameters") {
            let mut cursor = p.walk();
            for formal in p.named_children(&mut cursor) {
                let mut names = Vec::new();
                collect(parsed, formal, true, &mut names);
                units.push(names);
            }
        } else if let Some(p) = owner.child_by_field_name("parameter") {
            let mut names = Vec::new();
            collect(parsed, p, true, &mut names);
            units.push(names);
        }
        if expr_body {
            if let Some(b) = owner.child_by_field_name("body") {
                let mut names = Vec::new();
                collect(parsed, b, true, &mut names);
                units.push(names);
            }
        }
        Self {
            facts: Some(Facts {
                multi,
                roots,
                units,
                nonname,
                cache: BTreeMap::new(),
            }),
        }
    }

    /// `Hold(o)`: the least set containing `o` such that a unit mentioning a held name puts all its names in.
    fn hold(f: &mut Facts, target: &str) -> BTreeSet<String> {
        if let Some(h) = f.cache.get(target) {
            return h.clone();
        }
        let mut h: BTreeSet<String> = BTreeSet::from([target.to_string()]);
        loop {
            let before = h.len();
            for names in &f.units {
                if names.iter().any(|n| h.contains(n)) {
                    h.extend(names.iter().cloned());
                }
            }
            if h.len() == before {
                break;
            }
        }
        f.cache.insert(target.to_string(), h.clone());
        h
    }

    /// True when the binding twins of an alias whose resolved target base is `target` must stay.
    pub(super) fn keeps(&mut self, target: &str) -> bool {
        let Some(f) = self.facts.as_mut() else {
            return true;
        };
        let h = Self::hold(f, target);
        f.roots
            .iter()
            .any(|r| (r != target || f.multi.contains(target)) && h.contains(r))
            || f.nonname.iter().any(|ns| ns.iter().any(|n| h.contains(n)))
    }
}
