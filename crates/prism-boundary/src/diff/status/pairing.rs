use super::super::fields::{OperationFields, Root};
use super::super::limits::Ledger;
use super::super::*;
use crate::inventory::{can_have_body, success_key};
use crate::{Coverage, Operation, StatusInventory};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    Key(SuccessKey),
    Fallback,
}

impl Owner {
    fn is_wildcard(self) -> bool {
        matches!(self, Self::Key(SuccessKey::Range) | Self::Fallback)
    }
}

pub(super) struct Ownership {
    owners: Option<[Option<Owner>; 100]>,
    range_declared: bool,
}

impl Ownership {
    pub fn new(op: &Operation) -> Self {
        let [site] = op.declarations.as_slice() else {
            return Self {
                owners: None,
                range_declared: false,
            };
        };
        let StatusInventory::Readable(keys) = &site.status_inventory else {
            return Self {
                owners: None,
                range_declared: false,
            };
        };
        let explicit: BTreeSet<_> = keys
            .iter()
            .filter_map(|key| success_key(key, op.ranged_responses))
            .collect();
        let range_declared = explicit.contains(&SuccessKey::Range);
        let fallback = if range_declared {
            Some(Owner::Key(SuccessKey::Range))
        } else if keys.iter().any(|key| key == "default") {
            Some(Owner::Fallback)
        } else {
            None
        };
        let owners = std::array::from_fn(|i| {
            let code = 200 + i as u16;
            if !can_have_body(&op.id, code) {
                return None;
            }
            let exact = SuccessKey::Exact(code);
            if explicit.contains(&exact) {
                Some(Owner::Key(exact))
            } else {
                fallback
            }
        });
        Self {
            owners: Some(owners),
            range_declared,
        }
    }

    pub fn owns_range(&self) -> bool {
        self.owns(SuccessKey::Range) == Some(true)
    }

    fn owns(&self, key: SuccessKey) -> Option<bool> {
        self.owners
            .map(|owners| owners.contains(&Some(Owner::Key(key))))
    }
}

pub(super) fn range_reassigned(ownership: &[Ownership; 2]) -> [BTreeSet<SuccessKey>; 2] {
    [0, 1].map(|i| {
        let mut exacts = BTreeSet::new();
        if ownership[i].owns_range()
            && ownership[1 - i].owns(SuccessKey::Range) == Some(false)
            && ownership[1 - i].range_declared
        {
            if let [Some(local), Some(other)] = [ownership[i].owners, ownership[1 - i].owners] {
                for (from, to) in local.into_iter().zip(other) {
                    if let (
                        Some(Owner::Key(SuccessKey::Range)),
                        Some(Owner::Key(key @ SuccessKey::Exact(_))),
                    ) = (from, to)
                    {
                        exacts.insert(key);
                    }
                }
            }
        }
        exacts
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    Empty,
    NonJson,
    Json,
    Unknown,
}

fn body(root: &Root<'_, '_>, ownership: &Ownership, key: SuccessKey) -> Body {
    let possible = match key {
        SuccessKey::Exact(code) => can_have_body(&root.op.id, code),
        SuccessKey::Range => root.op.id.method() != "HEAD",
    };
    if !possible || ownership.owns(key) == Some(false) {
        return Body::Empty;
    }
    if ownership.owners.is_none() || root.empty_projection_with_unproved_media() {
        return Body::Unknown;
    }
    match root.message {
        Some(message)
            if message.coverage != Coverage::Withheld && message.json_source.is_some() =>
        {
            Body::Json
        }
        Some(message) if message.coverage == Coverage::Checked => Body::Empty,
        Some(message)
            if message.json_source.is_none()
                && message
                    .limits
                    .iter()
                    .any(|limit| limit.reason == "non_json_media") =>
        {
            Body::NonJson
        }
        _ => Body::Unknown,
    }
}

pub(super) fn select(
    ledger: &mut Ledger,
    ops: [&Operation; 2],
    fields: [&OperationFields<'_>; 2],
    ownership: &[Ownership; 2],
    changed: &[Vec<SuccessKey>; 2],
    uncertain: &[Vec<SuccessKey>; 2],
) -> Option<[SuccessKey; 2]> {
    if changed.iter().any(Vec::is_empty) {
        return None;
    }
    let roots = [0, 1].map(|i| {
        changed[i]
            .iter()
            .map(|key| {
                let root = Root::new(ops[i], fields[i], RootKey::Response(*key));
                let body = body(&root, &ownership[i], *key);
                (root, body, *key)
            })
            .collect::<Vec<_>>()
    });
    if roots
        .iter()
        .flatten()
        .all(|(_, body, _)| *body == Body::Empty)
    {
        return None;
    }
    let candidates = roots.each_ref().map(|roots| {
        roots
            .iter()
            .filter(|(_, body, _)| matches!(body, Body::Json | Body::Unknown))
            .collect::<Vec<_>>()
    });
    let unknown_peer = [0, 1].into_iter().any(|i| {
        uncertain[i].iter().any(|key| {
            let root = Root::new(ops[i], fields[i], RootKey::Response(*key));
            body(&root, &ownership[i], *key) != Body::Empty
        })
    });
    let transfer = match [ownership[0].owners, ownership[1].owners] {
        [Some(b), Some(a)] => b.into_iter().zip(a).any(|(b, a)| {
            let (Some(b), Some(a)) = (b, a) else {
                return false;
            };
            b != a && (b.is_wildcard() || a.is_wildcard())
        }),
        _ => false,
    };
    let reason = if transfer {
        "status_owner_transfer"
    } else if unknown_peer || candidates.iter().any(|roots| roots.len() > 1) {
        "ambiguous_root_pairing"
    } else if let [[(_, Body::Json, before)], [(_, Body::Json, after)]] =
        candidates.each_ref().map(Vec::as_slice)
    {
        return Some([*before, *after]);
    } else if roots
        .iter()
        .flatten()
        .any(|(_, body, _)| matches!(body, Body::Unknown | Body::NonJson))
    {
        "root_media_incomparable"
    } else {
        "root_body_unmatched"
    };
    for (i, roots) in roots.iter().enumerate() {
        for (root, body, _) in roots {
            if *body == Body::Empty || reason == "root_media_incomparable" && *body == Body::Json {
                continue;
            }
            let reason = if reason == "root_media_incomparable"
                && root.empty_projection_with_unproved_media()
            {
                "media_unverified"
            } else {
                reason
            };
            let mut limit =
                ComparisonLimit::new(SIDES[i], root.scope(), ComparisonClaim::RootPairing, reason);
            limit.evidence = root.sites();
            ledger.record(limit);
        }
    }
    None
}
