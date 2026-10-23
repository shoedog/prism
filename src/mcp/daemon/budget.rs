//! One connection's first-call wait (spec §4.2 "Readiness semantics", §5 "connection-local"),
//! and what the connection has heard of the build attempt it last joined.
use super::coordinator::AttemptOutcome;
use crate::mcp::settings::ClientSettings;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long one connection may wait for the index, counted separately for each build attempt,
/// and whether the connection has heard how that attempt ended.
///
/// A connection's deadline for an attempt is set by its first call that arrives during that
/// attempt and is kept for every later call during it, so a client that sends several calls
/// while one build runs waits its budget once, not once per call. A new attempt (a retry after
/// a failed build) starts a new deadline.
///
/// The budget also keeps the outcome of the attempt the connection last joined. If every call
/// of the connection for that attempt was answered warming and the attempt then failed, the
/// connection's next call hears that failure before anything is started (D32); that is the
/// in-process server's warming, then failed, then retry.
///
/// The only way to make one is from a [`ClientSettings`], whose constructor has already
/// refused a wait above `FIRST_CALL_WAIT_MAX`. The budget bounds the wait for the build; it
/// says nothing about how long a query then takes.
pub(super) struct ConnectionBudget {
    wait: Duration,
    joined: Option<Joined>,
}

/// The attempt a connection last joined.
struct Joined {
    attempt: u64,
    /// This connection's deadline for the attempt; `None` until the connection's first call
    /// during it, for a connection that was given the startup attempt at admission.
    deadline: Option<Instant>,
    /// The attempt's outcome, shared with the coordinator and every call that joined it.
    outcome: Arc<AttemptOutcome>,
    /// Whether a call of this connection has been told how the attempt ended.
    told: bool,
}

impl ConnectionBudget {
    pub(super) fn new(client: &ClientSettings) -> Self {
        Self {
            wait: client.first_call_wait(),
            joined: None,
        }
    }

    /// The budget of a connection admitted while a warm daemon's startup attempt (attempt 1,
    /// started by `--warm-at-startup` before any connection existed) is its only one: the
    /// connection holds that attempt as if it had joined it, without a deadline until its first
    /// call. Its first call then hears that attempt's failure, if it failed, instead of starting
    /// the retry: the in-process server's `build_failed`, then the retry on the next call.
    pub(super) fn joined_at_startup(client: &ClientSettings, outcome: Arc<AttemptOutcome>) -> Self {
        Self {
            wait: client.first_call_wait(),
            joined: Some(Joined {
                attempt: 1,
                deadline: None,
                outcome,
                told: false,
            }),
        }
    }

    /// Join `attempt`, whose outcome is `outcome`, for a call that arrives at `now`, and return
    /// this connection's deadline for it. A call during the attempt the connection already
    /// joined keeps that deadline; a call during another attempt starts a new one.
    ///
    /// If `now + wait` cannot be represented the deadline is `now`: the call does not wait.
    /// No `ClientSettings` can ask for a wait that long.
    pub(super) fn join(
        &mut self,
        attempt: u64,
        outcome: &Arc<AttemptOutcome>,
        now: Instant,
    ) -> Instant {
        let deadline = now.checked_add(self.wait).unwrap_or(now);
        if let Some(joined) = &mut self.joined {
            if joined.attempt == attempt {
                return *joined.deadline.get_or_insert(deadline);
            }
        }
        self.joined = Some(Joined {
            attempt,
            deadline: Some(deadline),
            outcome: Arc::clone(outcome),
            told: false,
        });
        deadline
    }

    /// One call's fixed disposal allowance; it also caps any build joined by that call.
    pub(super) fn wait_deadline(&self, now: Instant) -> Instant {
        now.checked_add(self.wait).unwrap_or(now)
    }

    pub(super) fn cap_deadline(&mut self, limit: Instant) -> Instant {
        let joined = self.joined.as_mut().expect("joined before deadline cap");
        let deadline = joined.deadline.unwrap_or(limit).min(limit);
        joined.deadline = Some(deadline);
        deadline
    }

    /// A later publication supersedes this connection's old error, even after unload.
    pub(super) fn unheard_failure_since(&mut self, watermark: u64) -> Option<String> {
        if self.joined.as_ref().is_some_and(|j| j.attempt < watermark) {
            self.told();
            return None;
        }
        self.unheard_failure()
    }

    /// Record that a call of this connection has been told how the attempt it joined ended.
    pub(super) fn told(&mut self) {
        if let Some(joined) = &mut self.joined {
            joined.told = true;
        }
    }

    /// The error of the attempt this connection last joined, if that attempt failed and no
    /// call of this connection has been told so. Records it as told: a connection hears its
    /// attempt's failure once.
    pub(super) fn unheard_failure(&mut self) -> Option<String> {
        let joined = self.joined.as_mut()?;
        if joined.told {
            return None;
        }
        match joined.outcome.ended() {
            Some(Err(error)) => {
                joined.told = true;
                Some(error.clone())
            }
            Some(Ok(())) | None => None,
        }
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
