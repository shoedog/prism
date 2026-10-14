//! The readiness coordinator and the provider slot of a shared daemon (spec §4.2).
//!
//! Two synchronization domains:
//!
//! - the **control plane**, one `Mutex<ControlState>` with one `Condvar`: where the index build
//!   is (`Idle`, `Building`, `Ready`, `Failed`) and how many calls are parked on it. It is held
//!   for a few field reads and writes at a time and while one build thread is started; not
//!   across a build, a query or a refresh.
//! - the **provider slot**, an `RwLock<Option<SessionProvider>>`, empty until a build is
//!   published into it.
//!
//! Lock order: code that holds the control lock does not touch the provider slot. Code that
//! holds the slot may take the control lock.
//!
//! Not here yet: a panic boundary. A poisoned lock is reported to the caller as [`Poisoned`]
//! and nothing is read through it; a panic in the builder is recorded as a failed attempt.
//! Turning both into the daemon-fatal abort of spec §3.1 is slice S3b.
use super::budget::ConnectionBudget;
use super::clock::{Clock, SystemClock};
use crate::mcp::lazy::{Readiness, SessionBuilder};
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{SessionProvider, StartupMode};
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use std::time::Instant;

/// What a call is told when the builder panicked. The in-process server uses the same text.
const BUILD_PANICKED: &str = "index build panicked";
const SLOT_POISONED: &str =
    "the index was built and could not be published: the provider lock is poisoned";
/// Used only if an attempt is found to be over with no failure recorded, which the two
/// functions that change the phase do not allow.
const BUILD_FAILED: &str = "index build failed";

/// A lock of the daemon is poisoned: a thread panicked while holding it. The state behind it
/// is not read again. Until slice S3b makes this the daemon-fatal abort of spec §3.1, the
/// caller ends its connection with this error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Poisoned {
    pub(super) lock: &'static str,
}

impl fmt::Display for Poisoned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the daemon's {} lock is poisoned: a thread panicked while holding it",
            self.lock
        )
    }
}

impl std::error::Error for Poisoned {}

/// Where the index build is (spec §4.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum BuildPhase {
    /// No build has been asked for.
    Idle,
    /// Attempt `attempt` is running since `started`.
    Building { attempt: u64, started: Instant },
    /// A provider is in the slot. `generation` is the provider's, as of the last completed
    /// exclusive access to it.
    Ready { generation: u64 },
    /// Attempt `attempt` ended without a provider. The next readiness call starts the retry.
    Failed { attempt: u64, error: String },
}

impl BuildPhase {
    /// The word `READY` and the status probe use for this phase.
    pub(super) fn state(&self) -> BuildState {
        match self {
            Self::Idle => BuildState::Idle,
            Self::Building { .. } => BuildState::Building,
            Self::Ready { .. } => BuildState::Ready,
            Self::Failed { .. } => BuildState::Failed,
        }
    }

    /// The generation of the published index; 0 while there is none.
    pub(super) fn generation(&self) -> u64 {
        match self {
            Self::Ready { generation } => *generation,
            Self::Idle | Self::Building { .. } | Self::Failed { .. } => 0,
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
    /// The error of the most recent failed attempt. It outlives the `Failed` phase, for a call
    /// that was waiting on that attempt and runs again only after the retry has begun.
    pub(super) last_error: Option<String>,
}

impl ControlState {
    /// What a call that belongs to `attempt` is told now; `None` while that attempt is still
    /// building.
    fn outcome_for(&self, attempt: u64) -> Option<Readiness> {
        match &self.build {
            BuildPhase::Ready { .. } => Some(Readiness::Ready),
            BuildPhase::Building {
                attempt: building, ..
            } if *building == attempt => None,
            BuildPhase::Failed { error, .. } => Some(Readiness::Failed {
                error: error.clone(),
            }),
            // A later attempt is building, so the one this call belongs to failed. The call is
            // told so; it does not join the retry with a budget it was given for another attempt.
            BuildPhase::Building { .. } | BuildPhase::Idle => {
                let error = self.last_error.clone();
                Some(Readiness::Failed {
                    error: error.unwrap_or_else(|| BUILD_FAILED.to_string()),
                })
            }
        }
    }
}

/// A snapshot of the control plane: what the status probe reports about the build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BuildStatus {
    pub(super) state: BuildState,
    /// The number of the latest attempt; 0 before the first.
    pub(super) attempt: u64,
    /// The generation of the published index; 0 while there is none.
    pub(super) generation: u64,
    /// Calls parked on the build at the moment of the snapshot.
    pub(super) waiting: u64,
}

/// The work of one build thread.
pub(super) type BuildJob = Box<dyn FnOnce() + Send>;

/// Starts a thread that runs a [`BuildJob`]. It is called with the control lock held, so it
/// must not run the job on the calling thread.
pub(super) type Spawner = Box<dyn Fn(BuildJob) -> std::io::Result<()> + Send + Sync>;

fn spawn_build_thread(job: BuildJob) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("prism-mcpd-build".to_string())
        .spawn(job)
        .map(|_handle| ())
}

/// A step of the build thread at which a test can stand in for another actor. Called with no
/// lock held, with the attempt's number.
#[cfg(test)]
pub(super) type AttemptHook = Arc<dyn Fn(u64) + Send + Sync>;

#[cfg(test)]
#[derive(Clone, Default)]
pub(super) struct TestHooks {
    /// After the provider is in the slot, before `Ready` is announced.
    pub(super) published: Option<AttemptHook>,
    /// After an attempt's outcome is recorded, before the parked calls are notified.
    pub(super) settled: Option<AttemptHook>,
}

/// What a [`DaemonState`] is made of. Production code uses [`Parts::production`]; tests replace
/// the clock, the spawner or the hooks.
pub(super) struct Parts {
    pub(super) builder: SessionBuilder,
    pub(super) startup: StartupMode,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) spawner: Spawner,
    #[cfg(test)]
    pub(super) hooks: TestHooks,
}

impl Parts {
    pub(super) fn production(builder: SessionBuilder, startup: StartupMode) -> Self {
        Self {
            builder,
            startup,
            clock: Arc::new(SystemClock),
            spawner: Box::new(spawn_build_thread),
            #[cfg(test)]
            hooks: TestHooks::default(),
        }
    }
}

/// One daemon's index build and the index it produced, shared by every connection.
pub(super) struct DaemonState {
    control: Mutex<ControlState>,
    changed: Condvar,
    provider: RwLock<Option<SessionProvider>>,
    builder: SessionBuilder,
    startup: StartupMode,
    clock: Arc<dyn Clock>,
    spawner: Spawner,
    #[cfg(test)]
    hooks: TestHooks,
}

impl DaemonState {
    /// A daemon state that builds with `builder`: on the first readiness call for
    /// `StartupMode::Lazy`, at once for `StartupMode::Background`.
    pub(super) fn new(builder: SessionBuilder, startup: StartupMode) -> anyhow::Result<Arc<Self>> {
        Self::from_parts(Parts::production(builder, startup))
    }

    pub(super) fn from_parts(parts: Parts) -> anyhow::Result<Arc<Self>> {
        if parts.startup == StartupMode::Eager {
            anyhow::bail!(
                "a shared daemon builds its index on the first tool call or in the background; \
                 StartupMode::Eager belongs to the in-process server"
            );
        }
        let state = Arc::new(Self {
            control: Mutex::new(ControlState {
                build: BuildPhase::Idle,
                attempts: 0,
                waiting: 0,
                last_error: None,
            }),
            changed: Condvar::new(),
            provider: RwLock::new(None),
            builder: parts.builder,
            startup: parts.startup,
            clock: parts.clock,
            spawner: parts.spawner,
            #[cfg(test)]
            hooks: parts.hooks,
        });
        if state.startup == StartupMode::Background {
            let mut control = state.lock_control()?;
            let _started = state.start_attempt(&mut control);
        }
        Ok(state)
    }

    /// How this daemon's index is started. `initialize` advertises it (spec §5).
    pub(super) fn startup_mode(&self) -> StartupMode {
        self.startup
    }

    /// Make sure a build has been started and wait for it, no longer than `budget` allows.
    ///
    /// The call first decides which attempt it belongs to, once: the one that is building, or a
    /// new one if none has been started or the last one failed. It then waits for that attempt
    /// only and returns
    ///
    /// - `Ready` once the index is published,
    /// - `Failed` if that attempt ended without an index (the call does not start the retry;
    ///   the next call does),
    /// - `Warming` when this connection's deadline for that attempt has passed. That is this
    ///   connection's wait ending; the build goes on and nobody else's deadline changes.
    ///
    /// Starting a new attempt and joining a running one happen under one acquisition of the
    /// control lock, so however many calls find the build failed, one of them starts the retry
    /// and the rest join it.
    pub(super) fn ensure_ready(
        self: &Arc<Self>,
        budget: &mut ConnectionBudget,
    ) -> Result<Readiness, Poisoned> {
        let mut control = self.lock_control()?;

        let building = match &control.build {
            BuildPhase::Ready { .. } => return Ok(Readiness::Ready),
            BuildPhase::Building { attempt, started } => Some((*attempt, *started)),
            BuildPhase::Idle | BuildPhase::Failed { .. } => None,
        };
        let (attempt, started) = building.unwrap_or_else(|| self.start_attempt(&mut control));
        let deadline = budget.deadline_for(attempt, self.clock.now());

        loop {
            if let Some(outcome) = control.outcome_for(attempt) {
                return Ok(outcome);
            }
            let now = self.clock.now();
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                return Ok(Readiness::Warming {
                    elapsed: now.saturating_duration_since(started),
                });
            }
            control.waiting = control.waiting.saturating_add(1);
            control = match self.changed.wait_timeout(control, remaining) {
                Ok((reacquired, _timed_out)) => reacquired,
                Err(_) => return Err(Poisoned { lock: "control" }),
            };
            control.waiting = control.waiting.saturating_sub(1);
        }
    }

    /// The control plane's view of the build.
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        let control = self.lock_control()?;
        Ok(BuildStatus {
            state: control.build.state(),
            attempt: control.attempts,
            generation: control.build.generation(),
            waiting: control.waiting,
        })
    }

    /// Run `query` on the published provider with shared access; `None` when nothing is
    /// published. `query` gets the whole provider, so it sees what the in-process server's
    /// read path sees, the stale evidence a refresh recorded included.
    pub(super) fn read<R>(
        &self,
        query: impl FnOnce(&SessionProvider) -> R,
    ) -> Result<Option<R>, Poisoned> {
        let slot = self
            .provider
            .read()
            .map_err(|_| Poisoned { lock: "provider" })?;
        let answer = slot.as_ref().map(query);
        Ok(answer)
    }

    /// Run `change` on the published provider with exclusive access; `None` when nothing is
    /// published. This is the only way to the provider's `&mut`. Before it lets go of the slot
    /// it copies the provider's generation into the control plane, so `status` reports the
    /// generation of the index that is served once the change is over.
    pub(super) fn write<R>(
        &self,
        change: impl FnOnce(&mut SessionProvider) -> R,
    ) -> Result<Option<R>, Poisoned> {
        let mut slot = self
            .provider
            .write()
            .map_err(|_| Poisoned { lock: "provider" })?;
        let Some(provider) = slot.as_mut() else {
            return Ok(None);
        };
        let result = change(&mut *provider);
        let generation = provider.generation();
        self.record_generation(&slot, generation)?;
        Ok(Some(result))
    }

    /// Copy the provider's generation into the control plane. The write guard is an argument
    /// so that this cannot be called after the slot has been let go: two refreshes are then
    /// recorded in the order they happened. This is the permitted lock order (slot, then
    /// control).
    fn record_generation(
        &self,
        _held: &RwLockWriteGuard<'_, Option<SessionProvider>>,
        generation: u64,
    ) -> Result<(), Poisoned> {
        let mut control = self.lock_control()?;
        if let BuildPhase::Ready {
            generation: recorded,
        } = &mut control.build
        {
            *recorded = generation;
        }
        Ok(())
    }

    /// Wake every parked call so that it looks at the clock again.
    ///
    /// A running daemon does not need this: a parked call wakes by itself when its deadline
    /// passes. Tests that move a manual clock call it. The control lock is taken first, so a
    /// call that has read the time and not yet parked is woken too.
    pub(super) fn wake(&self) {
        let _control = self.control.lock();
        self.changed.notify_all();
    }

    fn lock_control(&self) -> Result<MutexGuard<'_, ControlState>, Poisoned> {
        self.control
            .lock()
            .map_err(|_| Poisoned { lock: "control" })
    }

    /// Start the next attempt and return its number and start time. Called with the control
    /// lock held, from `Idle` or `Failed` only; that is what makes it the one place a build
    /// thread comes from.
    fn start_attempt(self: &Arc<Self>, control: &mut ControlState) -> (u64, Instant) {
        let attempt = control.attempts.saturating_add(1);
        let started = self.clock.now();
        control.attempts = attempt;
        control.build = BuildPhase::Building { attempt, started };

        let state = Arc::clone(self);
        let job: BuildJob = Box::new(move || state.run_build(attempt));
        if let Err(error) = (self.spawner)(job) {
            // No thread, so nothing will ever end this attempt: end it here. Nobody can have
            // joined it yet, because the control lock has been held since it began.
            let error = format!("could not start the index build thread: {error}");
            control.last_error = Some(error.clone());
            control.build = BuildPhase::Failed { attempt, error };
        }
        (attempt, started)
    }

    /// The build thread: run the builder with no lock held, then publish or record the failure.
    fn run_build(self: Arc<Self>, attempt: u64) {
        let fail_on_panic = PanicGuard {
            state: &*self,
            attempt,
        };
        let built = (self.builder)();
        match built {
            Ok(provider) => self.publish(attempt, provider),
            Err(error) => self.settle(
                attempt,
                BuildPhase::Failed {
                    attempt,
                    error: error.to_string(),
                },
            ),
        }
        drop(fail_on_panic);
    }

    /// Put the provider in the slot, then announce `Ready`. In that order: a call that is told
    /// `Ready` goes straight to the slot.
    fn publish(&self, attempt: u64, provider: SessionProvider) {
        let generation = provider.generation();
        let published = match self.provider.write() {
            Ok(mut slot) => {
                *slot = Some(provider);
                true
            }
            Err(_) => false,
        };
        if !published {
            self.settle(
                attempt,
                BuildPhase::Failed {
                    attempt,
                    error: SLOT_POISONED.to_string(),
                },
            );
            return;
        }
        #[cfg(test)]
        if let Some(hook) = &self.hooks.published {
            hook(attempt);
        }
        self.settle(attempt, BuildPhase::Ready { generation });
    }

    /// Record how `attempt` ended and tell every parked call. The outcome is recorded only if
    /// the phase is still `Building` that attempt, so an attempt ends once and `Ready` is not
    /// left. A poisoned control lock is left alone; the parked calls find it when they wake.
    fn settle(&self, attempt: u64, outcome: BuildPhase) {
        if let Ok(mut control) = self.control.lock() {
            let still_building = matches!(
                &control.build,
                BuildPhase::Building { attempt: building, .. } if *building == attempt
            );
            if still_building {
                if let BuildPhase::Failed { error, .. } = &outcome {
                    control.last_error = Some(error.clone());
                }
                control.build = outcome;
            }
        }
        #[cfg(test)]
        if let Some(hook) = &self.hooks.settled {
            hook(attempt);
        }
        self.changed.notify_all();
    }
}

/// Ends the attempt as failed if the build thread unwinds, so that a panic in the builder
/// cannot leave the phase `Building` with nobody left to end it.
struct PanicGuard<'a> {
    state: &'a DaemonState,
    attempt: u64,
}

impl Drop for PanicGuard<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.state.settle(
                self.attempt,
                BuildPhase::Failed {
                    attempt: self.attempt,
                    error: BUILD_PANICKED.to_string(),
                },
            );
        }
    }
}

#[cfg(test)]
#[path = "coordinator_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "coordinator_failure_tests.rs"]
mod failure_tests;

#[cfg(test)]
#[path = "coordinator_order_tests.rs"]
mod order_tests;
