//! #674: which recorded defs of a JS/TS/TSX owner may kill by `affects`. One rule: a recorded write kills
//! by `affects` only when (a) its recorded path is the COMPLETE storage identity of its target, and (b) its
//! receiver or base is the owner's own binding of that name. Otherwise it keeps main's path equality.
//!
//! (a) Storage identity. `AccessPath::from_expr` cuts an lvalue at its first `[`, so `o[k].x = 2` is
//! recorded as `o.[]`, `o[k][0] = 2` as `o.[]`, `o.a[k].b = 1` as `o.a.[]`, and `(o[k].x).y = 2` as
//! `o.[].y`. Treated as the storage it names, such a writer would kill every `o.<f>` def though the real
//! target can never replace one. The census already adds the precise kill-only def for these sites, so the
//! recorded one keeps path equality. Decision: count the target's complete access chain (member and
//! subscript steps, descending through parentheses, TS `as`/`satisfies`/`<T>`/`!` at every level) and
//! require no more steps than the recorded path has fields. (A truncated def stays a valid kill TARGET: a
//! shorter writer covering its recorded prefix covers the longer storage.)
//!
//! (b) Receiver and base identity. Not the owner's own binding, so path equality:
//! * code that runs later: a nested callable or an instance field initializer (a capture region; D2-c
//!   labels the outer edges);
//! * a `this` the census already treats as rebound (`this_is_rebound_at`: class static field values, static
//!   blocks, parameters and bodies of non-arrow callables; computed keys and decorators keep the owner's);
//! * a base bound by a declaration `BindingFacts` does not model because it has no recorded def: a catch
//!   clause parameter (plain, destructured, typed), or a function/class declared in an enclosing block.
//!   Both over-approximate by name. Remaining kinds are not chased (S1d owns the binding model).
use super::capture_regions::JsRegions;
use super::DefSite;
use crate::ast::ParsedFile;
use crate::js_write_census::completion::this_is_rebound_at;
use tree_sitter::Node;

/// One flag per recorded def (`defs[..n_recorded]`): may it kill by `affects`.
pub(super) fn affects_writers(
    parsed: &ParsedFile,
    owner: Node<'_>,
    defs: &[DefSite],
    regions: Option<&JsRegions<'_>>,
) -> Vec<bool> {
    let root = parsed.tree.root_node();
    defs.iter()
        .map(|def| {
            if regions.is_some_and(|r| r.innermost(def.start_byte).is_some()) {
                return false;
            }
            let Some(start) = root.descendant_for_byte_range(def.start_byte, def.start_byte + 1)
            else {
                return false;
            };
            if def.path.base == "this" && this_is_rebound_at(parsed, owner, def.start_byte) {
                return false;
            }
            complete_chain(start) <= def.path.fields.len()
                && !bound_without_def(parsed, owner, start, &def.path.base)
        })
        .collect()
}

/// Member and subscript steps of the complete access chain whose first token is at `start`: climb to the
/// chain's top (wrappers included), then count downward through the object of every step and wrapper.
fn complete_chain(start: Node<'_>) -> usize {
    let mut top = start;
    while let Some(parent) = top.parent() {
        let is_step = matches!(parent.kind(), "member_expression" | "subscript_expression")
            && parent.child_by_field_name("object").map(|o| o.id()) == Some(top.id());
        if !is_step && !is_wrapper(parent.kind()) {
            break;
        }
        top = parent;
    }
    depth(top)
}

fn is_wrapper(kind: &str) -> bool {
    matches!(
        kind,
        "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion"
    )
}

fn depth(node: Node<'_>) -> usize {
    match node.kind() {
        "member_expression" | "subscript_expression" => {
            1 + node.child_by_field_name("object").map_or(0, depth)
        }
        // `<T>x` puts the expression last; the other wrappers put it first (a type may follow).
        "type_assertion" => node
            .named_child(node.named_child_count().saturating_sub(1))
            .map_or(0, depth),
        k if is_wrapper(k) => node.named_child(0).map_or(0, depth),
        _ => 0,
    }
}

/// Whether `base` is bound, between `start` and the owner, by a catch parameter or a block-level function
/// or class declaration (no recorded def, so `BindingFacts` resolves a write to it as the outer binding).
fn bound_without_def(parsed: &ParsedFile, owner: Node<'_>, start: Node<'_>, base: &str) -> bool {
    let text = |n: Node<'_>| &parsed.source[n.start_byte()..n.end_byte()];
    let owner_body = owner.child_by_field_name("body").map(|b| b.id());
    let (mut prev, mut cur) = (start, start.parent());
    while let Some(n) = cur {
        if n.id() == owner.id() {
            return false;
        }
        match n.kind() {
            "catch_clause" => {
                let in_body = n
                    .child_by_field_name("body")
                    .is_some_and(|b| b.id() == prev.id());
                if in_body
                    && n.child_by_field_name("parameter")
                        .is_some_and(|p| binds(p, base, &text))
                {
                    return true;
                }
            }
            "statement_block" if Some(n.id()) != owner_body => {
                let mut cursor = n.walk();
                let declared = n.named_children(&mut cursor).any(|c| {
                    matches!(
                        c.kind(),
                        "function_declaration"
                            | "generator_function_declaration"
                            | "class_declaration"
                    ) && c
                        .child_by_field_name("name")
                        .is_some_and(|id| text(id) == base)
                });
                if declared {
                    return true;
                }
            }
            _ => {}
        }
        prev = n;
        cur = n.parent();
    }
    false
}

/// Whether the pattern `p` names `base` (any identifier inside it, a destructuring included).
fn binds<'a>(p: Node<'_>, base: &str, text: &impl Fn(Node<'_>) -> &'a str) -> bool {
    if matches!(
        p.kind(),
        "identifier" | "shorthand_property_identifier_pattern"
    ) && text(p) == base
    {
        return true;
    }
    let mut cursor = p.walk();
    let found = p.named_children(&mut cursor).any(|c| binds(c, base, text));
    found
}
