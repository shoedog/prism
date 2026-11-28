use super::fields::{self, OperationFields, Root};
use super::limits::Ledger;
use super::*;
use crate::Operation;
use std::collections::BTreeSet;

mod membership;
mod pairing;

use membership::{candidates, membership, record_unknown};
pub(super) use membership::{established_keys, standing};
use pairing::Ownership;

pub(super) fn compare<'a>(
    diff: &mut SchemaDiff<'a>,
    ledger: &mut Ledger,
    ops: [&'a Operation; 2],
    fields: [&OperationFields<'a>; 2],
) {
    let keys: BTreeSet<_> = ops.into_iter().flat_map(candidates).collect();
    let mut changed: [Vec<SuccessKey>; 2] = Default::default();
    let mut retained = Vec::new();
    let mut uncertain: [Vec<SuccessKey>; 2] = Default::default();
    for key in keys {
        let states = ops.map(|op| membership(op, key));
        match states {
            [Presence::Present, Presence::Absent] => changed[0].push(key),
            [Presence::Absent, Presence::Present] => changed[1].push(key),
            [Presence::Present, Presence::Present] => retained.push(key),
            _ => {
                let presence = [0, 1].map(|i| status_presence(ops[i], key, states[i]));
                for i in 0..2 {
                    if states[i] == Presence::Unknown {
                        uncertain[i].push(key);
                        record_unknown(ledger, SIDES[i], ops[i], key, Some(&presence));
                    }
                }
            }
        }
    }
    let roots = |key: RootKey| [0, 1].map(|i| Root::new(ops[i], fields[i], key.clone()));
    let ownership = ops.map(Ownership::new);
    let unique = ops.iter().all(|op| op.declarations.len() == 1);
    let paired = pairing::select(ledger, ops, fields, &ownership, &changed, &uncertain);
    let lost_range = pairing::range_reassigned(&ownership);
    for i in 0..2 {
        for key in &changed[i] {
            let mut sides =
                ops.map(|op| ChangeSide::new(status_presence(op, *key, Presence::Absent)));
            sides[i] = status_side(ops[i], *key);
            if paired.is_none_or(|keys| keys[i] != *key) {
                sides[i].fields = fields[i].details(ops[i], Some(&RootKey::Response(*key)));
                if lost_range[1 - i].contains(key) {
                    sides[1 - i].fields = fields[1 - i]
                        .details(ops[1 - i], Some(&RootKey::Response(SuccessKey::Range)));
                }
            }
            let mut change = ElementChange::new(&ops[i].id, sides);
            change.kinds.push(
                [
                    ChangeKind::SuccessStatusRemoved,
                    ChangeKind::SuccessStatusAdded,
                ][i],
            );
            diff.changes.push(change);
        }
    }
    if !unique {
        return;
    }
    let request = roots(RootKey::Request);
    if request.iter().any(|root| root.message.is_some()) {
        fields::compare(diff, ledger, request);
    }
    for key in retained {
        if key == SuccessKey::Range && ownership[0].owns_range() != ownership[1].owns_range() {
            continue;
        }
        fields::compare(diff, ledger, roots(RootKey::Response(key)));
    }
    if let Some([before, after]) = paired {
        let roots = [
            Root::new(ops[0], fields[0], RootKey::Response(before)),
            Root::new(ops[1], fields[1], RootKey::Response(after)),
        ];
        let fields = fields::compare(diff, ledger, roots);
        diff.response_root_pairs.push(ResponseRootPair {
            operation: ops[0].id.clone(),
            before,
            after,
            fields,
        });
    }
}

fn status_presence(op: &Operation, key: SuccessKey, state: Presence) -> ElementPresence {
    if state == Presence::Present {
        ElementPresence::present(ElementId::Status(op.id.clone(), key))
    } else {
        ElementPresence::without_id(state)
    }
}

fn status_side<'a>(op: &Operation, key: SuccessKey) -> ChangeSide<'a> {
    let mut side = ChangeSide::new(status_presence(op, key, Presence::Present));
    side.sites = op
        .declarations
        .iter()
        .map(|declaration| {
            let mut site = declaration.site.clone();
            site.pointer.extend(["responses".into(), key.to_string()]);
            site
        })
        .collect();
    side
}
