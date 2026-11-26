mod evidence;
mod shape;

use super::limits::Ledger;
use super::*;
use shape::{enums, shape_changes};
use std::collections::{BTreeMap, BTreeSet};

pub(super) use evidence::{standing, OperationFields, Root};

pub(super) fn compare<'a>(
    diff: &mut SchemaDiff<'a>,
    ledger: &mut Ledger,
    roots: [Root<'_, 'a>; 2],
) -> Vec<FieldPair> {
    for (i, root) in roots.iter().enumerate() {
        let reason = match root.key {
            RootKey::Request if !root.op.request_eligible => "request_dialect_excluded",
            RootKey::Response(_) if root.message.is_none() => "root_inventory_unavailable",
            _ => continue,
        };
        root.limit(
            SIDES[i],
            ledger,
            &[],
            ComparisonClaim::Coverage,
            reason,
            None,
        );
    }
    let paths: BTreeSet<_> = roots
        .iter()
        .filter_map(|root| root.paths)
        .flat_map(BTreeMap::keys)
        .collect();
    let mut pairs = Vec::new();
    for pointer in paths {
        let [before, after] = roots.each_ref().map(|root| root.lookup(pointer));
        let reasons = [0, 1].map(|i| match [before, after][i] {
            Some(entry) if entry.comparable => roots[i].authority_reason(),
            entry => roots[i].presence_reason(pointer).or_else(|| {
                entry
                    .filter(|_| !roots[i].declaration_only(pointer))
                    .map(|_| "field_presence_unchecked")
            }),
        });
        let presence = [0, 1].map(|i| {
            if let Some(entry) =
                [before, after][i].filter(|entry| entry.comparable && reasons[i].is_none())
            {
                ElementPresence::present(ElementId::Field(entry.field.id.clone()))
            } else {
                ElementPresence::without_id(if reasons[i].is_some() {
                    Presence::Unknown
                } else {
                    Presence::Absent
                })
            }
        });
        let states = presence.each_ref().map(|side| side.presence);
        if reasons.iter().any(Option::is_some) {
            for i in 0..2 {
                if let Some(reason) = reasons[i] {
                    if [before, after][i].is_none() && states[1 - i] == Presence::Present {
                        if let Some(stop) = roots[i].slot_unchecked(pointer) {
                            roots[i].limit(
                                SIDES[i],
                                ledger,
                                pointer,
                                ComparisonClaim::Property(ShapeSlot::Type),
                                stop,
                                None,
                            );
                        }
                    }
                    // A standing region record already covers paths blocked on both sides.
                    if states[1 - i] == Presence::Present || reason == "field_presence_unchecked" {
                        roots[i].limit(
                            SIDES[i],
                            ledger,
                            pointer,
                            ComparisonClaim::FieldPresence,
                            reason,
                            Some(&presence),
                        );
                    }
                }
            }
            continue;
        }

        let evidence = [before, after].map(|entry| entry.map(|entry| entry.field));
        let mut change = ElementChange::new(
            &roots[0].op.id,
            [0, 1].map(|i| {
                let mut side = ChangeSide::new(presence[i].clone());
                if let Some(field) = evidence[i] {
                    side.declaration_id = Some(field.id.clone());
                    side.shape = Some(field.shape.clone());
                    side.sites.clone_from(&field.sites);
                }
                side
            }),
        );
        if let [Some(b), Some(a)] = evidence {
            if states == [Presence::Present; 2] {
                if roots[0].key != roots[1].key {
                    pairs.push(FieldPair {
                        before: b.id.clone(),
                        after: a.id.clone(),
                    });
                }
                shape_changes(&mut change.kinds, &roots, [b, a]);
            }
            enums(&mut change, &b.shape.enum_values, &a.shape.enum_values);
        }
        if !change.kinds.contains(&ChangeKind::EnumDeclarationChanged) {
            match states {
                [Presence::Absent, Presence::Present] => change.kinds.push(ChangeKind::FieldAdded),
                [Presence::Present, Presence::Absent] => {
                    change.kinds.push(ChangeKind::FieldRemoved)
                }
                _ => (),
            }
        }
        if !change.kinds.is_empty() {
            change.kinds.sort_unstable();
            diff.changes.push(change);
        }
    }
    pairs
}
