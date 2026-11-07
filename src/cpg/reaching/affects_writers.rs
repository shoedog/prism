//! #674: which recorded defs of a JS/TS/TSX owner may kill by `affects`. A write kills below its own path
//! only when the recorded path IS the written storage, and only when it is not in code that runs later.
//!
//! * A truncated writer does not: `AccessPath::from_expr` cuts an lvalue at its first `[`, so `o[k].x = 2`
//!   and `o[k][0] = 2` are recorded as `o.[]`, `o.a[k].b = 1` as `o.a.[]`. Treated as the storage `o[k]`
//!   they would kill every `o.<f>` def, though `o[k].x` can never replace one. They keep main's equal-path
//!   kill; the census already adds the precise kill-only def (`o.[].x`) that carries their `affects` kill.
//!   The test is structural: the lvalue's chain from the def's own start byte has more member and
//!   subscript steps than the recorded path has fields. (A truncated def stays a valid kill TARGET: a
//!   shorter writer that covers its recorded prefix covers the longer storage.)
//! * A write in a nested callable or an instance field initializer (a capture region) does not: it may run
//!   later or never, and its binding may be a closure-local one the flat-fallback binding treats as the
//!   outer one. It keeps main's equal-path kill; D2-c labels the outer edges it affects.
use super::capture_regions::JsRegions;
use super::DefSite;
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// One flag per recorded def (`defs[..n_recorded]`): may it kill by `affects`.
pub(super) fn affects_writers(
    parsed: &ParsedFile,
    defs: &[DefSite],
    regions: Option<&JsRegions<'_>>,
) -> Vec<bool> {
    let root = parsed.tree.root_node();
    defs.iter()
        .map(|def| {
            let in_region = regions.is_some_and(|r| r.innermost(def.start_byte).is_some());
            !in_region && chain_steps(root, def.start_byte) <= def.path.fields.len()
        })
        .collect()
}

/// Member and subscript steps of the access chain that starts at `start` (the lvalue's base).
fn chain_steps(root: Node<'_>, start: usize) -> usize {
    let Some(mut node) = root.descendant_for_byte_range(start, start + 1) else {
        return 0;
    };
    let mut steps = 0;
    while let Some(parent) = node.parent() {
        let is_step = matches!(parent.kind(), "member_expression" | "subscript_expression")
            && parent.child_by_field_name("object").map(|o| o.id()) == Some(node.id());
        let is_wrapper = matches!(
            parent.kind(),
            "parenthesized_expression"
                | "non_null_expression"
                | "as_expression"
                | "satisfies_expression"
        );
        if is_step {
            steps += 1;
        } else if !is_wrapper {
            break;
        }
        node = parent;
    }
    steps
}
