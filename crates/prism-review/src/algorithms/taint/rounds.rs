//! Private completed-round expansion. The caller constructs all inputs from one captured review.
//! This invariant is not checked by an identity API. Panics are not caught; only errors become stops.
use super::context_budget::{
    AdmissionBound, AttemptCounters, ContextLimits, Published, RunGuard, StopCause,
};
use super::frontier::FrontierState;
use super::source_scope::admission::ScopedTaintOutcome;
use super::{TaintConfig, TaintTrace};
use prism::api::{ReviewContextLoader, ReviewInputs};
use prism::ast::ParsedFile;
use prism::cpg::{CpgContext, CrossingCeiling};
use prism::slice::FileParseQuality;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct ExpansionRequest<'a> {
    pub(crate) loader: &'a ReviewContextLoader<'a>,
    pub(crate) inputs: &'a ReviewInputs,
    pub(crate) baseline_ctx: &'a CpgContext<'a>,
    pub(crate) baseline_trace: &'a TaintTrace,
    pub(crate) config: &'a TaintConfig,
    pub(crate) limits: ContextLimits,
    pub(crate) ceiling: CrossingCeiling,
    pub(crate) guard: &'a dyn RunGuard,
}
pub(crate) struct CompletedBundle {
    pub(crate) round: u8,
    pub(crate) files: BTreeMap<String, ParsedFile>,
    pub(crate) sources: BTreeMap<String, String>,
    pub(crate) parse_quality: BTreeMap<String, FileParseQuality>,
    pub(crate) parse_warnings: Vec<String>,
    pub(crate) context_files: BTreeSet<String>,
    pub(crate) published: Published,
    pub(crate) analysis: Option<ScopedTaintOutcome>,
    pub(crate) frontier: FrontierState,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    FrontierExhausted,
    DepthReached,
    Bound(AdmissionBound),
    Guard(StopCause),
    StageFailed(String),
}
#[derive(Debug, Default)]
pub(crate) struct RoundStats {
    pub(crate) rounds_started: u8,
    pub(crate) rounds_completed: u8,
    pub(crate) elapsed_discovery_ns: u64,
    pub(crate) elapsed_read_parse_ns: u64,
    pub(crate) elapsed_build_ns: u64,
    pub(crate) elapsed_analysis_ns: u64,
    pub(crate) elapsed_total_ns: u64,
}
fn elapsed_ns(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
pub(crate) struct ExpansionOutcome {
    pub(crate) bundle: CompletedBundle,
    pub(crate) stop: Stop,
    pub(crate) attempts: AttemptCounters,
    pub(crate) stats: RoundStats,
}
mod driver;
mod stage;
pub(crate) use driver::expand;
#[cfg(test)]
mod tests;
