//! Advisory import targets and exact call argument spans for review consumers.

use super::ReviewInputs;
use crate::call_graph::CallSite;
use crate::review_context::import_targets::{Attribution, Basis, ImportTargets, Why};
use std::ops::Range;

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

/// Reuses a review's captured listing and root. Go declaration hints may read
/// listed files live; this handle neither rewalks nor parses candidate files.
#[non_exhaustive]
pub struct ReviewBoundaryHints<'a> {
    inputs: &'a ReviewInputs,
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
        Ok(Self { inputs, targets })
    }

    pub fn hint_for_call(&mut self, call: &CallSite) -> Option<ReviewImportHint> {
        let parsed = self.inputs.files.get(&call.caller.file)?;
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
        let Some(parsed) = self.inputs.files.get(&call.caller.file) else {
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
