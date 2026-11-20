//! Private request-scoped context accounting. Attempted bytes are targeted context reads, not all I/O.
use super::context_run::ReviewContext;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ContextStats {
    pub(crate) rounds_started: u64,
    pub(crate) rounds_completed: u64,
    pub(crate) attempted_files: u64,
    pub(crate) attempted_read_bytes: u64,
    pub(crate) admitted_files: u64,
    pub(crate) admitted_bytes: u64,
    pub(crate) admitted_depth: u64,
    pub(crate) eligible_boundary_calls: u64,
    pub(crate) loaded_unresolved_calls: u64,
    pub(crate) skips_invalid_path: u64,
    pub(crate) skips_not_listed: u64,
    pub(crate) skips_unsupported: u64,
    pub(crate) skips_original_file: u64,
    pub(crate) skips_unsafe_path: u64,
    pub(crate) skips_not_regular: u64,
    pub(crate) skips_unreadable: u64,
    pub(crate) skips_not_utf8: u64,
    pub(crate) skips_too_large: u64,
    pub(crate) skips_parse_failed: u64,
    pub(crate) skips_severe_parse: u64,
    pub(crate) skips_unknown: u64,
    pub(crate) bound_depth: u64,
    pub(crate) bound_files: u64,
    pub(crate) bound_bytes: u64,
    pub(crate) bound_time: u64,
    pub(crate) stage_failed: u64,
    pub(crate) cancelled: u64,
    pub(crate) additional_sinks: u64,
    pub(crate) additional_sources: u64,
    pub(crate) perturbation_dropped_sinks: u64,
    pub(crate) perturbation_dropped_sources: u64,
    pub(crate) elapsed_discovery_ns: u64,
    pub(crate) elapsed_read_parse_ns: u64,
    pub(crate) elapsed_build_ns: u64,
    pub(crate) elapsed_analysis_ns: u64,
    pub(crate) elapsed_total_ns: u64,
    pub(crate) union_stats_available: u64,
    pub(crate) driver_stats_available: u64,
    pub(crate) context_unavailable: u64,
}

impl ContextStats {
    pub(crate) fn from_context(context: &ReviewContext) -> Self {
        use crate::algorithms::taint::{
            context_budget::{AdmissionBound, SkipReason, StopCause},
            frontier::Disposition,
            rounds::Stop,
        };
        let mut result = Self {
            rounds_started: u64::from(context.stats.rounds_started),
            rounds_completed: u64::from(context.stats.rounds_completed),
            attempted_files: context.attempts.attempted_files,
            attempted_read_bytes: context.attempts.attempted_read_bytes,
            admitted_files: u64::try_from(context.published.files).unwrap_or(u64::MAX),
            admitted_bytes: context.published.bytes,
            admitted_depth: u64::from(context.round),
            eligible_boundary_calls: u64::try_from(context.frontier.provenance.len())
                .unwrap_or(u64::MAX),
            elapsed_discovery_ns: context.stats.elapsed_discovery_ns,
            elapsed_read_parse_ns: context.stats.elapsed_read_parse_ns,
            elapsed_build_ns: context.stats.elapsed_build_ns,
            elapsed_analysis_ns: context.stats.elapsed_analysis_ns,
            elapsed_total_ns: context.stats.elapsed_total_ns,
            driver_stats_available: 1,
            ..Self::default()
        };
        let mut sites = std::collections::BTreeMap::new();
        for (index, _, disposition) in &context.frontier.dispositions {
            let Some(provenance) = context.frontier.provenance.get(*index) else {
                continue;
            };
            let (resolved, unresolved) = sites.entry(&provenance.site).or_insert((false, false));
            *resolved |= *disposition == Disposition::Resolved;
            *unresolved |= *disposition == Disposition::LoadedUnresolved;
        }
        result.loaded_unresolved_calls = u64::try_from(
            sites
                .values()
                .filter(|(resolved, unresolved)| !resolved && *unresolved)
                .count(),
        )
        .unwrap_or(u64::MAX);
        for (_, reason) in &context.attempts.skips {
            let slot = match reason {
                SkipReason::InvalidPath => &mut result.skips_invalid_path,
                SkipReason::NotListed => &mut result.skips_not_listed,
                SkipReason::Unsupported => &mut result.skips_unsupported,
                SkipReason::OriginalFile => &mut result.skips_original_file,
                SkipReason::UnsafePath => &mut result.skips_unsafe_path,
                SkipReason::NotRegular => &mut result.skips_not_regular,
                SkipReason::Unreadable { .. } => &mut result.skips_unreadable,
                SkipReason::NotUtf8 => &mut result.skips_not_utf8,
                SkipReason::TooLarge { .. } => &mut result.skips_too_large,
                SkipReason::ParseFailed { .. } => &mut result.skips_parse_failed,
                SkipReason::SevereParse { .. } => &mut result.skips_severe_parse,
                SkipReason::Unknown => &mut result.skips_unknown,
            };
            *slot = slot.saturating_add(1);
        }
        match &context.stop {
            Stop::DepthReached => result.bound_depth = 1,
            Stop::Bound(AdmissionBound::Files { .. }) => result.bound_files = 1,
            Stop::Bound(AdmissionBound::SourceBytes { .. }) => result.bound_bytes = 1,
            Stop::Guard(StopCause::Time) => result.bound_time = 1,
            Stop::Guard(StopCause::Cancelled) => result.cancelled = 1,
            Stop::StageFailed(_) => result.stage_failed = 1,
            Stop::FrontierExhausted => {}
        }
        if let Some(stats) = context.union_stats() {
            result.union_stats_available = 1;
            result.additional_sinks = u64::try_from(stats.additional_sinks).unwrap_or(u64::MAX);
            result.additional_sources = u64::try_from(stats.additional_sources).unwrap_or(u64::MAX);
            result.perturbation_dropped_sinks =
                u64::try_from(stats.dropped_sinks).unwrap_or(u64::MAX);
            result.perturbation_dropped_sources =
                u64::try_from(stats.dropped_sources).unwrap_or(u64::MAX);
        }
        result
    }
    pub(crate) fn rows(&self) -> [(&'static str, u64); 39] {
        [
            ("review.taint_context.rounds_started", self.rounds_started),
            (
                "review.taint_context.rounds_completed",
                self.rounds_completed,
            ),
            ("review.taint_context.attempted_files", self.attempted_files),
            (
                "review.taint_context.attempted_read_bytes",
                self.attempted_read_bytes,
            ),
            ("review.taint_context.admitted_files", self.admitted_files),
            ("review.taint_context.admitted_bytes", self.admitted_bytes),
            ("review.taint_context.admitted_depth", self.admitted_depth),
            (
                "review.taint_context.eligible_boundary_calls",
                self.eligible_boundary_calls,
            ),
            (
                "review.taint_context.loaded_unresolved_calls",
                self.loaded_unresolved_calls,
            ),
            (
                "review.taint_context.skips.invalid_path",
                self.skips_invalid_path,
            ),
            (
                "review.taint_context.skips.not_listed",
                self.skips_not_listed,
            ),
            (
                "review.taint_context.skips.unsupported",
                self.skips_unsupported,
            ),
            (
                "review.taint_context.skips.original_file",
                self.skips_original_file,
            ),
            (
                "review.taint_context.skips.unsafe_path",
                self.skips_unsafe_path,
            ),
            (
                "review.taint_context.skips.not_regular",
                self.skips_not_regular,
            ),
            (
                "review.taint_context.skips.unreadable",
                self.skips_unreadable,
            ),
            ("review.taint_context.skips.not_utf8", self.skips_not_utf8),
            ("review.taint_context.skips.too_large", self.skips_too_large),
            (
                "review.taint_context.skips.parse_failed",
                self.skips_parse_failed,
            ),
            (
                "review.taint_context.skips.severe_parse",
                self.skips_severe_parse,
            ),
            ("review.taint_context.skips.unknown", self.skips_unknown),
            ("review.taint_context.bound_depth", self.bound_depth),
            ("review.taint_context.bound_files", self.bound_files),
            ("review.taint_context.bound_bytes", self.bound_bytes),
            ("review.taint_context.bound_time", self.bound_time),
            ("review.taint_context.stage_failed", self.stage_failed),
            ("review.taint_context.cancelled", self.cancelled),
            (
                "review.taint_context.additional_sinks",
                self.additional_sinks,
            ),
            (
                "review.taint_context.additional_sources",
                self.additional_sources,
            ),
            (
                "review.taint_context.perturbation_dropped_sinks",
                self.perturbation_dropped_sinks,
            ),
            (
                "review.taint_context.perturbation_dropped_sources",
                self.perturbation_dropped_sources,
            ),
            (
                "review.taint_context.elapsed_discovery_ns",
                self.elapsed_discovery_ns,
            ),
            (
                "review.taint_context.elapsed_read_parse_ns",
                self.elapsed_read_parse_ns,
            ),
            (
                "review.taint_context.elapsed_build_ns",
                self.elapsed_build_ns,
            ),
            (
                "review.taint_context.elapsed_analysis_ns",
                self.elapsed_analysis_ns,
            ),
            (
                "review.taint_context.elapsed_total_ns",
                self.elapsed_total_ns,
            ),
            (
                "review.taint_context.union_stats_available",
                self.union_stats_available,
            ),
            (
                "review.taint_context.driver_stats_available",
                self.driver_stats_available,
            ),
            (
                "review.taint_context.context_unavailable",
                self.context_unavailable,
            ),
        ]
    }
    pub(crate) fn export(&self) {
        for (name, value) in self.rows() {
            prism::profile::counter(name, value);
        }
    }
}

#[cfg(test)]
#[path = "context_stats_tests.rs"]
mod tests;
