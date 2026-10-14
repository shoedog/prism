//! Tests for the ways an attempt can end without a build: the thread cannot be started, the
//! builder panics, a lock is poisoned. In each, every parked call must get an answer.
use super::super::test_support::{
    build_after, fixture, panic_now, parts_with, scripted, state_with, status, wait_until, Conn,
    Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::RefreshPolicy;
use std::sync::atomic::{AtomicU64, Ordering};

fn built(calls: &AtomicU64) -> u64 {
    calls.load(Ordering::SeqCst)
}

/// `std::thread::spawn` panics when the thread cannot be created, and the first tool call of
/// any client reaches that line. Rules out ignoring the spawner's error (the call would then
/// wait for a build that never runs).
#[test]
fn a_build_thread_that_cannot_be_started_is_a_failed_attempt_and_the_next_call_retries() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let refusals_left = AtomicU64::new(1);
    let spawner: Spawner = Box::new(move |job: BuildJob| {
        if refusals_left.swap(0, Ordering::SeqCst) == 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "no more threads",
            ));
        }
        std::thread::Builder::new().spawn(job).map(|_handle| ())
    });
    parts.spawner = spawner;
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("could not start the index build thread: no more threads".to_string())
    );
    assert_eq!(
        status(&state),
        BuildStatus {
            state: BuildState::Failed,
            attempt: 1,
            generation: 0,
            waiting: 0
        }
    );
    assert_eq!(built(&calls), 0, "the builder never ran");

    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).attempt, 2);
    assert_eq!(built(&calls), 1);
}

/// S3a's interim behavior (slice S3b turns a builder panic into the daemon-fatal abort of spec
/// §3.1). Rules out a panic that leaves the phase `Building` forever.
#[test]
fn a_builder_panic_is_a_failed_attempt_every_waiter_hears_of() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![panic_now()]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 600);

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("index build panicked".to_string())
    );
    assert_eq!(status(&state).state, BuildState::Failed);
    assert_eq!(status(&state).waiting, 0);

    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 2);
}

/// Rules out recording an outcome without looking at the phase: a late `Failed` after `Ready`
/// would let the next call rebuild and replace an index that has since been refreshed.
#[test]
fn an_attempt_s_outcome_is_recorded_once_and_ready_is_never_left() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 600);

    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);

    // An outcome for an attempt that is not the one building is ignored.
    state.settle(9, BuildPhase::Ready { generation: 0 });
    assert_eq!(status(&state).state, BuildState::Building);
    assert_eq!(state.read(|_| ()), Ok(None));

    gate.open();
    assert_eq!(conn.outcome(), Outcome::Ready);

    // So is a second outcome for the attempt that already ended.
    state.settle(
        1,
        BuildPhase::Failed {
            attempt: 1,
            error: "too late".to_string(),
        },
    );
    assert_eq!(status(&state).state, BuildState::Ready);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 1, "nothing was rebuilt");
}

fn poison_the_control_lock(state: &Arc<DaemonState>) {
    let state = Arc::clone(state);
    let panicked = std::thread::spawn(move || {
        let _control = state.control.lock();
        panic!("poisoning the control lock for a test");
    })
    .join();
    assert!(panicked.is_err());
}

/// Rules out `unwrap` on the lock (a panic on a client's path) and a wait that never looks at
/// the lock again (a hang).
#[test]
fn a_poisoned_control_lock_is_an_error_for_every_caller_never_a_panic_or_a_hang() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let parked = Conn::open(&state, 600);

    parked.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    poison_the_control_lock(&state);
    state.wake();

    assert_eq!(parked.outcome(), Outcome::Poisoned, "a parked call returns");
    assert_eq!(state.status(), Err(Poisoned { lock: "control" }));
    let later = Conn::open(&state, 600);
    later.call();
    assert_eq!(later.outcome(), Outcome::Poisoned, "and so does a new one");

    // The build thread still ends: it publishes, finds the control lock poisoned and stops.
    gate.open();
    wait_until("the build thread to publish", || {
        state.read(|_| ()) == Ok(Some(()))
    });
}
