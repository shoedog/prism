//! Same-review map selection. Construction preserves the prepared runner's ownership.
//! These views confer no loading authority; inputs and context must belong to the same review.
use super::context_run::{ContextState, ReviewContext};
use super::map_view::MapView;
use super::{AlgorithmOutcome, ReviewInputs};
use prism::finding_confidence::{MinConfidence, ResolutionMode};
use prism::slice::{FileParseQuality, SliceFinding, SliceResult, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

/// Borrowed maps selected for one result or finding; use inputs from the same review.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct ResultView<'a> {
    maps: MapView<'a>,
    context_files: Option<&'a BTreeSet<String>>,
}
impl<'a> ResultView<'a> {
    /// Select completed context only for a taint result, rechecking eligibility.
    pub fn for_result(
        context: Option<&'a ReviewContext>,
        result: &SliceResult,
        inputs: &'a ReviewInputs,
    ) -> Self {
        view_for_result(context, result, inputs)
    }
    /// Select completed context only for a taint finding, rechecking eligibility.
    pub fn for_finding(
        context: Option<&'a ReviewContext>,
        finding: &SliceFinding,
        inputs: &'a ReviewInputs,
    ) -> Self {
        view_for_finding(context, finding, inputs)
    }
    pub(crate) fn context_files(&self) -> Option<&'a BTreeSet<String>> {
        self.context_files
    }

    pub(crate) fn maps(&self) -> MapView<'a> {
        self.maps
    }
    pub fn sources(&self) -> &'a BTreeMap<String, String> {
        self.maps.sources
    }
    pub(crate) fn context_parse_quality(&self) -> BTreeMap<String, FileParseQuality> {
        self.maps
            .parse_quality
            .iter()
            .filter(|(file, _)| {
                self.context_files
                    .is_some_and(|files| files.contains(*file))
            })
            .map(|(file, quality)| (file.clone(), quality.clone()))
            .collect()
    }
}

pub(crate) fn view_for_result<'a>(
    context: Option<&'a ReviewContext>,
    result: &SliceResult,
    inputs: &'a ReviewInputs,
) -> ResultView<'a> {
    select(context, result.algorithm == SlicingAlgorithm::Taint, inputs)
}
pub(crate) fn view_for_finding<'a>(
    context: Option<&'a ReviewContext>,
    finding: &SliceFinding,
    inputs: &'a ReviewInputs,
) -> ResultView<'a> {
    select(context, finding.algorithm == "taint", inputs)
}
fn select<'a>(
    context: Option<&'a ReviewContext>,
    taint: bool,
    inputs: &'a ReviewInputs,
) -> ResultView<'a> {
    if taint {
        // Recheck the gate for direct callers, including refused nonempty bundles.
        if let Some(context) = context.filter(|context| context.view().is_some()) {
            return ResultView {
                maps: MapView::new(&context.files, &context.sources, &context.parse_quality),
                context_files: Some(&context.context_files),
            };
        }
    }
    ResultView {
        maps: MapView::original(inputs),
        context_files: None,
    }
}

/// Single-algorithm outcome with its eligible same-review context, if any.
#[non_exhaustive]
pub struct ReviewAlgorithmRun {
    pub(crate) outcome: AlgorithmOutcome,
    context: ContextState,
    context_stats: Option<super::context_stats::ContextStats>,
}
impl ReviewAlgorithmRun {
    pub(crate) fn new(outcome: AlgorithmOutcome, context: ContextState) -> Self {
        Self {
            outcome,
            context,
            context_stats: None,
        }
    }
    pub(crate) fn with_stats(mut self, stats: Option<super::context_stats::ContextStats>) -> Self {
        self.context_stats = stats;
        self
    }
    pub(crate) fn stats(&self) -> Option<&super::context_stats::ContextStats> {
        self.context_stats.as_ref()
    }
    pub fn context(&self) -> Option<&ReviewContext> {
        match &self.context {
            ContextState::Expanded(context) if context.view().is_some() => Some(context),
            _ => None,
        }
    }
    /// Consume the owner, retaining the outcome and only eligible context.
    /// Private execution statistics are consumed with this owner, not exposed in the pair.
    pub fn into_parts(self) -> (AlgorithmOutcome, Option<ReviewContext>) {
        let eligible = self.context().is_some();
        let context = match self.context {
            ContextState::Expanded(context) if eligible => Some(context),
            _ => None,
        };
        (self.outcome, context)
    }
    pub fn view<'a>(&'a self, inputs: &'a ReviewInputs) -> ResultView<'a> {
        view_for_result(self.context(), &self.outcome.result, inputs)
    }
    /// Apply the existing confidence policy with the selected same-review maps.
    pub fn filter_findings(
        &mut self,
        inputs: &ReviewInputs,
        min_confidence: MinConfidence,
        resolution: ResolutionMode,
    ) {
        let context = match &self.context {
            ContextState::Expanded(context) => Some(context),
            _ => None,
        };
        let view = view_for_result(context, &self.outcome.result, inputs);
        super::run::filter_result_findings_in(
            &mut self.outcome.result,
            view.maps(),
            min_confidence,
            resolution,
        );
    }
}

#[cfg(test)]
#[path = "context_view_tests.rs"]
mod tests;
