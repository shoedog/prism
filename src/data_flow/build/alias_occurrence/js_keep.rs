//! R2 (alias-twin lane slice 4c, fold 4): which binding twins of a JS/TS alias stay (rule E, "OW-excl-o").
//!
//! The twin of an alias `u` (resolved target base `o`) kills `o`'s earlier defs in JS reaching definitions, and that
//! kill is what keeps a write through `u` off `exact` where option A (#783) does not certify the alias. A binding twin
//! of `u` is therefore KEPT (not removed) iff any of:
//! - K0: the write census refused the owner;
//! - K1: a write target of the owner with a member or subscript path has a root other than `o`: a census site whose
//!   root spelling is not `o`, or a target whose root is not a plain name at all (call, `new`, literal, sequence,
//!   conditional, `new.target`, `import.meta`, `super`: the census records none of these, so they are read here);
//! - K2: such a site is rooted at `o` and `o` is not single (see below).
//!
//! Soundness. A newly wrong `exact` is an edge the twin's kill used to doubt where some write between the def and
//! the use writes `o`'s storage through a name holding `o`'s value, the direct analysis (the same write spelled on `o`)
//! being non-`exact` there. Every such write is a member or subscript write target of the owner subtree, and the write
//! census is reaching definitions' own write oracle, so with no such target (the read-only destructuring idiom) there
//! is no write to contradict the promoted label. A target rooted at another name (K1) may hold `o`'s value: kept. A
//! target rooted at `o` is literally the direct write the analysis sees when `o` is single: not `this`, `arguments` or
//! `super`, with exactly ONE census declaration in the owner subtree and that declaration scoped to the whole owner
//! body (an owner formal, or a declaration that is a direct child of the owner's body block, never in a nested
//! callable, class, inner block, loop head, `catch` or `try`), so every occurrence of the spelling in the subtree is
//! that one variable (no shadowing declaration exists; a TDZ occurrence throws). Otherwise a root spelled `o` may be
//! another binding holding the value: kept. For `o` = `this`, single holds when no `this`-rooted write lies inside a
//! non-arrow callable, a class body, field, static block, computed key or decorator (each has a `this` of its own or
//! evaluates in another context). `o.q.x = 1` after `o.q = o` is field-sensitivity the direct form shares.
//!
//! Out of model, as for `Exact`: a call that mutates a holder (`Object.assign(u, ..)`, `u.m()`), module state through
//! calls, a captured name written in another owner (no twin on base), a `with` body. A census-refused owner keeps every
//! twin. The rule is removal-only relative to OW (keep every twin of an owner with any such write target); deleting K2
//! gives OW exactly.
use super::shared::{field_in_parent, walk_owner};
use crate::ast::ParsedFile;
use crate::js_write_census::walk::{census_with, Form};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// The owner's keep facts, computed once per owner.
pub(super) struct Keep {
    /// `None`: the census refused the owner (keep every twin).
    facts: Option<Facts>,
}

struct Facts {
    /// Root spellings of the census's member or subscript write sites.
    roots: BTreeSet<String>,
    /// A write target whose root is not a plain name.
    nonname: bool,
    /// Per spelling: census declaration count and whether every one is scoped to the whole owner body.
    declares: BTreeMap<String, (usize, bool)>,
    /// A `this`-rooted write sits in a context with a `this` of its own or another context.
    nested_this: bool,
}

const PATTERN_PARENTS: [&str; 6] = [
    "object_pattern",
    "array_pattern",
    "pair_pattern",
    "rest_pattern",
    "assignment_pattern",
    "object_assignment_pattern",
];

/// Ancestors (below the owner) that give a `this`-rooted write another `this` or another evaluation context.
fn other_this_context(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "function_expression"
            | "function"
            | "generator_function"
            | "generator_function_declaration"
            | "method_definition"
            | "class_body"
            | "class_heritage"
            | "class_static_block"
            | "field_definition"
            | "public_field_definition"
            | "computed_property_name"
            | "decorator"
    )
}

fn chain_root_is_name(node: Node<'_>) -> bool {
    let mut cur = node;
    loop {
        match cur.kind() {
            "member_expression" | "subscript_expression" => {
                match cur.child_by_field_name("object") {
                    Some(o) => cur = o,
                    None => return false,
                }
            }
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

/// True when the declaration at `[start, end)` (census kind `decl`) is scoped to the whole owner body: an owner
/// formal, or a `var`/`let`/`const`, function or class declaration that is a direct child of the owner's body block.
fn whole_body(parsed: &ParsedFile, owner: &Node<'_>, decl: &str, site: (usize, usize)) -> bool {
    let Some(mut node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.0, site.1)
    else {
        return false;
    };
    let body = owner
        .child_by_field_name("body")
        .filter(|b| b.kind() == "statement_block");
    let in_body = |n: Node<'_>| {
        n.parent()
            .zip(body)
            .is_some_and(|(p, b)| p.id() == b.id())
    };
    match decl {
        "declarator" => {
            while node
                .parent()
                .is_some_and(|p| PATTERN_PARENTS.contains(&p.kind()))
            {
                node = node.parent().unwrap_or(node);
            }
            let Some(d) = node.parent().filter(|p| p.kind() == "variable_declarator") else {
                return false;
            };
            d.parent().is_some_and(|l| {
                matches!(l.kind(), "lexical_declaration" | "variable_declaration") && in_body(l)
            })
        }
        "fn-hoist" | "class" => node.parent().is_some_and(|d| {
            matches!(
                d.kind(),
                "function_declaration"
                    | "generator_function_declaration"
                    | "class_declaration"
                    | "abstract_class_declaration"
            ) && in_body(d)
        }),
        _ => false,
    }
}

/// The nearest ancestor of the byte that is the owner, with any other-`this` context met on the way.
fn in_other_this_context(parsed: &ParsedFile, owner: &Node<'_>, byte: usize) -> bool {
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
        if other_this_context(p.kind()) {
            return true;
        }
        cur = p;
    }
    true
}

impl Keep {
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
        let mut roots = BTreeSet::new();
        let mut declares: BTreeMap<String, (usize, bool)> = BTreeMap::new();
        let mut nested_this = false;
        for s in &census.sites {
            if s.form == Form::Declare {
                if s.path.comps.is_empty() {
                    let own_formal =
                        s.decl == "param" && s.region.own_params && !s.region.nested;
                    let scoped = own_formal
                        || whole_body(parsed, owner, &s.decl, (s.target_start, s.target_end));
                    let e = declares.entry(s.path.base.clone()).or_insert((0, true));
                    e.0 += 1;
                    e.1 &= scoped;
                }
            } else if !s.path.comps.is_empty() {
                roots.insert(s.path.base.clone());
                nested_this |= s.path.base == "this"
                    && in_other_this_context(parsed, owner, s.target_start);
            }
        }
        let mut nonname = false;
        walk_owner(owner, |node| {
            if !nonname
                && matches!(node.kind(), "member_expression" | "subscript_expression")
                && !chain_root_is_name(node)
                && is_write_target(node)
            {
                nonname = true;
            }
        });
        Self {
            facts: Some(Facts {
                roots,
                nonname,
                declares,
                nested_this,
            }),
        }
    }

    /// True when the binding twins of an alias whose resolved target base is `target` must stay.
    pub(super) fn keeps(&self, target: &str) -> bool {
        let Some(f) = self.facts.as_ref() else {
            return true;
        };
        if f.nonname || f.roots.iter().any(|r| r != target) {
            return true;
        }
        // Every written root is `target`: single, or keep (a root so spelled may be another binding).
        if !f.roots.contains(target) {
            return false;
        }
        let single = if target == "this" {
            !f.nested_this
        } else {
            !matches!(target, "arguments" | "super") && f.declares.get(target) == Some(&(1, true))
        };
        !single
    }
}
