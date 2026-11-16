//! Private, same-review map selection. Construction preserves the prepared runner's ownership.
//! These views confer no loading authority; inputs and context must belong to the same review.
use super::context_run::{ContextState, ReviewContext};
use super::map_view::MapView;
use super::{AlgorithmOutcome, ReviewInputs};
use prism::finding_confidence::{MinConfidence, ResolutionMode};
use prism::slice::{FileParseQuality, SliceFinding, SliceResult, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(crate) struct ResultView<'a> {
    maps: MapView<'a>,
    context_files: Option<&'a BTreeSet<String>>,
}
impl<'a> ResultView<'a> {
    pub(crate) fn maps(&self) -> MapView<'a> {
        self.maps
    }
    pub(crate) fn sources(&self) -> &'a BTreeMap<String, String> {
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

#[non_exhaustive]
pub(crate) struct ReviewAlgorithmRun {
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
    pub(crate) fn context(&self) -> Option<&ReviewContext> {
        match &self.context {
            ContextState::Expanded(context) if context.view().is_some() => Some(context),
            _ => None,
        }
    }
    pub(crate) fn view<'a>(&'a self, inputs: &'a ReviewInputs) -> ResultView<'a> {
        view_for_result(self.context(), &self.outcome.result, inputs)
    }
    pub(crate) fn filter_findings(
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
