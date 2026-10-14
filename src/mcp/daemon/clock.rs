//! The daemon's clock, injected so that tests can move time instead of waiting for it.
use std::time::Instant;

/// Where the coordinator reads the time: when an attempt started, and whether a call's
/// deadline has passed.
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
