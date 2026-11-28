use super::*;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Ledger {
    records: BTreeMap<LimitKey, ComparisonLimit>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct LimitKey {
    side: ComparisonSide,
    scope: LimitScope,
    pointer: Vec<PointerToken>,
    claim: ComparisonClaim,
    reason: String,
}

impl Ledger {
    pub fn record(&mut self, limit: ComparisonLimit) {
        let key = LimitKey {
            side: limit.side,
            scope: limit.scope.clone(),
            pointer: limit.pointer.clone(),
            claim: limit.claim.clone(),
            reason: limit.reason.clone(),
        };
        match self.records.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(limit);
            }
            Entry::Occupied(mut entry) => {
                let prior = entry.get_mut();
                if prior.origin != limit.origin {
                    prior.origin = LimitOrigin::Both;
                }
                prior.evidence.extend(limit.evidence);
                prior.evidence.sort();
                prior.evidence.dedup();
                for site in limit.producer_sites {
                    if !prior.producer_sites.contains(&site) {
                        prior.producer_sites.push(site);
                    }
                }
                if prior.presence.is_none() {
                    prior.presence = limit.presence;
                }
            }
        }
    }

    pub fn finish(self, diff: &mut SchemaDiff<'_>) {
        diff.limits = self.records.into_values().collect();
        diff.limit_counts.before = diff
            .limits
            .iter()
            .take_while(|limit| limit.side == ComparisonSide::Before)
            .count();
        diff.limit_counts.after = diff.limits.len() - diff.limit_counts.before;
    }
}

impl ComparisonLimit {
    pub(super) fn new(
        side: ComparisonSide,
        scope: LimitScope,
        claim: ComparisonClaim,
        reason: &str,
    ) -> Self {
        Self {
            origin: LimitOrigin::Comparison,
            side,
            scope,
            claim,
            reason: reason.into(),
            pointer: Vec::new(),
            presence: None,
            evidence: Vec::new(),
            producer_sites: Vec::new(),
        }
    }

    pub(super) fn with_inventory_origin(mut self) -> Self {
        self.origin = LimitOrigin::InputInventory;
        self
    }

    pub(super) fn with_presence(mut self, [before, after]: &[ElementPresence; 2]) -> Self {
        self.presence = Some(PresenceComparison {
            before: before.clone(),
            after: after.clone(),
        });
        self
    }
}
