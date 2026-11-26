// P1c structural diff claims
// Compare admitted revisions with a common API, operation authority, and logical scope.
// Report whole-operation edits with their own statuses and observed field detail.
// Report explicit success-key edits independently of response-body eligibility.
// Pair unique proved JSON replacements and preserve both original root and field IDs.
// Compare named JSON field presence on checked evidence and compare all five signed shape slots.
// Keep declaration and member deltas independently for readable enum evidence.
// Preserve side-local unknowns, scope, reasons, evidence, and deduplicated limit counts.
// Report independently proved producer fact changes through typed revision-local inputs.
// Preserve unchanged structural facts through formatting, definition renames, and equivalent projections.
// Stay silent on value-only, unsigned union const, and unsigned item/map facts within P1b's known limits.
//
// known_limits deferred paired assertions from the gate disposition (28):
// O3; S2, S4, S8, S11–S14; R4, R5 reverse.
// F1 request/list/map variants, F3, F7, F8; T4 missing directions, T6, T8, T10; E4 disjoint, E7.
// C3, D3.
// N4, N6, N7, N9–N11. The focused N9/N10 silence controls reuse existing P1b inputs.

#[path = "support/admission.rs"]
mod admission;
#[path = "support/diff_audit.rs"]
mod diff_audit;
#[path = "support/diff_edges.rs"]
mod diff_edges;
#[path = "support/diff_input.rs"]
mod diff_input;
#[path = "support/diff_rows.rs"]
mod diff_rows;
#[allow(dead_code)]
#[path = "support/matrix.rs"]
mod matrix;

use prism_boundary::element::PointerToken;
use prism_boundary::{
    diff_schemas, ChangeKind, ComparisonClaim, ComparisonSide, Property, ShapeSlot,
};
use serde_json::Value;

#[test]
fn operation_status_and_root_changes() {
    diff_rows::run(&["operations", "statuses", "pairing"]);
}

#[test]
fn named_field_and_enum_changes() {
    diff_rows::run(&["fields", "shapes", "enums"]);
}

#[test]
fn standing_limits_collision_evidence_and_comparison_basis() {
    diff_rows::run(&["limits", "collisions"]);
}

#[test]
fn retained_producer_facts() {
    diff_rows::run(&["producer"]);
}

#[test]
fn review_fold1() {
    diff_rows::run(&["fold1"]);
}

#[test]
fn existing_diff_controls_and_neutral_edits() {
    diff_audit::audit_inputs();
    diff_rows::run(&["control", "neutral"]);
}

#[test]
fn each_unavailable_slot_leaves_an_independent_field_edit_reportable() {
    let bytes = diff_input::fixture("p1c_rows", "wire/string.json");
    let mut failures = Vec::new();
    for slot in [
        ShapeSlot::Type,
        ShapeSlot::Format,
        ShapeSlot::Required,
        ShapeSlot::Enum,
        ShapeSlot::ItemType,
    ] {
        for branch_dependent in [true, false] {
            let mut before = diff_input::schema(&bytes, "test-api", &Value::Null, 0);
            let mut after = diff_input::schema(&bytes, "test-api", &Value::Null, 1);
            let before_shape = &mut before.fields[0].shape;
            let after_shape = &mut after.fields[0].shape;
            match slot {
                ShapeSlot::Type => mark_unavailable(&mut before_shape.field_type, branch_dependent),
                ShapeSlot::Format => mark_unavailable(&mut before_shape.format, branch_dependent),
                ShapeSlot::Required => {
                    mark_unavailable(&mut before_shape.required, branch_dependent)
                }
                ShapeSlot::Enum => {
                    mark_unavailable(&mut before_shape.enum_values, branch_dependent)
                }
                ShapeSlot::ItemType => {
                    mark_unavailable(&mut before_shape.item_type, branch_dependent)
                }
            }
            let expected_kind = if slot == ShapeSlot::Required {
                after_shape.field_type = Property::Known(vec!["integer".into()]);
                ChangeKind::FieldTypeChanged
            } else {
                after_shape.required = Property::Known(true);
                ChangeKind::FieldRequiredAdded
            };
            let diff = diff_schemas(&before, &after).unwrap();
            let kinds: Vec<_> = diff
                .changes
                .iter()
                .flat_map(|row| row.kinds.iter().copied())
                .collect();
            let limits = diff
                .limits
                .iter()
                .filter(|limit| {
                    limit.side == ComparisonSide::Before
                        && limit.claim == ComparisonClaim::Property(slot)
                        && limit.pointer == [PointerToken::Property("x".into())]
                })
                .count();
            if kinds != [expected_kind] || limits != 1 {
                failures.push(format!("{slot:?}, branch-dependent={branch_dependent}: expected {expected_kind:?} and one limit, got {kinds:?} and {limits}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "independent slot comparison\n{}",
        failures.join("\n")
    );
}

fn mark_unavailable<T>(property: &mut Property<T>, branch_dependent: bool) {
    *property = if branch_dependent {
        Property::BranchDependent
    } else {
        Property::Unknown
    };
}

#[test]
fn unsigned_union_const_and_item_edits_are_silent() {
    for (row, before, after) in [
        ("fold1-union-const", "before.json", "after.json"),
        ("fold1-union-const", "any-before.json", "any-after.json"),
        ("fold4-item-enum-slot", "before.json", "after.json"),
        ("fold4-item-format-slot", "before.json", "after.json"),
        (
            "fold3-root-scalar-slots",
            "array-before.json",
            "array-after.json",
        ),
        (
            "fold3-root-scalar-slots",
            "map-before.json",
            "map-after.json",
        ),
        ("fold6-item-value-bounds", "narrow.json", "wide.json"),
    ] {
        let side = |name, side| {
            let bytes = diff_input::fixture("p1b_rows", &format!("{row}/{name}"));
            diff_input::schema(&bytes, "test-api", &Value::Null, side)
        };
        let before = side(before, 0);
        let after = side(after, 1);
        assert_ne!(before.byte_sha256, after.byte_sha256, "{row}");
        assert_eq!(before.content_hash, after.content_hash, "{row}");
        let diff = diff_schemas(&before, &after).unwrap();
        assert!(diff.changes.is_empty(), "{row}: {:?}", diff.changes);
        assert!(diff.response_root_pairs.is_empty(), "{row}");
        assert_eq!(diff.limit_counts.before, diff.limit_counts.after, "{row}");
    }

    // Same fold-6 request fixture, with a signed change from number to integer.
    let input = |row, name, side| {
        let bytes = diff_input::fixture("p1b_rows", &format!("{row}/{name}"));
        diff_input::schema(&bytes, "test-api", &Value::Null, side)
    };
    let before = input("fold6-item-value-bounds", "wide.json", 0);
    let after = input("control-fold6-item-value-bounds", "integer.json", 1);
    let diff = diff_schemas(&before, &after).unwrap();
    let changes: Vec<_> = diff
        .changes
        .iter()
        .flat_map(|row| row.kinds.iter().copied())
        .collect();
    assert_eq!(
        changes,
        [ChangeKind::FieldItemTypeChanged],
        "signed item-type control"
    );
}
