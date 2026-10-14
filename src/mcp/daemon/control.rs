//! What the control lock guards, and how an attempt's ending is kept for the calls that joined
//! it (spec §4.2). These are the types only: `coordinator.rs` holds the lock and makes every
//! change to them.
use crate::mcp::lazy::Readiness;
use crate::mcp::shared::preamble::BuildState;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

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

    /// What a call that belongs to this attempt is told; `None` while it is still building.
    pub(super) fn answer(&self) -> Option<Readiness> {
        self.ended.get().map(|ended| match ended {
            Ok(()) => Readiness::Ready,
            Err(error) => Readiness::Failed {
                error: error.clone(),
            },
        })
    }
}

/// Where the index build is (spec §4.2).
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
