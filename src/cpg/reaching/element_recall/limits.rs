//! I1 limits (lane 10, #847, DESIGN section 5): raw work, read fan-in and owner size.
//!
//! The order is the design's. Work `P` is counted first, from raw bucket rows, before any route,
//! pair or record is allocated; a unit is one raw Def row of a certified write times one distinct
//! eligible whole read it can feed. `P > raw_pairs` (or an overflowing sum) refuses the owner.
//! Survivors of capture, route and occupancy are then deduplicated, a read fed by more than
//! `read_sources` distinct writes loses all of its pairs, and an owner left with more than
//! `owner_edges` pairs loses the remainder. Equality passes each limit.
use super::join::Buckets;
use super::Span;
use crate::access_path::AccessPath;
use crate::data_flow::VarAccessKind;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const RAW_PAIRS: u64 = 1_000_000;
pub(crate) const READ_SOURCES: usize = 8;
pub(crate) const OWNER_EDGES: usize = 256;
/// Per-pair refusal records listed for an owner whose pairs exceed the allocation bound; the
/// owner record carries the exact total and the omitted count.
pub(crate) const REFUSAL_SAMPLE: usize = 16;

/// The three limits. Production uses [`Limits::PRODUCTION`]; unit tests inject smaller ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limits {
    pub raw_pairs: u64,
    pub read_sources: usize,
    pub owner_edges: usize,
}

impl Limits {
    pub(crate) const PRODUCTION: Limits = Limits {
        raw_pairs: RAW_PAIRS,
        read_sources: READ_SOURCES,
        owner_edges: OWNER_EDGES,
    };
}

/// How reaching definitions treats an owner. `Unseen` marks an owner the producer never handed
/// to the RD call or its preflight (no record, so nothing can be predicted for it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RdExit {
    Available,
    DefCap,
    LineCap,
    NoCfg,
    Unseen,
}

/// An RD exit plus whether the owner was skipped before the RD call (the early fallback cap).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rd {
    pub exit: RdExit,
    pub early: bool,
}

impl Rd {
    pub(crate) fn name(self) -> &'static str {
        match (self.exit, self.early) {
            (RdExit::Available, _) => "available",
            (RdExit::Unseen, _) => "unseen",
            (RdExit::DefCap, false) => "def_cap",
            (RdExit::LineCap, false) => "line_cap",
            (RdExit::NoCfg, false) => "no_cfg",
            (RdExit::DefCap, true) => "fallback_cap:def_cap",
            (RdExit::LineCap, true) => "fallback_cap:line_cap",
            (RdExit::NoCfg, true) => "fallback_cap:no_cfg",
        }
    }
}

/// The raw pair count `P` of one owner. A total that does not fit saturates at `u64::MAX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Work {
    pub total: u64,
    pub overflow: bool,
}

impl Work {
    /// Sum `rows x reads` over the buckets, with checked arithmetic.
    pub(crate) fn count(buckets: impl IntoIterator<Item = (u64, u64)>) -> Work {
        let mut work = Work {
            total: 0,
            overflow: false,
        };
        for (rows, reads) in buckets {
            match rows
                .checked_mul(reads)
                .and_then(|p| work.total.checked_add(p))
            {
                Some(total) if !work.overflow => work.total = total,
                _ => {
                    work.total = u64::MAX;
                    work.overflow = true;
                }
            }
        }
        work
    }

    /// Overflow refuses even when the saturated total equals the limit.
    pub(crate) fn exceeds(self, limit: u64) -> bool {
        self.overflow || self.total > limit
    }
}

/// Raw Def or Use rows of `(owner, path)` at exactly this line and byte span, duplicates included.
pub(crate) fn raw_rows(
    buckets: &Buckets,
    owner: (&str, &str, usize),
    path: &AccessPath,
    kind: VarAccessKind,
    at: Span,
) -> u64 {
    let key = (owner.0.into(), owner.1.into(), owner.2, path.clone());
    let same = |l: &&crate::data_flow::VarLocation| {
        (l.kind, l.line, l.start_byte, l.end_byte) == (kind, at.line, at.start, at.end)
    };
    buckets
        .get(&key)
        .map_or(0, |rows| rows.iter().filter(same).count() as u64)
}

/// What the fan-in and owner limits kept and omitted. Pairs are `(write, read)` spans.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Capped {
    pub kept: BTreeSet<(Span, Span)>,
    /// Reads fed by more than `read_sources` distinct writes, and the pairs they lost.
    pub fanin_reads: BTreeSet<Span>,
    pub fanin_pairs: usize,
    /// The remaining owner addition exceeded `owner_edges`: every pair of it is omitted.
    pub owner_over: bool,
    pub owner_pairs: usize,
}

/// Apply the fan-in limit to each read, then the owner limit to what remains.
pub(crate) fn cap(limits: &Limits, survivors: &BTreeSet<(Span, Span)>) -> Capped {
    let mut by_read: BTreeMap<Span, BTreeSet<Span>> = BTreeMap::new();
    for (w, r) in survivors {
        by_read.entry(*r).or_default().insert(*w);
    }
    let mut out = Capped::default();
    for (r, writes) in &by_read {
        if writes.len() > limits.read_sources {
            out.fanin_reads.insert(*r);
            out.fanin_pairs += writes.len();
        }
    }
    let rest: BTreeSet<_> = survivors
        .iter()
        .filter(|(_, r)| !out.fanin_reads.contains(r))
        .copied()
        .collect();
    if rest.len() > limits.owner_edges {
        out.owner_over = true;
        out.owner_pairs = rest.len();
    } else {
        out.kept = rest;
    }
    out
}
