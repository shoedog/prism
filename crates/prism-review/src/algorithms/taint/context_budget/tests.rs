//! Controlled reader and guard tests; real loader controls live in `real`.
use super::*;
use std::cell::Cell;
mod real;

struct Guard {
    calls: Cell<usize>,
    at: usize,
    cause: StopCause,
}
impl RunGuard for Guard {
    fn stop(&self) -> Option<StopCause> {
        let n = self.calls.get();
        self.calls.set(n + 1);
        (n >= self.at).then_some(self.cause)
    }
}
fn guard() -> Guard {
    Guard {
        calls: Cell::new(0),
        at: usize::MAX,
        cause: StopCause::Time,
    }
}
fn targets(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}
fn limits(files: usize, bytes: u64) -> ContextLimits {
    ContextLimits::for_test(2, files, bytes, Duration::from_secs(20)).unwrap()
}
fn loaded(bytes: u64) -> ReadReport<u64> {
    ReadReport {
        attempted_bytes: bytes,
        kind: ReadKind::Loaded { bytes, file: bytes },
    }
}

// CONTROLLED: closure is the reader.
#[test]
fn files_prefix_stops_before_the_extra_read() {
    let mut c = AttemptCounters::default();
    let mut reads = 0;
    let stage = admit_prefix(
        &targets(&["a", "b", "c"]),
        &limits(2, 20),
        Published::default(),
        &mut c,
        &guard(),
        &mut |_, _| {
            reads += 1;
            loaded(1)
        },
    );
    assert_eq!(stage.accepted.len(), 2);
    assert_eq!(reads, 2);
    assert_eq!(
        stage.bound,
        Some(AdmissionBound::Files {
            first_skipped: 2,
            skipped: 1
        })
    );
    let stage = admit_prefix::<u64>(
        &targets(&["x"]),
        &limits(0, 20),
        Published::default(),
        &mut c,
        &guard(),
        &mut |_, _| panic!("zero must not read"),
    );
    assert_eq!(
        stage.bound,
        Some(AdmissionBound::Files {
            first_skipped: 0,
            skipped: 1
        })
    );
}
// CONTROLLED: closure supplies the capacity outcome.
#[test]
fn byte_bound_seals_the_prefix() {
    let mut c = AttemptCounters::default();
    let mut reads = 0;
    let stage = admit_prefix(
        &targets(&["a", "b", "c"]),
        &limits(5, 10),
        Published::default(),
        &mut c,
        &guard(),
        &mut |_, _| {
            reads += 1;
            if reads == 1 {
                loaded(4)
            } else {
                ReadReport {
                    attempted_bytes: 7,
                    kind: ReadKind::CapacityBound,
                }
            }
        },
    );
    assert_eq!(stage.accepted.len(), 1);
    assert_eq!(reads, 2);
    assert_eq!(
        stage.bound,
        Some(AdmissionBound::SourceBytes {
            first_skipped: 1,
            skipped: 2
        })
    );
    assert_eq!(c.attempted_files, 2);
}
// CONTROLLED: closure supplies an oversized skip.
#[test]
fn oversize_skip_lets_a_later_file_in() {
    let mut c = AttemptCounters::default();
    let stage = admit_prefix(
        &targets(&["big", "small"]),
        &limits(1, 10),
        Published::default(),
        &mut c,
        &guard(),
        &mut |p, _| {
            if p == "big" {
                ReadReport {
                    attempted_bytes: 11,
                    kind: ReadKind::Skipped(SkipReason::TooLarge {
                        observed_at_least: 11,
                    }),
                }
            } else {
                loaded(2)
            }
        },
    );
    assert_eq!(stage.accepted.len(), 1);
    assert_eq!(stage.accepted[0].0, "small");
    assert_eq!(c.skips.len(), 1);
    assert_eq!(stage.bound, None);
}
// CONTROLLED: rejected bytes are observable attempted work only.
#[test]
fn rejected_reads_do_not_spend_capacity() {
    let mut c = AttemptCounters::default();
    let mut seen = Vec::new();
    let stage = admit_prefix(
        &targets(&["a", "b", "c"]),
        &limits(4, 1000),
        Published {
            files: 0,
            bytes: 100,
        },
        &mut c,
        &guard(),
        &mut |p, remaining| {
            seen.push(remaining);
            match p {
                "a" => loaded(50),
                "b" => ReadReport {
                    attempted_bytes: 500,
                    kind: ReadKind::Skipped(SkipReason::NotUtf8),
                },
                _ => loaded(1),
            }
        },
    );
    assert_eq!(stage.accepted.len(), 2);
    assert_eq!(seen, [900, 850, 850]);
    assert_eq!(c.attempted_read_bytes, 551);
}
// CONTROLLED: sentinel is in the supplied attempted count.
#[test]
fn sentinel_is_attempted_never_admitted() {
    let mut c = AttemptCounters::default();
    let s = admit_prefix(
        &targets(&["a", "b"]),
        &limits(2, 10),
        Published::default(),
        &mut c,
        &guard(),
        &mut |p, _| {
            if p == "a" {
                loaded(4)
            } else {
                ReadReport {
                    attempted_bytes: 7,
                    kind: ReadKind::CapacityBound,
                }
            }
        },
    );
    assert_eq!(s.accepted_bytes, 4);
    assert_eq!(c.attempted_read_bytes, 11);
    assert_eq!(c.attempted_files, 2);
}
// CONTROLLED: caller supplies completed counts from a prior round.
#[test]
fn published_totals_carry_across_rounds() {
    let mut c = AttemptCounters::default();
    let mut remaining = Vec::new();
    let s = admit_prefix(
        &targets(&["a", "b", "c", "d", "e"]),
        &limits(64, 100),
        Published {
            files: 60,
            bytes: 20,
        },
        &mut c,
        &guard(),
        &mut |_, r| {
            remaining.push(r);
            loaded(3)
        },
    );
    assert_eq!(s.accepted.len(), 4);
    assert_eq!(remaining, [80, 77, 74, 71]);
    assert_eq!(
        s.bound,
        Some(AdmissionBound::Files {
            first_skipped: 4,
            skipped: 1
        })
    );
}
// CONTROLLED: guard is injected and never sleeps.
#[test]
fn guard_stops_between_reads() {
    for cause in [StopCause::Time, StopCause::Cancelled] {
        let g = Guard {
            calls: Cell::new(0),
            at: 1,
            cause,
        };
        let mut c = AttemptCounters::default();
        let mut reads = 0;
        let s = admit_prefix(
            &targets(&["a", "b"]),
            &limits(2, 10),
            Published::default(),
            &mut c,
            &g,
            &mut |_, _| {
                reads += 1;
                loaded(2)
            },
        );
        assert_eq!(s.accepted.len(), 1);
        assert_eq!(s.stopped, Some(cause));
        assert_eq!(reads, 1);
        assert_eq!(c.attempted_files, 1);
        assert_eq!(c.attempted_read_bytes, 2);
        let immediate = Guard {
            calls: Cell::new(0),
            at: 0,
            cause,
        };
        let before = c.attempted_files;
        let s = admit_prefix(
            &targets(&["x"]),
            &limits(1, 10),
            Published::default(),
            &mut c,
            &immediate,
            &mut |_, _| loaded(1),
        );
        assert_eq!(s.stopped, Some(cause));
        assert_eq!(c.attempted_files, before);
    }
}
// CONTROLLED: duplicate paths are intentionally fed to the closure.
#[test]
fn order_and_identity_are_the_callers() {
    let mut c = AttemptCounters::default();
    let mut seen = Vec::new();
    let s = admit_prefix(
        &targets(&["z", "a", "z"]),
        &limits(4, 10),
        Published::default(),
        &mut c,
        &guard(),
        &mut |p, _| {
            seen.push(p.to_string());
            loaded(1)
        },
    );
    assert_eq!(s.accepted.len(), 3);
    assert_eq!(seen, targets(&["z", "a", "z"]));
    assert_eq!(
        s.accepted
            .iter()
            .map(|(p, _)| p.clone())
            .collect::<Vec<_>>(),
        seen
    );
}
#[test]
fn limits_validate_depth() {
    assert!(ContextLimits::new(0, 1).is_err());
    assert!(ContextLimits::new(5, 1).is_err());
    assert!(ContextLimits::new(1, 1).is_ok());
    assert!(ContextLimits::new(4, 1).is_ok());
    let l = ContextLimits::default();
    assert_eq!(
        (l.depth, l.max_files, l.source_bytes, l.time),
        (2, 64, 8 * 1024 * 1024, Duration::from_secs(20))
    );
    assert_eq!(TimeGuard::new(Duration::ZERO).stop(), Some(StopCause::Time));
    let old = TimeGuard {
        started: Instant::now().checked_sub(Duration::from_secs(2)).unwrap(),
        limit: Duration::from_secs(1),
    };
    assert_eq!(old.stop(), Some(StopCause::Time));
    assert_eq!(old.stop(), Some(StopCause::Time));
}

// CONTROLLED: invalid private-caller inputs fail closed without arithmetic panics.
#[test]
fn invalid_counts_and_reader_fail_closed() {
    let mut counters = AttemptCounters {
        attempted_files: u64::MAX,
        attempted_read_bytes: u64::MAX,
        skips: Vec::new(),
    };
    let s = admit_prefix(
        &targets(&["a"]),
        &limits(1, 4),
        Published {
            files: usize::MAX,
            bytes: u64::MAX,
        },
        &mut counters,
        &guard(),
        &mut |_, _| -> ReadReport<u64> { panic!("file bound precedes read") },
    );
    assert_eq!(
        s.bound,
        Some(AdmissionBound::Files {
            first_skipped: 0,
            skipped: 1
        })
    );
    let s = admit_prefix(
        &targets(&["a"]),
        &limits(1, 4),
        Published {
            files: 0,
            bytes: u64::MAX,
        },
        &mut counters,
        &guard(),
        &mut |_, remaining| {
            assert_eq!(remaining, 0);
            loaded(1)
        },
    );
    assert_eq!(
        s.bound,
        Some(AdmissionBound::SourceBytes {
            first_skipped: 0,
            skipped: 1
        })
    );
    assert_eq!(counters.attempted_files, u64::MAX);
    assert_eq!(counters.attempted_read_bytes, u64::MAX);
}
