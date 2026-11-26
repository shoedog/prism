//! Syntax predicates for the S1 read population and own-parameter-list replacement.

use crate::ast::ParsedFile;
use crate::js_write_census::walk::Site;
use tree_sitter::Node;

fn transparent(kind: &str) -> bool {
    matches!(
        kind,
        "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion"
    )
}

/// A member write target that also reads the stored value: `o.k++`, `--o[k]`, `o.k += 1`,
/// `o.k ||= 1`. A plain `o.k = 1`, `delete o.k` or pattern target stores without reading.
pub(super) fn reads_value(target: Node<'_>) -> bool {
    let mut child = target;
    while let Some(parent) = child.parent() {
        match parent.kind() {
            kind if transparent(kind) => {}
            "update_expression" => return true,
            "augmented_assignment_expression" => {
                return parent.child_by_field_name("left") == Some(child);
            }
            _ => return false,
        }
        child = parent;
    }
    false
}

/// A plain write to a parameter, in a computed key of the owner's own parameter list,
/// that the list always evaluates (no conditional operand between write and key).
pub(super) fn unconditional_key_write(parsed: &ParsedFile, site: &Site) -> bool {
    if !site.region.own_params
        || site.region.nested
        || site.region.instance_field
        || site.region.with_body
        || !site.path.comps.is_empty()
    {
        return false;
    }
    let Some(mut node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.target_start, site.target_end)
    else {
        return false;
    };
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "computed_property_name" => {
                return parent
                    .parent()
                    .is_some_and(|pair| pair.kind() == "pair_pattern");
            }
            kind if transparent(kind) => {}
            "assignment_expression" => {
                if parent.child_by_field_name("left") != Some(node) {
                    return false;
                }
            }
            "sequence_expression" => {}
            _ => return false,
        }
        node = parent;
    }
    false
}
