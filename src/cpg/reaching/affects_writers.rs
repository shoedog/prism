//! #674: which recorded defs of a JS/TS/TSX owner may kill by `affects`. The rule, stated once: a recorded
//! write kills by `affects` only when ALL hold; otherwise it keeps main's path equality (fail closed).
//!
//! 1. **Identity equality.** The census computes a precise identity for the same write site (the identity
//!    `completion.rs` compares with recorded coverage, keyed by the target's start byte) and the recorded
//!    def's `AccessPath` equals it exactly, base and every field. `AccessPath::from_expr` cuts an lvalue at its
//!    first `[` (`o[k].x` is recorded `o.[]`, the census says `o.[].x`), splits text on dots (a key
//!    `"a.b.c"`), and spells wrapped receivers oddly (`(this as any).a` has base `this as any`; the census
//!    says `this`): every such def differs from its census identity and so keeps path equality. No census
//!    site (a declaration, a parameter, a refused owner or target kind, an escaped spelling, a parse error)
//!    is the same. The census's own precise kill-only defs carry the `affects` kills of the sites it covers
//!    differently.
//! 2. **Receiver.** If the base is `this`, it is not rebound relative to the owner (`this_is_rebound_at`:
//!    class static field values, static blocks, non-arrow callables; computed keys keep the owner's).
//! 3. **Declarations.** The base is not bound, between the write and the owner, by a catch parameter or by a
//!    function or class declared directly in an enclosing block or `switch` body: no recorded def, so
//!    `BindingFacts` would resolve the write to the outer binding. Matched by spelling (over-approximates).
//! 4. **Not deferred.** Not inside a nested callable or an instance field initializer (a capture region):
//!    D2-c labels the outer edges those writes reach.
//!
//! Recorded writes with no census identity (the cost): declarations and parameters (`let o`, `o` as a
//! formal), `for` head targets and other forms the census does not mark a candidate, everything above.
use super::capture_regions::JsRegions;
use super::census_defs::CensusFacts;
use super::DefSite;
use crate::ast::ParsedFile;
use crate::js_write_census::completion::this_is_rebound_at;
use tree_sitter::Node;

/// One flag per recorded def (`defs[..n_recorded]`): may it kill by `affects`.
pub(super) fn affects_writers(
    parsed: &ParsedFile,
    owner: Node<'_>,
    census: &CensusFacts,
    defs: &[DefSite],
    regions: Option<&JsRegions<'_>>,
) -> Vec<bool> {
    let root = parsed.tree.root_node();
    defs.iter()
        .map(|def| {
            if regions.is_some_and(|r| r.innermost(def.start_byte).is_some()) || !census.covers(def)
            {
                return false;
            }
            let Some(start) = root.descendant_for_byte_range(def.start_byte, def.start_byte + 1)
            else {
                return false;
            };
            !(def.path.base == "this" && this_is_rebound_at(parsed, owner, def.start_byte))
                && !bound_without_def(parsed, owner, start, &def.path.base)
        })
        .collect()
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
                if declares(n, base, &text) {
                    return true;
                }
            }
            // A function declared in a case is scoped to the whole switch body, sibling cases included.
            "switch_body" => {
                let mut cursor = n.walk();
                let declared = n
                    .named_children(&mut cursor)
                    .any(|case| declares(case, base, &text));
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

/// Whether a direct child of `block` declares a function or class named `base`.
fn declares<'a>(block: Node<'_>, base: &str, text: &impl Fn(Node<'_>) -> &'a str) -> bool {
    let mut cursor = block.walk();
    let found = block.named_children(&mut cursor).any(|c| {
        matches!(
            c.kind(),
            "function_declaration" | "generator_function_declaration" | "class_declaration"
        ) && c
            .child_by_field_name("name")
            .is_some_and(|id| text(id) == base)
    });
    found
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
