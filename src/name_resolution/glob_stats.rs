//! Process-global, per-measurement telemetry for deferred-glob expansion
//! (spec §3.5). Reset at `call_stats` entry, snapshot after the re-resolution
//! pass. The counters are expansion-event counts, not final-edge counts; the
//! realized edge buy is read from `kind_exact`/`unresolved_unknown_name`.

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct GlobExpandStats {
    resolved_l1: AtomicUsize,
    resolved_l2: AtomicUsize,
    depth_exceeded: AtomicUsize,
    cycle: AtomicUsize,
    external: AtomicUsize,
    multi_target: AtomicUsize,
    vis_unknown: AtomicUsize,
    member_multi: AtomicUsize,
    member_undecidable: AtomicUsize,
    member_hidden_continued: AtomicUsize,
    member_hidden_continue_hit: AtomicUsize,
    member_hidden_continue_empty: AtomicUsize,
    member_hidden_continue_poison: AtomicUsize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GlobExpandSnapshot {
    pub resolved_l1: usize,
    pub resolved_l2: usize,
    pub depth_exceeded: usize,
    pub cycle: usize,
    pub external: usize,
    pub multi_target: usize,
    pub vis_unknown: usize,
    pub member_multi: usize,
    pub member_undecidable: usize,
    pub member_hidden_continued: usize,
    pub member_hidden_continue_hit: usize,
    pub member_hidden_continue_empty: usize,
    pub member_hidden_continue_poison: usize,
}

impl GlobExpandStats {
    const fn z() -> AtomicUsize {
        AtomicUsize::new(0)
    }

    /// `depth` is the current glob depth: 1 for the first hop, 2 for the second.
    pub fn record_resolved(&self, depth: usize) {
        match depth {
            1 => &self.resolved_l1,
            _ => &self.resolved_l2,
        }
        .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_depth_exceeded(&self) {
        self.depth_exceeded.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cycle(&self) {
        self.cycle.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_external(&self) {
        self.external.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_multi_target(&self) {
        self.multi_target.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_vis_unknown(&self) {
        self.vis_unknown.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_member_multi(&self) {
        self.member_multi.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_member_undecidable(&self) {
        self.member_undecidable.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_member_hidden_continued(&self) {
        self.member_hidden_continued.fetch_add(1, Ordering::Relaxed);
    }

    // The three `member_hidden_continue_*` counters are same-invocation OPPORTUNITY
    // telemetry, NOT a buy bound: a `glob_lookup` invocation that took a hidden-
    // continue ended Hit / Empty / Poison. `policy.combine` may fold a Hit's
    // candidates to ResolvedSet/Ambiguous (no Exact edge) and a nested hit can be
    // swallowed by an outer poison, so `_hit` over/under-counts final recovery — the
    // realized buy is read from the `kind_exact` delta, never these counters.
    pub fn record_member_hidden_continue_hit(&self) {
        self.member_hidden_continue_hit.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_member_hidden_continue_empty(&self) {
        self.member_hidden_continue_empty
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_member_hidden_continue_poison(&self) {
        self.member_hidden_continue_poison
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        for a in [
            &self.resolved_l1,
            &self.resolved_l2,
            &self.depth_exceeded,
            &self.cycle,
            &self.external,
            &self.multi_target,
            &self.vis_unknown,
            &self.member_multi,
            &self.member_undecidable,
            &self.member_hidden_continued,
            &self.member_hidden_continue_hit,
            &self.member_hidden_continue_empty,
            &self.member_hidden_continue_poison,
        ] {
            a.store(0, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> GlobExpandSnapshot {
        let g = |a: &AtomicUsize| a.load(Ordering::Relaxed);
        GlobExpandSnapshot {
            resolved_l1: g(&self.resolved_l1),
            resolved_l2: g(&self.resolved_l2),
            depth_exceeded: g(&self.depth_exceeded),
            cycle: g(&self.cycle),
            external: g(&self.external),
            multi_target: g(&self.multi_target),
            vis_unknown: g(&self.vis_unknown),
            member_multi: g(&self.member_multi),
            member_undecidable: g(&self.member_undecidable),
            member_hidden_continued: g(&self.member_hidden_continued),
            member_hidden_continue_hit: g(&self.member_hidden_continue_hit),
            member_hidden_continue_empty: g(&self.member_hidden_continue_empty),
            member_hidden_continue_poison: g(&self.member_hidden_continue_poison),
        }
    }
}

/// The process-global sink used by production resolution. Tests can inject a
/// local `&GlobExpandStats` via engine entries added with the expansion logic.
pub static GLOBAL: GlobExpandStats = GlobExpandStats {
    resolved_l1: GlobExpandStats::z(),
    resolved_l2: GlobExpandStats::z(),
    depth_exceeded: GlobExpandStats::z(),
    cycle: GlobExpandStats::z(),
    external: GlobExpandStats::z(),
    multi_target: GlobExpandStats::z(),
    vis_unknown: GlobExpandStats::z(),
    member_multi: GlobExpandStats::z(),
    member_undecidable: GlobExpandStats::z(),
    member_hidden_continued: GlobExpandStats::z(),
    member_hidden_continue_hit: GlobExpandStats::z(),
    member_hidden_continue_empty: GlobExpandStats::z(),
    member_hidden_continue_poison: GlobExpandStats::z(),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_reset_snapshot_roundtrip() {
        let s = GlobExpandStats::default();
        s.record_resolved(1);
        s.record_resolved(2);
        s.record_depth_exceeded();
        s.record_cycle();
        s.record_external();
        s.record_multi_target();
        s.record_vis_unknown();
        s.record_member_multi();
        s.record_member_undecidable();
        s.record_member_hidden_continued();
        s.record_member_hidden_continue_hit();
        s.record_member_hidden_continue_empty();
        s.record_member_hidden_continue_poison();
        let snap = s.snapshot();
        assert_eq!(snap.resolved_l1, 1);
        assert_eq!(snap.resolved_l2, 1);
        assert_eq!(snap.depth_exceeded, 1);
        assert_eq!(snap.cycle, 1);
        assert_eq!(snap.external, 1);
        assert_eq!(snap.multi_target, 1);
        assert_eq!(snap.vis_unknown, 1);
        assert_eq!(snap.member_multi, 1);
        assert_eq!(snap.member_undecidable, 1);
        assert_eq!(snap.member_hidden_continued, 1);
        assert_eq!(snap.member_hidden_continue_hit, 1);
        assert_eq!(snap.member_hidden_continue_empty, 1);
        assert_eq!(snap.member_hidden_continue_poison, 1);
        s.reset();
        assert_eq!(s.snapshot().resolved_l1, 0);
    }
}
