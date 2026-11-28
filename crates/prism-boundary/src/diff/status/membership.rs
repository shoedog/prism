use crate::diff::fields::{self, OperationFields};
use crate::diff::limits::Ledger;
use crate::diff::operations::operation_sites;
use crate::diff::*;
use crate::inventory::success_key;
use crate::{Operation, StatusInventory};
use std::collections::BTreeSet;

pub(super) fn candidates(op: &Operation) -> impl Iterator<Item = SuccessKey> + '_ {
    op.declarations.iter().flat_map(move |site| {
        match &site.status_inventory {
            StatusInventory::Readable(keys) => keys.as_slice(),
            StatusInventory::Withheld => &[],
        }
        .iter()
        .filter_map(|key| success_key(key, op.ranged_responses))
    })
}

pub(super) fn membership(op: &Operation, key: SuccessKey) -> Presence {
    let spelling = key.to_string();
    let proofs = || {
        op.declarations
            .iter()
            .map(|site| match &site.status_inventory {
                StatusInventory::Withheld => None,
                StatusInventory::Readable(keys) => Some(
                    (key != SuccessKey::Range || op.ranged_responses) && keys.contains(&spelling),
                ),
            })
    };
    if !op.declarations.is_empty() && proofs().all(|proof| proof == Some(true)) {
        Presence::Present
    } else if !op.declarations.is_empty() && proofs().all(|proof| proof == Some(false)) {
        Presence::Absent
    } else {
        Presence::Unknown
    }
}

pub(in crate::diff) fn established_keys(op: &Operation) -> Vec<SuccessKey> {
    candidates(op)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| membership(op, *key) == Presence::Present)
        .collect()
}

pub(in crate::diff) fn standing(
    ledger: &mut Ledger,
    side: ComparisonSide,
    op: &Operation,
    fields: &OperationFields<'_>,
) {
    for site in &op.declarations {
        if site.status_inventory == StatusInventory::Withheld {
            let mut limit = ComparisonLimit::new(
                side,
                LimitScope::Status(op.id.clone(), None),
                ComparisonClaim::StatusMembership,
                op.status_refusals
                    .get(&site.site)
                    .copied()
                    .unwrap_or("status_membership_unknown"),
            )
            .with_inventory_origin();
            let mut evidence = site.site.clone();
            evidence.pointer.push("responses".into());
            limit.evidence.push(evidence);
            ledger.record(limit);
        }
    }
    if op.declarations.len() > 1 {
        for key in candidates(op).collect::<BTreeSet<_>>() {
            if membership(op, key) == Presence::Unknown {
                record_unknown(ledger, side, op, key, None);
            }
        }
    }
    fields::standing(op, fields, side, ledger);
}

pub(super) fn record_unknown(
    ledger: &mut Ledger,
    side: ComparisonSide,
    op: &Operation,
    key: SuccessKey,
    presence: Option<&[ElementPresence; 2]>,
) {
    let mut limit = ComparisonLimit::new(
        side,
        LimitScope::Status(op.id.clone(), Some(key)),
        ComparisonClaim::StatusMembership,
        "status_membership_unknown",
    );
    limit = match presence {
        Some(presence) => limit.with_presence(presence),
        None => limit.with_inventory_origin(),
    };
    limit.evidence = operation_sites(op);
    ledger.record(limit);
}
