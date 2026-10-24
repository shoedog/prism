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
//! Panics (spec §3.1 "Fatal", §4.2 "Panic policy"): a panic in a query or a refresh run through
//! [`DaemonState::read`] or [`DaemonState::write`], on the build thread, or in the control plane
//! ends the daemon through `on_fatal`, and so does a lock found poisoned (`fatal.rs`). Nothing
//! is read through a poisoned lock.
use super::activity::ToolActivity;
use super::budget::ConnectionBudget;
use super::clock::{Clock, SystemClock};
use super::control::answer_for;
pub(super) use super::control::{
    Admission, Answer, AttemptOutcome, BuildPhase, BuildStatus, ControlState,
};
use super::fatal::{abort_daemon, Domain, Fatal, FatalOnUnwind, OnFatal};
use super::idle::{spawn_disposal_thread, IdleState};
use crate::mcp::lazy::SessionBuilder;
// The tests in this file's child modules name `BuildState` through `use super::*`.
#[cfg(test)]
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{SessionProvider, StartupMode};
use std::fmt;
use std::ops::{Deref, DerefMut};
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
/// is not read again. Every place that makes one has first ended the daemon (`poisoned`), so a
/// caller sees this error only when a test's `on_fatal` returns; it then ends its connection.
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
    pub(super) lifecycle: Option<super::log::EventObserver>,
    pub(super) wall_time: Option<Arc<dyn Fn() -> std::time::SystemTime + Send + Sync>>,
    pub(super) fingerprint: Option<Arc<dyn Fn() + Send + Sync>>,
    /// After the provider is in the slot, before `Ready` is announced.
    pub(super) published: Option<AttemptHook>,
    /// After an attempt's outcome is recorded, before the parked calls are notified.
    pub(super) settled: Option<AttemptHook>,
    /// On a connection's thread, after a query found the index stale under an automatic
    /// refresh policy and before the call asks for exclusive access.
    pub(super) exclusive: Option<Arc<dyn Fn() + Send + Sync>>,
    pub(super) dispatch: Option<Arc<dyn Fn() + Send + Sync>>,
    pub(super) response: Option<Arc<dyn Fn() + Send + Sync>>,
    pub(super) returned: Option<Arc<dyn Fn() + Send + Sync>>,
    pub(super) dispose: Option<Arc<dyn Fn() + Send + Sync>>,
}

/// What a [`DaemonState`] is made of. Production code uses [`Parts::production`]; tests replace
/// the clock, the spawner or the hooks.
pub(super) struct Parts {
    pub(super) builder: SessionBuilder,
    pub(super) startup: StartupMode,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) spawner: Spawner,
    pub(super) disposal_spawner: Spawner,
    /// What ends the daemon. Production: [`abort_daemon`].
    pub(super) on_fatal: OnFatal,
    #[cfg(test)]
    pub(super) hooks: TestHooks,
    pub(super) idle_timeout: Option<std::time::Duration>,
    pub(super) lifecycle: super::log::LifecycleLog,
}

impl Parts {
    pub(super) fn production(builder: SessionBuilder, startup: StartupMode) -> Self {
        Self {
            builder,
            startup,
            clock: Arc::new(SystemClock),
            spawner: Box::new(spawn_build_thread),
            disposal_spawner: Box::new(spawn_disposal_thread),
            on_fatal: Arc::new(abort_daemon),
            #[cfg(test)]
            hooks: TestHooks::default(),
            idle_timeout: None,
            lifecycle: super::log::LifecycleLog::default(),
        }
    }
}

/// One daemon's index build and the index it produced, shared by every connection.
pub(super) struct DaemonState {
    pub(super) control: Mutex<ControlState>,
    pub(super) changed: Condvar,
    pub(super) provider: RwLock<Option<SessionProvider>>,
    builder: SessionBuilder,
    startup: StartupMode,
    clock: Arc<dyn Clock>,
    spawner: Spawner,
    pub(super) disposal_spawner: Spawner,
    pub(super) idle_timeout: Option<std::time::Duration>,
    pub(super) lifecycle: super::log::LifecycleLog,
    pub(super) idle_waker: Mutex<Option<super::idle::Wake>>,
    pub(super) on_fatal: OnFatal,
    #[cfg(test)]
    pub(super) hooks: TestHooks,
}

impl DaemonState {
    /// A daemon state that builds with `builder`: on the first readiness call for
    /// `StartupMode::Lazy`, at once for `StartupMode::Background`.
    #[cfg_attr(not(test), allow(dead_code))]
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
                admission: Admission::new(),
                idle: IdleState::default(),
            }),
            changed: Condvar::new(),
            provider: RwLock::new(None),
            builder: parts.builder,
            startup: parts.startup,
            clock: parts.clock,
            spawner: parts.spawner,
            disposal_spawner: parts.disposal_spawner,
            idle_timeout: parts.idle_timeout.filter(|n| !n.is_zero()),
            lifecycle: parts.lifecycle.with_timeout(parts.idle_timeout),
            idle_waker: Mutex::new(None),
            on_fatal: parts.on_fatal,
            #[cfg(test)]
            hooks: parts.hooks,
        });
        if state.startup == StartupMode::Background {
            let mut event = None;
            {
                let mut control = state.lock_control()?;
                let _fatal = state.guard(Domain::Control);
                let _started = state.start_attempt(&mut control, &mut event);
            }
            if let Some(event) = event {
                state.lifecycle_event(event);
            }
        }
        Ok(state)
    }

    /// How this daemon's index is started. `initialize` advertises it (spec §5).
    pub(super) fn startup_mode(&self) -> StartupMode {
        self.startup
    }

    /// Make sure a build has been started and wait for it, no longer than `budget` allows.
    ///
    /// If the attempt this connection last joined has failed and no call of the connection has
    /// been told so (each was answered `Warming`), this call is answered about that attempt
    /// and starts nothing: `Failed` with its text, `Superseded`, or `Ready` when a later attempt
    /// has published the index (D32). Otherwise the call
    /// first decides which attempt it belongs to, once: the one that is building, or a
    /// new one if none has been started or the last one failed. From then on it reads only that
    /// attempt's outcome, and returns
    ///
    /// - `Ready` once that attempt has published the index (or at once, if the index was ready
    ///   when the call arrived), and also when that attempt failed and, by the time the call
    ///   runs again, a later attempt has published the index (the owner's decision of
    ///   2026-10-13),
    /// - `Failed`, with that attempt's own error, if that attempt ended without an index and
    ///   nothing has superseded it. The call does not start the retry; the next call does,
    /// - `Superseded` if that attempt ended without an index and, by the time the call runs
    ///   again, a later attempt is building. The failure is stale then, and the call is told
    ///   to retry instead of being given it,
    /// - `Warming` when this connection's deadline for that attempt has passed. That is this
    ///   connection's wait ending; the build goes on and nobody else's deadline changes.
    ///
    /// Starting a new attempt and joining a running one happen under one acquisition of the
    /// control lock, so however many calls find the build failed, one of them starts the retry
    /// and the rest join it.
    #[cfg(test)]
    pub(super) fn ensure_ready(
        self: &Arc<Self>,
        budget: &mut ConnectionBudget,
    ) -> Result<Answer, Poisoned> {
        let mut activity = self.admit_tool()?;
        self.ensure_serving(budget, &mut activity)
    }

    pub(super) fn ensure_serving(
        self: &Arc<Self>,
        budget: &mut ConnectionBudget,
        activity: &mut ToolActivity,
    ) -> Result<Answer, Poisoned> {
        let mut control = self.lock_control()?;
        let _fatal = self.guard(Domain::Control);
        let mut disposal_deadline = None;
        while control.idle.disposing && control.admission.draining.is_none() {
            let now = self.now();
            let deadline = *disposal_deadline.get_or_insert_with(|| budget.wait_deadline(now));
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                return Ok(Answer::Warming {
                    elapsed: std::time::Duration::ZERO,
                });
            }
            control.waiting += 1;
            control = self
                .changed
                .wait_timeout(control, remaining)
                .map_err(|_| self.poisoned("control"))?
                .0;
            control.waiting -= 1;
        }
        if self.idle_timeout.is_some()
            && (control.admission.draining.is_some() || control.idle.disposal_failed)
        {
            return Ok(Answer::Failed {
                error: if control.admission.draining.is_some() {
                    "the daemon is draining"
                } else {
                    "a previous index disposal failed"
                }
                .into(),
            });
        }
        if let Some(error) = budget.unheard_failure_since(control.idle.watermark) {
            let answer = answer_for(&Err(error), &control.build);
            if answer == Answer::Ready {
                activity.serving(&mut control);
            }
            return Ok(answer);
        }
        let building = match &control.build {
            BuildPhase::Ready => {
                activity.serving(&mut control);
                return Ok(Answer::Ready);
            }
            BuildPhase::Building {
                attempt,
                started,
                outcome,
            } => Some((*attempt, *started, Arc::clone(outcome))),
            BuildPhase::Idle | BuildPhase::Unloaded { .. } | BuildPhase::Failed { .. } => None,
        };
        let mut event = None;
        let (attempt, started, outcome) =
            building.unwrap_or_else(|| self.start_attempt(&mut control, &mut event));
        let mut deadline = budget.join(attempt, &outcome, self.now());
        if let Some(limit) = disposal_deadline {
            deadline = budget.cap_deadline(limit);
        }
        if let Some(event) = event {
            // Spawn failure is already terminal. Freeze this caller's answer before another
            // client can retry, then release both guards before the potentially blocking sink.
            budget.told();
            let answer = answer_for(
                outcome.ended().expect("spawn failure ended"),
                &control.build,
            );
            drop(_fatal);
            drop(control);
            self.lifecycle_event(event);
            return Ok(answer);
        }
        loop {
            if self.idle_timeout.is_some()
                && (control.admission.draining.is_some() || control.idle.disposal_failed)
            {
                return Ok(Answer::Failed {
                    error: if control.admission.draining.is_some() {
                        "the daemon is draining"
                    } else {
                        "a previous index disposal failed"
                    }
                    .into(),
                });
            }
            if let Some(ended) = outcome.ended() {
                budget.told();
                let answer = answer_for(ended, &control.build);
                if answer == Answer::Ready {
                    activity.serving(&mut control);
                }
                return Ok(answer);
            }
            let now = self.now();
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                return Ok(Answer::Warming {
                    elapsed: now.saturating_duration_since(started),
                });
            }
            control.waiting += 1;
            control = self
                .changed
                .wait_timeout(control, remaining)
                .map_err(|_| self.poisoned("control"))?
                .0;
            control.waiting -= 1;
        }
    }

    /// The control plane's view of the build.
    #[cfg_attr(not(test), allow(dead_code))] // read by tests only
    pub(super) fn status(&self) -> Result<BuildStatus, Poisoned> {
        Ok(self.control_plane()?.build_status())
    }

    /// The control lock, held, with the control plane's panic guard made right after it (D47).
    /// Every entry point S4 adds takes the lock through this, so the order is the type's.
    pub(super) fn control_plane(&self) -> Result<ControlPlane<'_>, Poisoned> {
        let control = self.lock_control()?;
        Ok(ControlPlane {
            _fatal: self.guard(Domain::Control),
            control,
        })
    }

    /// The daemon's clock.
    pub(super) fn now(&self) -> Instant {
        self.clock.now()
    }

    /// Whether the control lock is held right now, by this thread or another. A poisoned lock
    /// that nobody holds is not held. Tests only.
    #[cfg(test)]
    pub(super) fn control_is_held(&self) -> bool {
        matches!(
            self.control.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        )
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
    ///
    /// A panic in `query` ends the daemon (provider dispatch, spec §3.1).
    pub(super) fn read<R>(
        &self,
        query: impl FnOnce(&SessionProvider) -> R,
    ) -> Result<Option<R>, Poisoned> {
        let slot = self
            .provider
            .read()
            .map_err(|_| self.poisoned("provider"))?;
        let _fatal = self.guard(Domain::Dispatch);
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
    ///
    /// A panic in `change` ends the daemon (provider dispatch, spec §3.1): a refresh that
    /// unwinds may have replaced part of the provider and not the rest.
    pub(super) fn write<R>(
        &self,
        change: impl FnOnce(&mut SessionProvider) -> R,
    ) -> Result<Option<R>, Poisoned> {
        let mut slot = self
            .provider
            .write()
            .map_err(|_| self.poisoned("provider"))?;
        let _fatal = self.guard(Domain::Dispatch);
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

    /// The test seam between a call's read and its exclusive access (`TestHooks::exclusive`).
    /// Called with no lock held.
    #[cfg(test)]
    pub(super) fn before_exclusive(&self) {
        if let Some(hook) = &self.hooks.exclusive {
            hook();
        }
    }

    /// Wake every parked call so that it looks at the clock again.
    ///
    /// A running daemon does not need this: a parked call wakes by itself when its deadline
    /// passes. Tests that move a manual clock call it. The control lock is taken first, so a
    /// call that has read the time and not yet parked is woken too.
    #[cfg(test)]
    pub(super) fn wake(&self) {
        let _control = self.control.lock();
        self.changed.notify_all();
    }

    fn lock_control(&self) -> Result<MutexGuard<'_, ControlState>, Poisoned> {
        self.control.lock().map_err(|_| self.poisoned("control"))
    }

    /// Ends the daemon if this thread unwinds before the guard is dropped (`fatal.rs`).
    ///
    /// Each caller makes it right after it has taken its lock, never before: locals are dropped
    /// in reverse order, so the guard then fires while the lock is still held, and in
    /// production the process aborts before another thread can take the lock (a refresh
    /// waiting on a reader that panicked, say). Taking the lock cannot panic; a poisoned lock
    /// is reported by `poisoned`.
    pub(super) fn guard(&self, domain: Domain) -> FatalOnUnwind<'_> {
        FatalOnUnwind::new(&self.on_fatal, domain)
    }

    /// `lock` was found poisoned: end the daemon (spec §4.2, "no lock is ever unwrapped past
    /// poisoning"). The error returned reaches the caller only when a test's `on_fatal`
    /// returns.
    pub(super) fn poisoned(&self, lock: &'static str) -> Poisoned {
        (self.on_fatal)(Fatal::Poisoned { lock });
        Poisoned { lock }
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
        event: &mut Option<super::log::Event>,
    ) -> (u64, Instant, Arc<AttemptOutcome>) {
        if let BuildPhase::Unloaded { last_published } = control.build {
            control.idle.previous = Some(last_published);
        }
        let started = self.clock.now();
        self.lifecycle.start_clock(started);
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
            *event = Some(self.log_build(control, &Err(error.clone())));
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
    fn publish(&self, attempt: u64, mut provider: SessionProvider) {
        // Rejected late providers stay on this build thread and are dropped after both locks.
        let accepted = {
            let mut slot = match self.provider.write() {
                Ok(slot) => slot,
                Err(_) => {
                    let _ = self.poisoned("provider");
                    self.settle(attempt, Err(SLOT_POISONED.into()));
                    return;
                }
            };
            let mut control = match self.control_plane() {
                Ok(control) => control,
                Err(_) => {
                    // Only a returning test fatal sink reaches this: retain the resident
                    // coordinator's post-fatal publication/settlement observations, without
                    // ever reading the poisoned control state or authorizing service.
                    *slot = Some(provider);
                    drop(slot);
                    self.settle(attempt, Err("the control lock is poisoned".into()));
                    return;
                }
            };
            if (self.idle_timeout.is_some() && control.admission.draining.is_some())
                || control.idle.disposal_failed
                || !matches!(control.build, BuildPhase::Building { attempt: a, .. } if a == attempt)
            {
                false
            } else {
                if let Some(previous) = control.idle.previous {
                    provider.resume_idle_generation(previous);
                }
                control.generation = provider.generation();
                debug_assert!(slot.is_none());
                *slot = Some(provider);
                true
            }
        };
        if !accepted {
            self.settle(
                attempt,
                Err("the daemon is draining; obsolete publication rejected".into()),
            );
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
    /// attempt ends once and `Ready` is not left. A poisoned control lock ends the daemon; under
    /// a test's `on_fatal` the parked calls find it when they wake.
    fn settle(&self, attempt: u64, mut ended: Result<(), String>) {
        // Successful publication is finalized in slot → control order. A drain may have
        // arrived after storing the provider but before announcing it; with unloading enabled,
        // retract on this build thread, never in the draining server loop or under either lock.
        let mut slot = if ended.is_ok() {
            match self.provider.write() {
                Ok(slot) => Some(slot),
                Err(_) => {
                    let _ = self.poisoned("provider");
                    self.settle(attempt, Err(SLOT_POISONED.into()));
                    return;
                }
            }
        } else {
            None
        };
        let mut retired = None;
        let mut event = None;
        let control = match self.control.lock() {
            Ok(control) => Some(control),
            Err(_) => {
                (self.on_fatal)(Fatal::Poisoned { lock: "control" });
                None
            }
        };
        if let Some(mut control) = control {
            let _fatal = self.guard(Domain::Control);
            if ended.is_ok() && self.idle_timeout.is_some() && control.admission.draining.is_some()
            {
                retired = slot.as_mut().and_then(|slot| slot.take());
                control.generation = 0;
                ended = Err("the daemon is draining; obsolete publication rejected".into());
            }
            let building = match &control.build {
                BuildPhase::Building {
                    attempt: building,
                    outcome,
                    ..
                } if *building == attempt => Some(Arc::clone(outcome)),
                _ => None,
            };
            if let Some(outcome) = building {
                event = Some(self.log_build(&control, &ended));
                control.build = match &ended {
                    Ok(()) => {
                        control.idle.published_at = Some(self.now());
                        control.idle.watermark = attempt;
                        BuildPhase::Ready
                    }
                    Err(error) => BuildPhase::Failed {
                        attempt,
                        error: error.clone(),
                    },
                };
                outcome.fill(ended);
            }
        }
        drop(slot);
        drop(retired);
        #[cfg(test)]
        if let Some(hook) = &self.hooks.settled {
            hook(attempt);
        }
        self.changed.notify_all();
        self.wake_idle_loop();
        if let Some(event) = event {
            self.lifecycle_event(event);
        }
    }
}

/// The control lock and the guard made right after it. Fields drop in declaration order, so the
/// guard fires while the lock is still held (D47).
pub(super) struct ControlPlane<'a> {
    _fatal: FatalOnUnwind<'a>,
    control: MutexGuard<'a, ControlState>,
}

impl Deref for ControlPlane<'_> {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl DerefMut for ControlPlane<'_> {
    fn deref_mut(&mut self) -> &mut ControlState {
        &mut self.control
    }
}

/// Ends the daemon if the build thread unwinds (spec §3.1: a panic on the build thread is
/// fatal). Under a test's `on_fatal`, which returns, it then ends the attempt as failed, so
/// that the phase is not left `Building` with nobody to end it.
struct PanicGuard<'a> {
    state: &'a DaemonState,
    attempt: u64,
}

impl Drop for PanicGuard<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            (self.state.on_fatal)(Fatal::Panicked(Domain::Build));
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
