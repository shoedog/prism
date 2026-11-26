//! Cached occurrence inventory and public projections.
use super::*;

#[derive(Clone, Debug, Default)]
pub(in crate::ast) struct Index {
    lines: BTreeMap<usize, Vec<NameRead>>,
    pub(in super::super) facts: Vec<ProtoWrite>,
    refused_reads: BTreeSet<AstSpan>,
    refused_writes: BTreeMap<usize, BTreeSet<AstSpan>>,
    member_writes: BTreeMap<usize, std::sync::Arc<BTreeSet<AstSpan>>>,
    history: BTreeMap<AstSpan, proof::MemberHistory>,
    opaque_calls: BTreeMap<usize, std::sync::Arc<BTreeSet<AstSpan>>>,
    external_stores: BTreeMap<usize, BTreeSet<AstSpan>>,
    #[cfg(test)]
    pub(in super::super) census_work: (usize, usize),
}

impl Index {
    pub(in super::super) fn counts(&self) -> (usize, BTreeMap<&'static str, usize>) {
        let mut complete = 0;
        let mut reasons = BTreeMap::new();
        for read in self.lines.values().flatten() {
            complete += usize::from(read.resolution_complete);
            if let Some(reason) = read.incomplete_reason {
                *reasons.entry(reason).or_default() += 1;
            }
        }
        (complete, reasons)
    }

    // Publish the refusal explicitly. Expanding a shared history on every
    // occurrence in a refused owner's selected line is quadratic.
    pub(in super::super) fn listing_line(
        &self,
        line: usize,
        remaining: &mut usize,
    ) -> Vec<NameRead> {
        let mut reads = self.lines.get(&line).cloned().unwrap_or_default();
        for read in &mut reads {
            let histories = self.histories(read);
            let cost = histories
                .iter()
                .flatten()
                .map(|(sites, _)| sites.len())
                .sum::<usize>();
            if read.candidate_history_refused || cost > *remaining {
                read.candidate_history_refused = true;
                read.reaching_versions.clear();
            } else if cost > 0 {
                *remaining -= cost;
                Self::expand_history(read, histories);
            }
        }
        reads
    }

    pub(in super::super) fn reasons(
        &self,
        line: usize,
    ) -> impl Iterator<Item = (&NameRead, &'static str)> {
        self.lines
            .get(&line)
            .into_iter()
            .flatten()
            .filter_map(|read| read.incomplete_reason.map(|reason| (read, reason)))
    }

    pub(in super::super) fn line(&self, line: usize) -> Vec<NameRead> {
        let mut reads = self.lines.get(&line).cloned().unwrap_or_default();
        for read in &mut reads {
            read.candidate_history_refused = false;
            let histories = self.histories(read);
            Self::expand_history(read, histories);
        }
        reads
    }
    fn histories(&self, read: &NameRead) -> [Option<(&BTreeSet<AstSpan>, usize)>; 4] {
        let Some(binding) = read.binding.as_ref().map(|b| b.start_byte) else {
            return [None; 4];
        };
        let refused = self.refused_reads.contains(&read.span);
        let history = self.history.get(&read.span).copied().unwrap_or_default();
        let member_limit = if refused && read.path != read.base {
            Some(usize::MAX)
        } else {
            history.members
        };
        [
            refused
                .then(|| self.refused_writes.get(&binding))
                .flatten()
                .map(|sites| (sites, usize::MAX)),
            self.member_writes
                .get(&binding)
                .zip(member_limit)
                .map(|(sites, limit)| (sites.as_ref(), limit)),
            self.opaque_calls
                .get(&binding)
                .zip(history.calls)
                .map(|(sites, limit)| (sites.as_ref(), limit)),
            history.aliases.and_then(|(owner, limit)| {
                self.external_stores.get(&owner).map(|sites| (sites, limit))
            }),
        ]
    }

    fn expand_history(read: &mut NameRead, histories: [Option<(&BTreeSet<AstSpan>, usize)>; 4]) {
        if histories.iter().any(Option::is_some) {
            let sites = histories.into_iter().flatten().flat_map(|(sites, limit)| {
                sites
                    .iter()
                    .take_while(move |site| site.start_byte <= limit)
                    .filter(move |site| site.end_byte <= limit)
            });
            read.reaching_versions = read
                .reaching_versions
                .iter()
                .chain(sites)
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
        }
    }
}

impl Analysis<'_> {
    pub(in super::super) fn index(&self, facts: Vec<ProtoWrite>) -> Index {
        let mut lines = BTreeMap::<usize, Vec<NameRead>>::new();
        let mut refused_reads = BTreeSet::new();
        let mut history = BTreeMap::new();
        let selectors: BTreeSet<_> = facts
            .iter()
            .flat_map(|fact| &fact.key_operands)
            .filter(|operand| operand.role == super::super::KeyRole::Selector)
            .map(|operand| (operand.start_byte, operand.end_byte))
            .collect();
        for site in self.sites.values().filter(|site| {
            site.visible
                || selectors.contains(&(site.read.span.start_byte, site.read.span.end_byte))
        }) {
            let mut read = site.read.clone();
            if self.effects.direct_eval || self.effects.skipped.contains(&site.owner) {
                read.resolution_complete = false;
                read.incomplete_reason = None;
                refused_reads.insert(read.span.clone());
                read.candidate_history_refused = self.effects.skipped.contains(&site.owner);
            }
            let mut member_history = site.member_history;
            if site.widened_member {
                member_history.members = Some(usize::MAX);
            }
            if member_history.members.is_some()
                || member_history.calls.is_some()
                || member_history.aliases.is_some()
            {
                history.insert(read.span.clone(), member_history);
            }
            lines.entry(site.line).or_default().push(read);
        }
        let mut extra = BTreeMap::new();
        for key in facts.iter().flat_map(|fact| &fact.key_operands) {
            let span = AstSpan {
                start_byte: key.start_byte,
                end_byte: key.end_byte,
            };
            if !self.sites.contains_key(&span) {
                extra.entry(span).or_insert(key);
            }
        }
        for (span, key) in extra {
            lines.entry(key.line).or_default().push(NameRead {
                path: key.name.clone(),
                base: key.base.clone(),
                span,
                owner: key.owner.clone(),
                binding: None,
                reaching_versions: Vec::new(),
                resolution_complete: false,
                incomplete_reason: None,
                candidate_history_refused: false,
            });
        }
        for reads in lines.values_mut() {
            reads.sort_by(|left, right| left.span.cmp(&right.span));
        }
        let refused_writes = if refused_reads.is_empty() {
            BTreeMap::new()
        } else {
            self.effects.known.clone()
        };
        Index {
            lines,
            facts,
            refused_reads,
            refused_writes,
            history,
            opaque_calls: self.effects.proof.opaque_calls.clone(),
            external_stores: self.effects.proof.external_stores.clone(),
            member_writes: self.effects.member_writes.clone(),
            #[cfg(test)]
            census_work: (self.effects.proof.work, self.effects.proof.rhs_nodes),
        }
    }
}
