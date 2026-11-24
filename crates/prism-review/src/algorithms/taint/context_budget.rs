//! Retained context source capacity is not cumulative I/O: attempted bytes can exceed it.
//! A read and parse (at most 2 MiB plus a sentinel) is one bounded step the guard cannot
//! interrupt. Advisory Go declaration hint reads are uncapped and are not counted here.
use prism::api::{ContextFile, ContextReadAttempt, ContextReadOutcome, ContextSkip};
use std::time::{Duration, Instant};

const SOURCE_BYTES: u64 = 8 * 1024 * 1024;
const RUN_TIME: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy)]
pub(crate) struct ContextLimits {
    pub(crate) depth: u8,
    pub(crate) max_files: usize,
    pub(crate) source_bytes: u64,
    pub(crate) time: Duration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LimitError;
impl ContextLimits {
    pub(crate) fn new(depth: u8, max_files: usize) -> Result<Self, LimitError> {
        if !(1..=4).contains(&depth) {
            return Err(LimitError);
        }
        Ok(Self {
            depth,
            max_files,
            source_bytes: SOURCE_BYTES,
            time: RUN_TIME,
        })
    }
    #[cfg(test)]
    fn for_test(
        depth: u8,
        max_files: usize,
        source_bytes: u64,
        time: Duration,
    ) -> Result<Self, LimitError> {
        let mut limits = Self::new(depth, max_files)?;
        limits.source_bytes = source_bytes;
        limits.time = time;
        Ok(limits)
    }
}
impl Default for ContextLimits {
    fn default() -> Self {
        Self::new(2, 64).expect("default depth is valid")
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Published {
    pub(crate) files: usize,
    pub(crate) bytes: u64,
}
#[derive(Debug, Default)]
pub(crate) struct AttemptCounters {
    pub(crate) attempted_files: u64,
    pub(crate) attempted_read_bytes: u64,
    pub(crate) skips: Vec<(String, SkipReason)>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StopCause {
    Time,
    Cancelled,
}
pub(crate) trait RunGuard {
    fn stop(&self) -> Option<StopCause>;
}
pub(crate) struct TimeGuard {
    started: Instant,
    limit: Duration,
}
impl TimeGuard {
    pub(crate) fn new(limit: Duration) -> Self {
        Self {
            started: Instant::now(),
            limit,
        }
    }
}
impl RunGuard for TimeGuard {
    fn stop(&self) -> Option<StopCause> {
        (self.started.elapsed() >= self.limit).then_some(StopCause::Time)
    }
}
pub(crate) struct ReadReport<T> {
    pub(crate) attempted_bytes: u64,
    pub(crate) kind: ReadKind<T>,
}
pub(crate) enum ReadKind<T> {
    Loaded { bytes: u64, file: T },
    Skipped(SkipReason),
    CapacityBound,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkipReason {
    InvalidPath,
    NotListed,
    Unsupported,
    OriginalFile,
    UnsafePath,
    NotRegular,
    Unreadable {
        kind: std::io::ErrorKind,
        message: String,
    },
    NotUtf8,
    TooLarge {
        observed_at_least: u64,
    },
    ParseFailed {
        message: String,
    },
    SevereParse {
        error_count: usize,
        node_count: usize,
    },
    Unknown,
}
pub(crate) fn report_from_attempt(attempt: ContextReadAttempt) -> ReadReport<ContextFile> {
    let kind = match attempt.outcome {
        ContextReadOutcome::Loaded(file) => ReadKind::Loaded {
            bytes: file.source.len() as u64,
            file,
        },
        ContextReadOutcome::Skipped(skip) => ReadKind::Skipped(match skip {
            ContextSkip::InvalidPath => SkipReason::InvalidPath,
            ContextSkip::NotListed => SkipReason::NotListed,
            ContextSkip::Unsupported => SkipReason::Unsupported,
            ContextSkip::OriginalFile => SkipReason::OriginalFile,
            ContextSkip::UnsafePath => SkipReason::UnsafePath,
            ContextSkip::NotRegular => SkipReason::NotRegular,
            ContextSkip::Unreadable { kind, message } => SkipReason::Unreadable { kind, message },
            ContextSkip::NotUtf8 => SkipReason::NotUtf8,
            ContextSkip::TooLarge { observed_at_least } => {
                SkipReason::TooLarge { observed_at_least }
            }
            ContextSkip::ParseFailed { message } => SkipReason::ParseFailed { message },
            ContextSkip::SevereParse {
                error_count,
                node_count,
            } => SkipReason::SevereParse {
                error_count,
                node_count,
            },
            _ => SkipReason::Unknown,
        }),
        ContextReadOutcome::SourceCapacityBound => ReadKind::CapacityBound,
        _ => ReadKind::CapacityBound,
    };
    ReadReport {
        attempted_bytes: attempt.attempted_read_bytes,
        kind,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdmissionBound {
    Files {
        first_skipped: usize,
        skipped: usize,
    },
    SourceBytes {
        first_skipped: usize,
        skipped: usize,
    },
}
pub(crate) struct StagedAdmission<T> {
    pub(crate) accepted: Vec<(String, T)>,
    pub(crate) accepted_bytes: u64,
    pub(crate) bound: Option<AdmissionBound>,
    pub(crate) stopped: Option<StopCause>,
}
/// Callers must supply completed counts within the limits and a reader that does not return
/// Loaded bytes exceeding its remaining allowance. Counters saturate rather than wrap.
pub(crate) fn admit_prefix<T>(
    targets: &[String],
    limits: &ContextLimits,
    published: Published,
    counters: &mut AttemptCounters,
    guard: &dyn RunGuard,
    read: &mut dyn FnMut(&str, u64) -> ReadReport<T>,
) -> StagedAdmission<T> {
    let mut stage = StagedAdmission {
        accepted: Vec::new(),
        accepted_bytes: 0,
        bound: None,
        stopped: None,
    };
    for (index, path) in targets.iter().enumerate() {
        if let Some(cause) = guard.stop() {
            stage.stopped = Some(cause);
            return stage;
        }
        if published.files.saturating_add(stage.accepted.len()) >= limits.max_files {
            stage.bound = Some(AdmissionBound::Files {
                first_skipped: index,
                skipped: targets.len() - index,
            });
            return stage;
        }
        let remaining = limits
            .source_bytes
            .saturating_sub(published.bytes)
            .saturating_sub(stage.accepted_bytes);
        let report = read(path, remaining);
        counters.attempted_files = counters.attempted_files.saturating_add(1);
        counters.attempted_read_bytes = counters
            .attempted_read_bytes
            .saturating_add(report.attempted_bytes);
        match report.kind {
            ReadKind::Loaded { bytes, file } if bytes <= remaining => {
                stage.accepted_bytes = stage.accepted_bytes.saturating_add(bytes);
                stage.accepted.push((path.clone(), file));
            }
            ReadKind::Loaded { .. } | ReadKind::CapacityBound => {
                stage.bound = Some(AdmissionBound::SourceBytes {
                    first_skipped: index,
                    skipped: targets.len() - index,
                });
                return stage;
            }
            ReadKind::Skipped(reason) => counters.skips.push((path.clone(), reason)),
        }
    }
    stage
}
#[cfg(test)]
mod tests;
