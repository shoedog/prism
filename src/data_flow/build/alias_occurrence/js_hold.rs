//! R2 (alias-twin lane slice 4c, fold 3): the value-flow closure VF.
//!
//! A binding twin of an alias `u` with resolved target `o` is kept when the owner may write through some other name
//! that can hold `o`'s value. `Hold(o)` is the connected component of `o`'s identity in the owner's value-flow graph
//! (`js_hold_flow.rs`: value flow, calls, exceptions, callables, `arguments`; identities from `js_hold_resolve.rs`:
//! binding identity instead of spelling, one `this` per non-arrow callable). The twins of `u` are kept iff some
//! written root of the write census (a non-declaration member or subscript write site) other than the target's own
//! identity is in `Hold(o)`, or a write target whose root is not a plain name (call, sequence, conditional, literal,
//! `new`, `super`) can evaluate to a name of `Hold(o)`. A census-refused owner keeps every twin, as does a target
//! the owner never mentions. Undirected and flow-insensitive, so independent of alias formation syntax, rebinding
//! order, containers, closures, `this` and callbacks.
//!
//! Out of model, as for `Exact`: a value reaching a written name through no syntactic channel of the owner (module
//! state), a write in another owner through a captured name, a write in a `with` body, getters, setters and Proxy
//! traps that alias, names that alias `o` only by the caller's choice (a formal callee's captured state), and one
//! call shape: a plain call of a free name or a formal, reached by no callable, whose result is unused, that stores
//! one argument into another (`stash(arr, o); arr[0].x = 1`).
use super::js_hold_flow::Flow;
use super::shared::field_in_parent;
use crate::ast::ParsedFile;
use crate::js_write_census::walk::{census_with, Form};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// True when the member or subscript expression is written (assignment, update, `delete`, `for` head, destructuring
/// leaf with or without a default).
pub(super) fn is_write_target(node: Node<'_>) -> bool {
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

/// The owner's value-flow facts, computed once per owner.
pub(super) struct Hold {
    /// `None`: the census refused the owner (keep every twin).
    facts: Option<Facts>,
}

struct Facts {
    comp: Vec<usize>,
    by_spelling: BTreeMap<String, Vec<usize>>,
    roots: Vec<usize>,
    nonname: Vec<BTreeSet<usize>>,
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
        let mut flow = Flow::new(parsed, *owner, &census.sites);
        let roots: Vec<usize> = census
            .sites
            .iter()
            .filter(|s| s.form != Form::Declare && !s.path.comps.is_empty())
            .map(|s| flow.root_id(&s.path.base, s.target_start))
            .collect();
        let nonname: Vec<Vec<usize>> = flow.nonname().to_vec();
        let n = flow.len();
        let comp: Vec<usize> = (0..n).map(|i| flow.find(i)).collect();
        let nonname = nonname
            .into_iter()
            .map(|v| v.into_iter().map(|i| comp[i]).collect())
            .collect();
        let by_spelling = flow.spellings();
        Self {
            facts: Some(Facts {
                comp,
                by_spelling,
                roots,
                nonname,
            }),
        }
    }

    /// True when the binding twins of an alias whose resolved target base is `target` must stay.
    pub(super) fn keeps(&self, target: &str) -> bool {
        let Some(f) = self.facts.as_ref() else {
            return true;
        };
        let Some(ids) = f.by_spelling.get(target).filter(|v| !v.is_empty()) else {
            return true;
        };
        ids.iter().any(|&t| {
            let c = f.comp[t];
            f.roots.iter().any(|&r| r != t && f.comp[r] == c)
                || f.nonname.iter().any(|ns| ns.contains(&c))
        })
    }
}
