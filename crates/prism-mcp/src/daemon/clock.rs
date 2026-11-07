//! The daemon's clock, injected so that tests can move time instead of waiting for it.
use std::time::Instant;

/// Where the coordinator reads the time: when an attempt started, and whether a call's
/// deadline has passed.
///
/// A clock must make progress. A parked call sleeps for what is left of its budget in real
/// time and then reads this clock again; if the clock has not moved, the call sleeps again,
/// so a clock that stands still keeps a call parked for as long as its attempt runs. A clock
/// that goes backwards is treated as one that has not moved.
pub(super) trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

/// The clock a running daemon uses.
pub(super) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
