//! The one gate that decides a claim's status.
//! `ClaimStatus` has no public constructor: only `ClaimDecision::from_reasons`
//! builds it, and it admits a claim only when no reason remains and every
//! required dependency is itself admitted.

use super::{ClaimDecision, Refusal};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Admitted,
    Unchecked,
}

/// Typed claim status (serialized as `"admitted"` or `"unchecked"`).
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ClaimStatus(Kind);

impl ClaimStatus {
    fn decide(admitted: bool) -> Self {
        Self(if admitted {
            Kind::Admitted
        } else {
            Kind::Unchecked
        })
    }

    pub fn is_admitted(self) -> bool {
        self.0 == Kind::Admitted
    }
}

impl PartialEq<&str> for ClaimStatus {
    fn eq(&self, other: &&str) -> bool {
        match self.0 {
            Kind::Admitted => *other == "admitted",
            Kind::Unchecked => *other == "unchecked",
        }
    }
}

pub(crate) struct RequiredDependency<'a> {
    id: &'a str,
    decision: Option<&'a ClaimDecision>,
    refusal_code: Option<&'static str>,
}

impl<'a> RequiredDependency<'a> {
    pub(crate) fn new(id: &'a str, decision: Option<&'a ClaimDecision>) -> Self {
        Self {
            id,
            decision,
            refusal_code: None,
        }
    }

    pub(crate) fn with_refusal_code(mut self, code: &'static str) -> Self {
        self.refusal_code = Some(code);
        self
    }
}

impl ClaimDecision {
    // The edges are derived, not listed: a claim that reads an input depends on that
    // input's settled selection claim (`Ledger::selection_claim`, including the
    // subject head in a historical review), and `ClaimStatus` is built only below.
    // Required admission edges: claim -> input, slot and declared receipt roots;
    // receipt -> origin input and declared dependencies (including config, references,
    // derived records and their media proofs); computed tree -> merge receipts and
    // historical exact-tree evidence. The receipt walk checks raw records, and
    // schema admission also passes the settled decisions of required media proofs.
    // Direct schema media attachments are optional unless another required edge
    // reaches them. Their own dependencies are always required for the media claim.
    //
    // Frozen edges: input -> declared input/config dependencies; slot and reference
    // -> named inputs; cutoff -> its subject, computed result and selection inputs;
    // comparison -> cutoff and both inputs; usage and review action -> referenced
    // comparisons; binding status -> selected and linked scan inputs; call changes
    // and membership -> their named inputs. Absent inapplicable links are optional.
    // Coverage backlinks and row/occurrence references within those records are
    // validated for integrity, and do not certify the linked content.
    pub(crate) fn from_reasons<'a>(
        input_id: Option<&str>,
        mut reasons: Vec<Refusal>,
        dependencies: impl IntoIterator<Item = RequiredDependency<'a>>,
    ) -> Self {
        let mut unadmitted = dependencies.into_iter().filter(|dependency| {
            dependency
                .decision
                .is_none_or(|decision| !decision.is_admitted())
        });
        let first = unadmitted.next();
        let blocked = first.is_some();
        if let Some(dependency) = &first {
            reasons.insert(0, Refusal::new("missing_dependency", dependency.id));
        }
        for dependency in first.into_iter().chain(unadmitted) {
            if let Some(code) = dependency.refusal_code {
                reasons.push(Refusal::new(code, dependency.id));
            } else if let Some(decision) = dependency.decision {
                reasons.extend(decision.reasons.iter().cloned());
            }
        }
        let mut by_code: BTreeMap<String, Refusal> = BTreeMap::new();
        for refusal in reasons {
            let existing = by_code
                .entry(refusal.code.clone())
                .or_insert_with(|| refusal.clone());
            existing.merge(refusal);
        }
        // Preserve the causal refusal as the primary reason when adding an edge diagnostic.
        let dependency_reason = blocked
            .then(|| by_code.remove("missing_dependency"))
            .flatten();
        let admitted = by_code.is_empty() && dependency_reason.is_none();
        Self {
            status: ClaimStatus::decide(admitted),
            reasons: by_code.into_values().chain(dependency_reason).collect(),
            input_id: input_id.filter(|_| admitted).map(str::to_owned),
            requested_scope: None,
            effective_scope: None,
            selection: None,
            uncovered_scope: None,
            dirty: None,
            media_root: None,
            evidence: Vec::new(),
        }
    }

    pub(crate) fn is_admitted(&self) -> bool {
        self.status.is_admitted()
    }
}
