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
pub(super) use super::control::{AttemptOutcome, BuildPhase, BuildStatus, ControlState};
use crate::mcp::lazy::{Readiness, SessionBuilder};
// The tests in this file's child modules name `BuildState` through `use super::*`.
#[cfg(test)]
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{SessionProvider, StartupMode};
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use std::time::Instant;

/// What a call is told when the builder panicked. The in-process server uses the same text.
const BUILD_PANICKED: &str = "index build panicked";
/// What a call is told once every attempt number has been used.
const ATTEMPTS_EXHAUSTED: &str = "the index build attempt counter is exhausted";
/// The build thread's name. Linux keeps 15 bytes of it.
const BUILD_THREAD_NAME: &str = "prism-idx-build";
const SLOT_POISONED: &str =
    "the index was built and could not be published: the provider lock is poisoned";

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

/// The work of one build thread.
pub(super) type BuildJob = Box<dyn FnOnce() + Send>;

/// Starts a thread that runs a [`BuildJob`]. It is called with the control lock held, so it
/// must return promptly (every other caller of the control plane waits while it runs, and the
/// calling connection's deadline is not set until it has returned) and must not run the job on
/// the calling thread (the job ends by taking the control lock, which the caller still holds).
/// An error means no thread exists and the job will never run.
pub(super) type Spawner = Box<dyn Fn(BuildJob) -> std::io::Result<()> + Send + Sync>;

fn spawn_build_thread(job: BuildJob) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name(BUILD_THREAD_NAME.to_string())
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
                generation: 0,
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
    /// new one if none has been started or the last one failed. From then on it reads only that
    /// attempt's outcome, whatever the phase has become by the time it runs again, and returns
    ///
    /// - `Ready` once that attempt has published the index (or at once, if the index was ready
    ///   when the call arrived),
    /// - `Failed`, with that attempt's own error, if that attempt ended without an index. The
    ///   call does not start the retry (the next call does), does not join a retry that has
    ///   begun in the meantime, and is told of the failure even if a retry has since succeeded:
    ///   its next call finds the index ready,
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
            BuildPhase::Ready => return Ok(Readiness::Ready),
            BuildPhase::Building {
                attempt,
                started,
                outcome,
            } => Some((*attempt, *started, Arc::clone(outcome))),
            BuildPhase::Idle | BuildPhase::Failed { .. } => None,
        };
        let (attempt, started, outcome) =
            building.unwrap_or_else(|| self.start_attempt(&mut control));
        let deadline = budget.deadline_for(attempt, self.clock.now());

        loop {
            if let Some(answer) = outcome.answer() {
                return Ok(answer);
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
            generation: control.generation,
            waiting: control.waiting,
        })
    }

    /// Run `query` on the provider in the slot with shared access; `None` when the slot is
    /// empty. `query` gets the whole provider, so it sees what the in-process server's read
    /// path sees, the stale evidence a refresh recorded included.
    ///
    /// This looks at the slot as it is. The slot is filled a moment before `Ready` is
    /// announced, so a caller that has not been told `Ready` by `ensure_ready` can find a
    /// provider here while `status` still says `Building`.
    ///
    /// `query` must not call `read` or `write` again: the slot's lock is not re-entrant, and a
    /// nested call can deadlock. `status` is safe to call from it.
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

    /// Run `change` on the provider in the slot with exclusive access; `None` when the slot is
    /// empty. This is the only way to the provider's `&mut`. Before it lets go of the slot it
    /// copies the provider's generation into the control plane, so `status` reports the
    /// generation of the provider in the slot once the change is over. That holds whether or
    /// not `Ready` has been announced yet: like `read`, this looks at the slot as it is.
    ///
    /// `change` must not call `read` or `write` again: the slot's lock is not re-entrant, and
    /// a nested call deadlocks. `status` is safe to call from it.
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

    /// Copy the generation of the provider in the slot into the control plane, whatever the
    /// phase. The write guard is an argument so that this cannot be called after the slot has
    /// been let go: the control plane's generation then changes in the order the slot's
    /// content did. This is the permitted lock order (slot, then control).
    fn record_generation(
        &self,
        _held: &RwLockWriteGuard<'_, Option<SessionProvider>>,
        generation: u64,
    ) -> Result<(), Poisoned> {
        let mut control = self.lock_control()?;
        control.generation = generation;
        Ok(())
    }

    /// Empty the provider slot and leave the phase as it is. Only for the test of what a
    /// connection does if `Ready` were ever announced over an empty slot.
    #[cfg(test)]
    pub(super) fn unpublish_for_tests(&self) {
        if let Ok(mut slot) = self.provider.write() {
            *slot = None;
        }
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

    /// Start the next attempt and return its number, its start time and its outcome, still
    /// empty. Called with the control lock held, from `Idle` or `Failed` only; that is what
    /// makes it the one place a build thread comes from.
    ///
    /// An attempt's number is what a connection's budget is kept by, so a number is used once.
    /// When the counter cannot go on, no attempt is started: the phase is `Failed` and the
    /// outcome returned is already filled with that failure.
    fn start_attempt(
        self: &Arc<Self>,
        control: &mut ControlState,
    ) -> (u64, Instant, Arc<AttemptOutcome>) {
        let started = self.clock.now();
        let outcome = Arc::new(AttemptOutcome::default());
        let Some(attempt) = control.attempts.checked_add(1) else {
            let attempt = control.attempts;
            outcome.fill(Err(ATTEMPTS_EXHAUSTED.to_string()));
            control.build = BuildPhase::Failed {
                attempt,
                error: ATTEMPTS_EXHAUSTED.to_string(),
            };
            return (attempt, started, outcome);
        };
        control.attempts = attempt;
        control.build = BuildPhase::Building {
            attempt,
            started,
            outcome: Arc::clone(&outcome),
        };

        let state = Arc::clone(self);
        let job: BuildJob = Box::new(move || state.run_build(attempt));
        if let Err(error) = (self.spawner)(job) {
            // No thread, so nothing will ever end this attempt: end it here. Nobody but the
            // caller can have joined it, because the control lock has been held since it began.
            let error = format!("could not start the index build thread: {error}");
            outcome.fill(Err(error.clone()));
            control.build = BuildPhase::Failed { attempt, error };
        }
        (attempt, started, outcome)
    }

    /// The build thread: run the builder with no lock held, then publish or record the failure.
    fn run_build(self: Arc<Self>, attempt: u64) {
        let fail_on_panic = PanicGuard {
            state: &self,
            attempt,
        };
        let built = (self.builder)();
        match built {
            Ok(provider) => self.publish(attempt, provider),
            Err(error) => self.settle(attempt, Err(error.to_string())),
        }
        drop(fail_on_panic);
    }

    /// Put the provider in the slot and record its generation, then announce `Ready`. In that
    /// order: a call that is told `Ready` goes straight to the slot. The generation is recorded
    /// while the guard that stored the provider is still held, so exclusive access that lands
    /// before the announcement finds it recorded and records its own after it.
    fn publish(&self, attempt: u64, provider: SessionProvider) {
        let generation = provider.generation();
        let published = match self.provider.write() {
            Ok(mut slot) => {
                *slot = Some(provider);
                // A poisoned control lock is left alone here as it is in `settle`.
                let _recorded = self.record_generation(&slot, generation);
                true
            }
            Err(_) => false,
        };
        if !published {
            self.settle(attempt, Err(SLOT_POISONED.to_string()));
            return;
        }
        #[cfg(test)]
        if let Some(hook) = &self.hooks.published {
            hook(attempt);
        }
        self.settle(attempt, Ok(()));
    }

    /// Record how `attempt` ended, in its own outcome and in the phase, and tell every parked
    /// call. Nothing is recorded unless the phase is still `Building` that attempt, so an
    /// attempt ends once and `Ready` is not left. A poisoned control lock is left alone; the
    /// parked calls find it when they wake.
    fn settle(&self, attempt: u64, ended: Result<(), String>) {
        if let Ok(mut control) = self.control.lock() {
            let building = match &control.build {
                BuildPhase::Building {
                    attempt: building,
                    outcome,
                    ..
                } if *building == attempt => Some(Arc::clone(outcome)),
                _ => None,
            };
            if let Some(outcome) = building {
                control.build = match &ended {
                    Ok(()) => BuildPhase::Ready,
                    Err(error) => BuildPhase::Failed {
                        attempt,
                        error: error.clone(),
                    },
                };
                outcome.fill(ended);
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
            self.state
                .settle(self.attempt, Err(BUILD_PANICKED.to_string()));
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

#[cfg(test)]
#[path = "coordinator_edge_tests.rs"]
mod edge_tests;
