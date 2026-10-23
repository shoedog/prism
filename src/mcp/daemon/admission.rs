//! Admission and retirement (spec §4.2 "Admission and retirement", §3.1 "Draining"): the
//! counts of pending and admitted connections and the decision to drain, all under the control
//! lock, so that an admission and a retirement cannot interleave.
//!
//! Every function here is an entry point into the control plane and takes the lock through
//! `DaemonState::control_plane`, which makes the control plane's panic guard right after the
//! lock (D47): a panic or a poisoned lock in any of them ends the daemon.
use super::budget::ConnectionBudget;
use super::control::{AttemptOutcome, BuildPhase, BuildStatus, StopReason};
use super::coordinator::{DaemonState, Poisoned};
use crate::mcp::settings::ClientSettings;
use crate::mcp::StartupMode;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What the handshake stage does with a connection it has just accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    /// Counted as pending: read its `HELLO`.
    Pending,
    /// Pending and admitted connections are at the bound: answer `BUSY` and close it.
    Busy { max_clients: u32 },
    /// The daemon drains: answer `RETIRING` and close it.
    Retiring,
}

/// What a pending connection whose `HELLO` asked for the served identity is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Admit {
    /// Counted as admitted: write `READY` with this build status. If the write fails, call
    /// `release`.
    Ready(BuildStatus),
    /// The daemon drains: answer `RETIRING` and close it. `admit` has already taken this
    /// connection out of the pending count, so the caller must not call `leave_handshake`.
    Retiring,
}

/// What a control request is answered (spec §4.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Control {
    /// The daemon's state, read in the critical section in which the request left the counts.
    Status(ControlSnapshot),
    /// The daemon drains: answer `RETIRING`. Nothing was recorded.
    Retiring,
}

/// The control plane's half of a status reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ControlSnapshot {
    pub(super) build: BuildStatus,
    /// Admitted and pending connections, the request itself not counted.
    pub(super) admitted: u32,
    pub(super) pending: u32,
    /// How long nothing has been admitted or pending; `None` while something is.
    pub(super) idle_for: Option<Duration>,
    /// Whether a shutdown has been requested, by this request or an earlier one.
    pub(super) shutdown: bool,
}

/// The connection counts, for tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))] // read by tests only
pub(super) struct AdmissionStatus {
    pub(super) pending: u32,
    pub(super) admitted: u32,
    pub(super) draining: Option<StopReason>,
}

impl DaemonState {
    /// Count a connection the accept loop has just accepted as pending, unless the daemon
    /// drains or pending and admitted connections together are at `max_clients` (spec §4.2:
    /// the bound is over both).
    pub(super) fn enter_handshake(&self, max_clients: u32) -> Result<Entry, Poisoned> {
        let mut control = self.control_plane()?;
        let admission = &mut control.admission;
        if admission.draining.is_some() {
            return Ok(Entry::Retiring);
        }
        if admission.pending.saturating_add(admission.admitted) >= max_clients {
            return Ok(Entry::Busy { max_clients });
        }
        admission.pending = admission.pending.saturating_add(1);
        admission.idle_since = None;
        Ok(Entry::Pending)
    }

    /// A pending connection left the handshake stage without being admitted: it timed out,
    /// closed, sent a line that was refused, or was answered `MISMATCH`, or was answered
    /// `RETIRING` by the drain (which empties the connections that were still pending). It is
    /// the only way out of the pending count besides `admit`: a connection that `enter_handshake`
    /// answered `Entry::Retiring` or `Busy` was never counted, and one that `admit` answered
    /// `Admit::Retiring` was uncounted by `admit`, so neither calls this.
    pub(super) fn leave_handshake(&self) -> Result<(), Poisoned> {
        let mut control = self.control_plane()?;
        // Read inside the critical section: a time read before the lock can be older than a
        // change another thread makes in between (review round 1).
        let now = self.now();
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        admission.left(now);
        Ok(())
    }

    /// Admit a pending connection whose `HELLO` asked for the served identity: in one
    /// acquisition of the control lock it stops being pending and, unless the daemon drains,
    /// becomes admitted, and the build status for its `READY` is read. The count goes up before
    /// `READY` is written (spec §4.2); the caller calls `release` if the write fails.
    pub(super) fn admit(&self) -> Result<Admit, Poisoned> {
        let mut control = self.control_plane()?;
        // Read inside the critical section: a time read before the lock can be older than a
        // change another thread makes in between (review round 1).
        let now = self.now();
        let status = control.build_status();
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        if admission.draining.is_some() {
            admission.left(now);
            return Ok(Admit::Retiring);
        }
        admission.admitted = admission.admitted.saturating_add(1);
        admission.idle_since = None;
        Ok(Admit::Ready(status))
    }

    /// An admitted connection ended: its serve loop returned, or its `READY` could not be
    /// written, or its worker could not be started.
    pub(super) fn release(&self) -> Result<(), Poisoned> {
        let mut control = self.control_plane()?;
        // Read inside the critical section: a time read before the lock can be older than a
        // change another thread makes in between (review round 1).
        let now = self.now();
        let admission = &mut control.admission;
        admission.admitted = admission.admitted.saturating_sub(1);
        admission.left(now);
        Ok(())
    }

    /// The retirement decision (spec §4.2): if nothing has been admitted or pending for
    /// `idle_ttl` and no build is running, the daemon drains from now on, and `unlink` (the
    /// socket's removal) runs inside the same critical section. An operator's shutdown request
    /// drains it as soon as nothing is admitted or pending, whatever the TTL and a running build
    /// (spec §4.7, D100). Returns why the daemon drains, if it does, whether it started now or
    /// earlier.
    pub(super) fn retire_if_idle(
        &self,
        idle_ttl: Duration,
        unlink: impl FnOnce(),
    ) -> Result<Option<StopReason>, Poisoned> {
        let mut control = self.control_plane()?;
        // Read inside the critical section: a time read before the lock can be older than a
        // change another thread makes in between (review round 1).
        let now = self.now();
        if let Some(reason) = control.admission.draining {
            return Ok(Some(reason));
        }
        let admission = &mut control.admission;
        if admission.pending != 0 || admission.admitted != 0 {
            return Ok(None);
        }
        if admission.shutdown {
            admission.draining = Some(StopReason::Shutdown);
            unlink();
            return Ok(Some(StopReason::Shutdown));
        }
        let since = *admission.idle_since.get_or_insert(now);
        admission.quiet_since.get_or_insert(since);
        if now.saturating_duration_since(since) < idle_ttl
            || matches!(control.build, BuildPhase::Building { .. })
            || control.idle.disposing
            || control.idle.pending != 0
            || control.idle.leases != 0
        {
            return Ok(None);
        }
        control.admission.draining = Some(StopReason::Idle);
        unlink();
        Ok(Some(StopReason::Idle))
    }

    /// Drain whatever the counts say (root lost or replaced, files replaced, a signal). The
    /// first reason recorded is kept; `unlink` runs inside the critical section that records
    /// it, and not again. Returns the reason the daemon drains for.
    pub(super) fn drain(
        &self,
        reason: StopReason,
        unlink: impl FnOnce(),
    ) -> Result<StopReason, Poisoned> {
        let mut control = self.control_plane()?;
        if let Some(earlier) = control.admission.draining {
            return Ok(earlier);
        }
        control.admission.draining = Some(reason);
        unlink();
        self.notify_draining();
        Ok(reason)
    }

    /// When the idle TTL runs out if nothing changes: `None` while a connection is pending or
    /// admitted, and before the first retirement decision. The accept loop sleeps no longer
    /// than this.
    pub(super) fn idle_deadline(&self, idle_ttl: Duration) -> Result<Option<Instant>, Poisoned> {
        let control = self.control_plane()?;
        Ok(control
            .admission
            .idle_since
            .and_then(|since| since.checked_add(idle_ttl)))
    }

    /// A pending connection whose line was a control request leaves the handshake stage as if it
    /// had never been counted: if nothing is left pending or admitted, the idle time is the one
    /// it found (`quiet_since`), so an operator's probe does not keep an idle daemon up (S4's
    /// interface for S5b). With `shutdown`, the daemon drains once nothing is admitted or pending
    /// (D100). Unless the daemon drains already, the answer is its state, read in the same
    /// critical section, after the request has left the counts.
    pub(super) fn control_exchange(&self, shutdown: bool) -> Result<Control, Poisoned> {
        let mut control = self.control_plane()?;
        // Read inside the critical section, as every admission function does (review round 1).
        let now = self.now();
        let build = control.build_status();
        let admission = &mut control.admission;
        admission.pending = admission.pending.saturating_sub(1);
        let idle = admission.pending == 0 && admission.admitted == 0;
        if idle {
            admission.idle_since = admission.quiet_since;
        }
        if admission.draining.is_some() {
            return Ok(Control::Retiring);
        }
        admission.shutdown |= shutdown;
        Ok(Control::Status(ControlSnapshot {
            build,
            admitted: admission.admitted,
            pending: admission.pending,
            idle_for: admission
                .idle_since
                .filter(|_| idle)
                .map(|since| now.saturating_duration_since(since)),
            shutdown: admission.shutdown,
        }))
    }

    #[cfg_attr(not(test), allow(dead_code))] // read by tests only
    pub(super) fn admission_status(&self) -> Result<AdmissionStatus, Poisoned> {
        let control = self.control_plane()?;
        Ok(AdmissionStatus {
            pending: control.admission.pending,
            admitted: control.admission.admitted,
            draining: control.admission.draining,
        })
    }

    /// The first-call budget of a connection that is about to be served. A connection of a warm
    /// daemon (`StartupMode::Background`) whose startup attempt is still the only attempt is
    /// given that attempt, so that its first call hears it fail as the in-process server's
    /// first call does, instead of starting the retry (S3a's parity requirement for S4). Every
    /// other connection starts with an empty budget.
    pub(super) fn budget_for(&self, client: &ClientSettings) -> Result<ConnectionBudget, Poisoned> {
        let control = self.control_plane()?;
        if self.startup_mode() != StartupMode::Background || control.attempts != 1 {
            return Ok(ConnectionBudget::new(client));
        }
        let startup = match &control.build {
            BuildPhase::Building {
                attempt: 1,
                outcome,
                ..
            } => Arc::clone(outcome),
            BuildPhase::Failed { attempt: 1, error } => {
                let outcome = Arc::new(AttemptOutcome::default());
                outcome.fill(Err(error.clone()));
                outcome
            }
            _ => return Ok(ConnectionBudget::new(client)),
        };
        Ok(ConnectionBudget::joined_at_startup(client, startup))
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "admission_control_tests.rs"]
mod control_tests;
