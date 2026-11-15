//! Advisory import targets and exact call argument spans for review consumers.

use super::ReviewInputs;
use crate::ast::ParsedFile;
use crate::call_graph::{CallSite, ScopeGraphBuildInputs};
use crate::review_context::import_targets::{Attribution, Basis, ImportTargets, Why};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewHintBasis {
    Bound,
    Candidates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewUnresolvedReason {
    NoFile,
    TooMany,
    ReplaceUnproven,
}

/// Loading advice only; neither variant establishes call resolution or flow.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewImportHint {
    InRepo {
        files: Vec<String>,
        spec: String,
        member: Option<String>,
        declaring: Vec<String>,
        basis: ReviewHintBasis,
    },
    Unresolvable {
        spec: String,
        why: ReviewUnresolvedReason,
    },
}

/// One re-export step through a barrel module. Loading advice, never an edge or a proof.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReviewBarrelHop {
    pub file: String,
    pub member: String,
}

/// Reuses a review's captured listing and root. Go declaration hints may read
/// listed files live; this handle neither rewalks nor parses candidate files.
#[non_exhaustive]
pub struct ReviewBoundaryHints<'a> {
    files: &'a BTreeMap<String, ParsedFile>,
    targets: ImportTargets<'a>,
}

impl<'a> ReviewBoundaryHints<'a> {
    pub fn new(inputs: &'a ReviewInputs) -> anyhow::Result<Self> {
        let targets = ImportTargets::new(
            &inputs.scope_graph_inputs,
            &inputs.source_listing,
            inputs.files.keys().cloned().collect(),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))?;
        Ok(Self {
            files: &inputs.files,
            targets,
        })
    }

    /// The round's constructor: caller lookup and loaded identities come from the one
    /// `files` map, the scope inputs and listing are the round's own.
    pub(super) fn for_round(
        files: &'a BTreeMap<String, ParsedFile>,
        scope: &'a ScopeGraphBuildInputs,
        listing: &Arc<BTreeSet<String>>,
    ) -> anyhow::Result<Self> {
        let listing = Ok(Arc::clone(listing));
        let targets = ImportTargets::new(scope, &listing, files.keys().cloned().collect())
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        Ok(Self { files, targets })
    }

    /// One re-export step of `member` through `barrel`, read from the barrel's parse in this
    /// handle's own file map (`None` when the barrel is not in it; never a disk read).
    /// One hop per call, not filtered against loaded files: the caller owns cycles and budgets.
    pub fn follow_barrel(&mut self, barrel: &str, member: &str) -> Option<ReviewBarrelHop> {
        let parsed = self.files.get(barrel)?;
        let hop = self.targets.follow_barrel(barrel, parsed, member)?;
        Some(ReviewBarrelHop {
            file: hop.file,
            member: hop.member,
        })
    }

    pub fn hint_for_call(&mut self, call: &CallSite) -> Option<ReviewImportHint> {
        let parsed = self.files.get(&call.caller.file)?;
        match self
            .targets
            .targets_for_call(&call.caller.file, parsed, call)
        {
            Attribution::InRepo(hit) => Some(ReviewImportHint::InRepo {
                files: hit.files,
                spec: hit.spec,
                member: hit.member,
                declaring: hit.declaring,
                basis: match hit.basis {
                    Basis::Bound => ReviewHintBasis::Bound,
                    Basis::Candidates => ReviewHintBasis::Candidates,
                },
            }),
            Attribution::Unresolvable { spec, why } => Some(ReviewImportHint::Unresolvable {
                spec,
                why: match why {
                    Why::NoFile => ReviewUnresolvedReason::NoFile,
                    Why::TooMany => ReviewUnresolvedReason::TooMany,
                    Why::ReplaceUnproven => ReviewUnresolvedReason::ReplaceUnproven,
                },
            }),
            _ => None,
        }
    }

    /// Half-open argument-expression ranges in source order, including existing
    /// Python keyword/splat spans. These do not establish parameter bindings.
    pub fn argument_spans(&self, call: &CallSite) -> Vec<Range<usize>> {
        let Some(parsed) = self.files.get(&call.caller.file) else {
            return Vec::new();
        };
        parsed
            .call_argument_texts_and_spans_at(
                call.start_byte,
                call.source_callee_name
                    .as_deref()
                    .unwrap_or(&call.callee_name),
            )
            .into_iter()
            .map(|(_, span)| span)
            .filter(|span| call.start_byte <= span.start && span.end <= call.end_byte)
            .collect()
    }
}

#[cfg(test)]
#[path = "review_context_tests.rs"]
mod tests;
