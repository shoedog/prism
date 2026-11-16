//! One captured review, one optional expansion, and one RAW cached result.
use crate::algorithms::taint::{
    context_budget::{AttemptCounters, ContextLimits, Published, TimeGuard},
    frontier::FrontierState,
    result_union::{union_results, UnionError, UnionInput, UnionStats},
    rounds::{expand, ExpansionOutcome, ExpansionRequest, RoundStats, Stop},
    TaintConfig, TaintTrace,
};
use prism::api::{ContextUnavailable, ReviewContextLoader, ReviewInputs};
use prism::ast::ParsedFile;
use prism::cpg::{CpgContext, CrossingCeiling};
use prism::slice::{FileParseQuality, SliceResult, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct ContextRequest {
    pub(crate) limits: ContextLimits,
    pub(crate) ceiling: CrossingCeiling,
}
#[derive(Debug)]
pub(crate) enum UnionState {
    NotNeeded,
    Applied {
        stats: UnionStats,
        added_block_ids: Vec<usize>,
    },
    Refused(UnionError),
}
pub(crate) struct ReviewContext {
    pub(crate) files: BTreeMap<String, ParsedFile>,
    pub(crate) sources: BTreeMap<String, String>,
    pub(crate) parse_quality: BTreeMap<String, FileParseQuality>,
    pub(crate) parse_warnings: Vec<String>,
    pub(crate) context_files: BTreeSet<String>,
    pub(crate) round: u8,
    pub(crate) published: Published,
    pub(crate) stats: RoundStats,
    pub(crate) stop: Stop,
    pub(crate) attempts: AttemptCounters,
    pub(crate) frontier: FrontierState,
    pub(crate) union: UnionState,
}
impl ReviewContext {
    pub(crate) fn view(&self) -> Option<super::map_view::MapView<'_>> {
        if self.context_files.is_empty() || !matches!(self.union, UnionState::Applied { .. }) {
            return None;
        }
        Some(super::map_view::MapView::new(
            &self.files,
            &self.sources,
            &self.parse_quality,
        ))
    }
    pub(crate) fn union_stats(&self) -> Option<&UnionStats> {
        match &self.union {
            UnionState::Applied { stats, .. } => Some(stats),
            _ => None,
        }
    }
    pub(crate) fn added_block_ids(&self) -> &[usize] {
        match &self.union {
            UnionState::Applied {
                added_block_ids, ..
            } => added_block_ids,
            _ => &[],
        }
    }
}
pub(crate) enum ContextState {
    NotRequested,
    Unavailable(ContextUnavailable),
    Expanded(ReviewContext),
}
impl ContextState {
    pub(crate) fn view<'a>(
        &'a self,
        inputs: &'a ReviewInputs,
        algorithm: SlicingAlgorithm,
    ) -> super::map_view::MapView<'a> {
        if algorithm == SlicingAlgorithm::Taint {
            if let Self::Expanded(context) = self {
                if let Some(view) = context.view() {
                    return view;
                }
            }
        }
        super::map_view::MapView::original(inputs)
    }
}
pub(super) struct RunExpansion<'a> {
    request: Option<&'a ContextRequest>,
    cached: Option<SliceResult>,
    state: ContextState,
}
impl<'a> RunExpansion<'a> {
    pub(super) fn new(request: Option<&'a ContextRequest>) -> Self {
        Self {
            request,
            cached: None,
            state: ContextState::NotRequested,
        }
    }
    pub(super) fn requested(&self) -> bool {
        self.request.is_some()
    }
    pub(super) fn cached(&self) -> Option<SliceResult> {
        self.cached.clone()
    }
    pub(super) fn apply(
        &mut self,
        inputs: &ReviewInputs,
        ctx: &CpgContext<'_>,
        config: &TaintConfig,
        baseline: SliceResult,
        trace: &TaintTrace,
    ) -> SliceResult {
        let Some(request) = self.request else {
            return baseline;
        };
        if let Some(result) = self.cached() {
            return result;
        }
        let loader = match ReviewContextLoader::new(inputs) {
            Ok(loader) => loader,
            Err(unavailable) => {
                self.state = ContextState::Unavailable(unavailable);
                self.cached = Some(baseline.clone());
                return baseline;
            }
        };
        // All borrows come from this one captured review; repo is never an authority.
        let guard = TimeGuard::new(request.limits.time);
        let outcome = expand(ExpansionRequest {
            loader: &loader,
            inputs,
            baseline_ctx: ctx,
            baseline_trace: trace,
            config,
            limits: request.limits,
            ceiling: request.ceiling,
            guard: &guard,
        });
        let original = inputs.files.keys().cloned().collect();
        let (result, context) = complete(baseline, outcome, &original);
        self.state = ContextState::Expanded(context);
        self.cached = Some(result.clone());
        result
    }
    pub(super) fn view<'b>(
        &'b self,
        inputs: &'b ReviewInputs,
        algorithm: SlicingAlgorithm,
    ) -> super::map_view::MapView<'b> {
        self.state.view(inputs, algorithm)
    }
    pub(super) fn into_state(self) -> ContextState {
        self.state
    }
}
fn complete(
    baseline: SliceResult,
    outcome: ExpansionOutcome,
    original: &BTreeSet<String>,
) -> (SliceResult, ReviewContext) {
    let bundle = outcome.bundle;
    let (result, union) = match bundle.analysis.as_ref() {
        Some(analysis) if !bundle.context_files.is_empty() => {
            match union_results(UnionInput {
                baseline: &baseline,
                expanded: &analysis.result,
                attribution: &analysis.emission,
                original_files: original,
                context_files: &bundle.context_files,
            }) {
                Ok(union) => (
                    union.result,
                    UnionState::Applied {
                        stats: union.stats,
                        added_block_ids: union.added_block_ids,
                    },
                ),
                Err(error) => (baseline, UnionState::Refused(error)),
            }
        }
        _ => (baseline, UnionState::NotNeeded),
    };
    (
        result,
        ReviewContext {
            files: bundle.files,
            sources: bundle.sources,
            parse_quality: bundle.parse_quality,
            parse_warnings: bundle.parse_warnings,
            context_files: bundle.context_files,
            round: bundle.round,
            published: bundle.published,
            stats: outcome.stats,
            stop: outcome.stop,
            attempts: outcome.attempts,
            frontier: bundle.frontier,
            union,
        },
    )
}
#[cfg(test)]
#[path = "context_run_tests.rs"]
mod tests;
