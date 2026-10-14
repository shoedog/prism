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
    /// This connection's deadline for the attempt.
    deadline: Instant,
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
        if let Some(joined) = &self.joined {
            if joined.attempt == attempt {
                return joined.deadline;
            }
        }
        let deadline = now.checked_add(self.wait).unwrap_or(now);
        self.joined = Some(Joined {
            attempt,
            deadline,
            outcome: Arc::clone(outcome),
            told: false,
        });
        deadline
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
