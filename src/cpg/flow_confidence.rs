//! Confidence that a `CpgEdge::DataFlow` edge's definition actually reaches
//! its use. Produced by the reaching-definitions pass (`src/cpg/reaching.rs`,
//! Task 2), NOT by the R1-R7 call-resolution ladder (`ResolutionConfidence`,
//! `src/resolution.rs`). See design §4.1.

use crate::resolution::ResolutionConfidence;
use serde::{Deserialize, Serialize};

/// Confidence that a DataFlow edge's definition actually reaches its use.
/// Two-valued lattice (Exact ⊐ NameOnly), same shape as `ResolutionConfidence`,
/// but a DIFFERENT producer: the reaching-definitions pass, not the R1–R7 call
/// ladder. Loop-carried edges are Exact — RD proved reachability through a back
/// edge; the distinction is telemetry only (§4.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FlowConfidence {
    Exact,
    NameOnly(FlowDoubt),
}

/// Why an edge could not be proven. Every variant is a reason to UNDER-assert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FlowDoubt {
    /// RD proved a redefinition of the same path kills this def before the use.
    /// `kill_line` is the lowest-numbered killing statement line on any path.
    Killed { kill_line: u32 },
    /// Two Defs of one AccessPath collapse onto one line-granular endpoint (§4.3).
    SameLine,
    /// No CFG node for the def or use line, function over the RD cap, or the
    /// function has no CFG edges at all.
    CfgIncomplete,
    /// The edge exists only through the alias map.
    AliasUnstable,
    /// Step-5b arg→param edge whose resolved callee is NameOnly.
    CallNameOnly,
}

impl FlowConfidence {
    /// A total-order badness key: `(kind_rank, tie_break)`, compared
    /// lexicographically. `Exact` has the lowest `kind_rank` (0), so every
    /// `NameOnly` outranks it. Within `Killed`, the tie-break is the
    /// NEGATED `kill_line` so the numerically LOWER `kill_line` (the
    /// earliest-proven kill) sorts as the greater badness, i.e. wins
    /// `worst`. `worst` is then "the value with the greater badness key",
    /// which is a `max` over a genuine total order — commutative,
    /// associative and idempotent by construction, so no case-by-case proof
    /// is needed for those three properties.
    fn badness_key(self) -> (u8, i64) {
        match self {
            FlowConfidence::Exact => (0, 0),
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line }) => (1, -(kill_line as i64)),
            FlowConfidence::NameOnly(FlowDoubt::SameLine) => (2, 0),
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete) => (3, 0),
            FlowConfidence::NameOnly(FlowDoubt::AliasUnstable) => (4, 0),
            FlowConfidence::NameOnly(FlowDoubt::CallNameOnly) => (5, 0),
        }
    }

    /// The lattice meet. NEVER use `std::cmp::min`: derived `Ord` puts `Exact`
    /// FIRST, so `min` returns the BEST label, not the worst. Same trap as
    /// `ParseQuality::min_over`, which uses `worst.max(..)`
    /// (`src/finding_confidence.rs:107`).
    pub fn worst(self, other: Self) -> Self {
        if self.badness_key() >= other.badness_key() {
            self
        } else {
            other
        }
    }

    pub fn is_exact(self) -> bool {
        matches!(self, FlowConfidence::Exact)
    }

    pub fn level(self) -> &'static str {
        match self {
            FlowConfidence::Exact => "exact",
            FlowConfidence::NameOnly(_) => "nameonly",
        }
    }
}

impl From<ResolutionConfidence> for FlowConfidence {
    fn from(c: ResolutionConfidence) -> Self {
        match c {
            ResolutionConfidence::Exact => FlowConfidence::Exact,
            ResolutionConfidence::NameOnly => FlowConfidence::NameOnly(FlowDoubt::CallNameOnly),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::ResolutionConfidence;

    const ALL: [FlowConfidence; 6] = [
        FlowConfidence::Exact,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 7 }),
        FlowConfidence::NameOnly(FlowDoubt::SameLine),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        FlowConfidence::NameOnly(FlowDoubt::AliasUnstable),
        FlowConfidence::NameOnly(FlowDoubt::CallNameOnly),
    ];

    #[test]
    fn worst_is_commutative_and_associative() {
        for a in ALL {
            for b in ALL {
                assert_eq!(a.worst(b), b.worst(a), "{a:?} vs {b:?}");
                for c in ALL {
                    assert_eq!(
                        a.worst(b).worst(c),
                        a.worst(b.worst(c)),
                        "{a:?} {b:?} {c:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn nameonly_absorbs_exact() {
        for a in ALL.into_iter().skip(1) {
            assert_eq!(FlowConfidence::Exact.worst(a), a);
            assert!(!FlowConfidence::Exact.worst(a).is_exact());
        }
        assert_eq!(
            FlowConfidence::Exact.worst(FlowConfidence::Exact),
            FlowConfidence::Exact
        );
    }

    #[test]
    fn two_killed_doubts_keep_the_lower_kill_line() {
        let lo = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 });
        let hi = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 9 });
        assert_eq!(lo.worst(hi), lo);
        assert_eq!(hi.worst(lo), lo);
    }

    /// Pins the derived-`Ord` trap explicitly (§7.2): `Exact` sorts FIRST, so
    /// `min` returns the BEST label. `worst` must not be `min`.
    #[test]
    fn worst_is_not_std_cmp_min() {
        let a = FlowConfidence::Exact;
        let b = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
        assert_eq!(
            std::cmp::min(a, b),
            FlowConfidence::Exact,
            "derived Ord puts Exact first"
        );
        assert_eq!(a.worst(b), b);
        assert_ne!(a.worst(b), std::cmp::min(a, b));
    }

    #[test]
    fn vocabulary_matches_the_finding_confidence_spelling() {
        assert_eq!(FlowConfidence::Exact.level(), "exact");
        for a in ALL.into_iter().skip(1) {
            assert_eq!(a.level(), "nameonly");
        }
        assert!(FlowConfidence::Exact.is_exact());
    }

    #[test]
    fn resolution_confidence_conversion_maps_both_poles() {
        assert_eq!(
            FlowConfidence::from(ResolutionConfidence::Exact),
            FlowConfidence::Exact
        );
        assert_eq!(
            FlowConfidence::from(ResolutionConfidence::NameOnly),
            FlowConfidence::NameOnly(FlowDoubt::CallNameOnly)
        );
    }
}
