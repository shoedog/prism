//! One connection's first-call wait (spec §4.2 "Readiness semantics", §5 "connection-local").
use crate::mcp::settings::ClientSettings;
use std::time::{Duration, Instant};

/// How long one connection may wait for the index, counted separately for each build attempt.
///
/// A connection's deadline for an attempt is set by its first call that arrives during that
/// attempt and is kept for every later call during it, so a client that sends several calls
/// while one build runs waits its budget once, not once per call. A new attempt (a retry after
/// a failed build) starts a new deadline.
///
/// The only way to make one is from a [`ClientSettings`], whose constructor has already
/// refused a wait above `FIRST_CALL_WAIT_MAX`. The budget bounds the wait for the build; it
/// says nothing about how long a query then takes.
pub(super) struct ConnectionBudget {
    wait: Duration,
    /// The attempt this connection last joined, and its deadline for that attempt.
    joined: Option<(u64, Instant)>,
}

impl ConnectionBudget {
    pub(super) fn new(client: &ClientSettings) -> Self {
        Self {
            wait: client.first_call_wait(),
            joined: None,
        }
    }

    /// This connection's deadline for `attempt`, for a call that arrives at `now`.
    ///
    /// If `now + wait` cannot be represented the deadline is `now`: the call does not wait.
    /// No `ClientSettings` can ask for a wait that long.
    pub(super) fn deadline_for(&mut self, attempt: u64, now: Instant) -> Instant {
        if let Some((joined, deadline)) = self.joined {
            if joined == attempt {
                return deadline;
            }
        }
        let deadline = now.checked_add(self.wait).unwrap_or(now);
        self.joined = Some((attempt, deadline));
        deadline
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
