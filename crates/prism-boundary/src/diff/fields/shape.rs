use super::Root;
use crate::diff::limits::Ledger;
use crate::diff::*;
use crate::{Field, Property};
use std::collections::{BTreeMap, BTreeSet};

fn unavailable<T>(property: &Property<T>) -> Option<&'static str> {
    match property {
        Property::Unknown => Some("unknown"),
        Property::BranchDependent => Some("branch_dependent"),
        _ => None,
    }
}

pub(super) fn shape_limits(
    root: &Root<'_, '_>,
    field: &Field,
    side: ComparisonSide,
    ledger: &mut Ledger,
) {
    let FieldShape {
        field_type,
        format,
        required,
        enum_values,
        item_type,
    } = &field.shape;
    for (slot, reason) in [
        (ShapeSlot::Type, unavailable(field_type)),
        (ShapeSlot::Format, unavailable(format)),
        (
            ShapeSlot::Required,
            if *required == Property::Missing {
                Some("requiredness_missing")
            } else {
                unavailable(required)
            },
        ),
        (ShapeSlot::Enum, unavailable(enum_values)),
        (ShapeSlot::ItemType, unavailable(item_type)),
    ] {
        if let Some(reason) = reason {
            let pointer = &field.id.pointer;
            let reason = if reason == "unknown" {
                root.blocks()
                    .iter()
                    .filter(|limit| {
                        pointer.starts_with(&limit.pointer)
                            || slot == ShapeSlot::ItemType
                                && limit.pointer.strip_prefix(pointer.as_slice())
                                    == Some(&[PointerToken::ListItem])
                    })
                    .max_by_key(|limit| limit.pointer.len())
                    .map_or(reason, |limit| limit.reason.as_str())
            } else {
                reason
            };
            root.limit(
                side,
                ledger,
                pointer,
                ComparisonClaim::Property(slot),
                reason,
                None,
            );
        }
    }
}

pub(super) fn shape_changes(kinds: &mut Vec<ChangeKind>, b: &FieldShape, a: &FieldShape) {
    fn different<T: PartialEq>(b: &Property<T>, a: &Property<T>) -> bool {
        unavailable(b).is_none() && unavailable(a).is_none() && b != a
    }
    fn types_differ(b: &Property<Vec<String>>, a: &Property<Vec<String>>) -> bool {
        match (b, a) {
            (Property::Known(b), Property::Known(a)) => {
                b.iter().collect::<BTreeSet<_>>() != a.iter().collect::<BTreeSet<_>>()
            }
            _ => different(b, a),
        }
    }
    for (changed, kind) in [
        (
            types_differ(&b.field_type, &a.field_type),
            ChangeKind::FieldTypeChanged,
        ),
        (
            different(&b.format, &a.format),
            ChangeKind::FieldFormatChanged,
        ),
        (
            types_differ(&b.item_type, &a.item_type),
            ChangeKind::FieldItemTypeChanged,
        ),
        (
            (&b.required, &a.required) == (&Property::Known(false), &Property::Known(true)),
            ChangeKind::FieldRequiredAdded,
        ),
        (
            (&b.required, &a.required) == (&Property::Known(true), &Property::Known(false)),
            ChangeKind::FieldRequiredRemoved,
        ),
    ] {
        if changed {
            kinds.push(kind);
        }
    }
}

pub(super) fn enums(
    change: &mut ElementChange<'_>,
    b: &Property<Vec<EnumValue>>,
    a: &Property<Vec<EnumValue>>,
) {
    match (b, a) {
        (Property::Known(_), Property::Missing) | (Property::Missing, Property::Known(_)) => {
            change.kinds.push(ChangeKind::EnumDeclarationChanged);
        }
        (Property::Known(b), Property::Known(a)) => {
            use crate::schema::signature::member;
            let b: BTreeMap<_, _> = b.iter().map(|value| (member(value), value)).collect();
            let a: BTreeMap<_, _> = a.iter().map(|value| (member(value), value)).collect();
            change.enum_added = a
                .iter()
                .filter(|(key, _)| !b.contains_key(*key))
                .map(|(_, value)| (*value).clone())
                .collect();
            change.enum_removed = b
                .iter()
                .filter(|(key, _)| !a.contains_key(*key))
                .map(|(_, value)| (*value).clone())
                .collect();
            if !change.enum_added.is_empty() {
                change.kinds.push(ChangeKind::EnumValuesAdded);
            }
            if !change.enum_removed.is_empty() {
                change.kinds.push(ChangeKind::EnumValuesRemoved);
            }
        }
        _ => (),
    }
}
