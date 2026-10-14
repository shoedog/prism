//! What the control lock guards, and how an attempt's ending is kept for the calls that joined
//! it (spec §4.2). These are the types only: `coordinator.rs` holds the lock and makes every
//! change to them.
use crate::mcp::shared::preamble::BuildState;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// What a readiness call is told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Answer {
    /// The index is published.
    Ready,
    /// This connection's deadline for its attempt has passed and the attempt is still
    /// building. `elapsed` is the attempt's age.
    Warming { elapsed: Duration },
    /// The call's own attempt ended without an index, with this error, and nothing has
    /// superseded that: no later attempt is building and the index is not ready.
    Failed { error: String },
    /// The call's own attempt ended without an index, and the daemon has since moved on: a
    /// later attempt is building, or the index is ready. The failure is stale, so the call is
    /// not given it; it is told to retry. It is never given another attempt's outcome.
    Superseded,
}

/// What a call is told about its own attempt's ending `own`, given the phase now (D26). An
/// attempt that published the index is `Ready`. A failed attempt is `Superseded` when a later
/// attempt is building or the index is ready, and otherwise `Failed` with its own text. The
/// phase contributes only which of those it is: no other attempt's outcome or text.
pub(super) fn answer_for(own: &Result<(), String>, phase: &BuildPhase) -> Answer {
    match own {
        Ok(()) => Answer::Ready,
        Err(error) => match phase {
            BuildPhase::Building { .. } | BuildPhase::Ready => Answer::Superseded,
            BuildPhase::Failed { .. } | BuildPhase::Idle => Answer::Failed {
                error: error.clone(),
            },
        },
    }
}

/// How one build attempt ended: empty while it builds, filled when it ends, and not changed
/// after that (a second fill has no effect). The attempt's phase holds one share of it and
/// every call that starts or joins the attempt holds another, so a call reads the outcome of
/// its own attempt however far the phase has moved on by the time the call runs again.
#[derive(Debug, Default)]
pub(super) struct AttemptOutcome {
    ended: OnceLock<Result<(), String>>,
}

impl AttemptOutcome {
    /// Record how the attempt ended. Only the first call has any effect.
    pub(super) fn fill(&self, ended: Result<(), String>) {
        let _ = self.ended.set(ended);
    }

    /// How the attempt ended: `Ok` if it published the index, its error if it did not; `None`
    /// while it is still building.
    pub(super) fn ended(&self) -> Option<&Result<(), String>> {
        self.ended.get()
    }
}

/// Where the index build is (spec §4.2).

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;

#[derive(Debug)]
pub(super) enum BuildPhase {
    /// No build has been asked for.
    Idle,
    /// Attempt `attempt` is running since `started`. `outcome` is that attempt's, still empty.
    Building {
        attempt: u64,
        started: Instant,
        outcome: Arc<AttemptOutcome>,
    },
    /// A provider is in the slot and calls have been told so. Its generation is
    /// [`ControlState::generation`], which is kept apart from the phase because the slot is
    /// filled, and can be written to, before this phase is announced.
    Ready,
    /// Attempt `attempt` ended without a provider. The next readiness call starts the retry.
    Failed { attempt: u64, error: String },
}

impl BuildPhase {
    /// The word `READY` and the status probe use for this phase.
    pub(super) fn state(&self) -> BuildState {
        match self {
            Self::Idle => BuildState::Idle,
            Self::Building { .. } => BuildState::Building,
            Self::Ready => BuildState::Ready,
            Self::Failed { .. } => BuildState::Failed,
        }
    }
}

/// What the control lock guards. Slice S4 adds the admission counters and the retirement
/// decision beside the build phase, under the same lock.
pub(super) struct ControlState {
    pub(super) build: BuildPhase,
    /// How many attempts have been started; also the number of the latest one.
    pub(super) attempts: u64,
    /// How many calls are parked on the condition variable right now.
    pub(super) waiting: u64,
    /// The generation of the provider in the slot; 0 while the slot is empty. It is written by
    /// `record_generation` and by nothing else, each time while the slot's write guard is
    /// held: when the provider is stored, and at the end of every exclusive access. It does
    /// not depend on the phase, so it is true in the interval between publication and the
    /// announcement of `Ready` as well.
    pub(super) generation: u64,
}

/// A snapshot of the control plane: what the status probe reports about the build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BuildStatus {
    pub(super) state: BuildState,
    /// The number of the latest attempt; 0 before the first.
    pub(super) attempt: u64,
    /// The generation of the provider in the slot; 0 while the slot is empty.
    pub(super) generation: u64,
    /// Calls parked on the build at the moment of the snapshot.
    pub(super) waiting: u64,
}
