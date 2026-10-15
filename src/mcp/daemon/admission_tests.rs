//! Tests for admission and retirement under the control lock (spec §4.2 "Admission and
//! retirement", §9 tests 2 and 16 as far as the control plane decides them). Time is a clock
//! the test moves; nothing here sleeps to order two events.
use super::super::clock::Clock;
use super::super::coordinator::{DaemonState, Parts};
use super::super::fatal::{Domain, Fatal};
use super::super::test_support::{
    build_after, client_settings, fail_now, fixture, parts_with, scripted, state_with, status,
    wait_until, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{RefreshPolicy, StartupMode};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

const TTL: Duration = Duration::from_secs(900);

fn lazy_state() -> (tempfile::TempDir, Arc<ManualClock>, Arc<DaemonState>) {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    (dir, clock, state)
}

fn counts(state: &DaemonState) -> AdmissionStatus {
    state
        .admission_status()
        .expect("the control lock is not poisoned")
}

fn retire(state: &DaemonState) -> Option<StopReason> {
    state
        .retire_if_idle(TTL, || {})
        .expect("the control lock is not poisoned")
}

/// Move the clock without waking anyone: nothing here is parked.
fn later(clock: &ManualClock, state: &DaemonState, secs: u64) {
    super::super::test_support::advance(clock, state, secs);
}

/// Spec §4.2: `--max-clients` bounds pending and admitted connections together, and a
/// connection over the bound is answered `BUSY` without being counted. Rules out a bound on
/// admitted connections alone, under which stalled handshakes are unbounded.
#[test]
fn the_bound_counts_pending_and_admitted_connections_together() {
    let (_dir, _clock, state) = lazy_state();

    assert_eq!(state.enter_handshake(2), Ok(Entry::Pending));
    assert_eq!(state.enter_handshake(2), Ok(Entry::Pending));
    assert_eq!(
        state.enter_handshake(2),
        Ok(Entry::Busy { max_clients: 2 }),
        "two pending connections fill a bound of two"
    );
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    assert_eq!(
        state.enter_handshake(2),
        Ok(Entry::Busy { max_clients: 2 }),
        "one pending and one admitted fill it too"
    );
    assert_eq!(
        counts(&state),
        AdmissionStatus {
            pending: 1,
            admitted: 1,
            draining: None
        }
    );

    state.release().expect("release");
    assert_eq!(state.enter_handshake(2), Ok(Entry::Pending));
    state.leave_handshake().expect("leave");
    state.leave_handshake().expect("leave");
    assert_eq!(
        counts(&state),
        AdmissionStatus {
            pending: 0,
            admitted: 0,
            draining: None
        }
    );
}

/// `READY` carries the build state and generation read in the same critical section that
/// admits the connection.
#[test]
fn an_admission_reports_the_build_status_it_was_made_under() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    let Ok(Admit::Ready(admitted)) = state.admit() else {
        panic!("a connection is admitted");
    };
    assert_eq!(admitted, status(&state));
    assert_eq!(admitted.state, BuildState::Idle);
}

/// Spec §4.2: retirement needs nothing admitted and nothing pending for the idle TTL. The idle
/// time counts from the moment both counts reached zero, not from the daemon's start. Rules out
/// a TTL measured from the last admission (a long session would retire the daemon at once when
/// it ends) and retirement with a connection still pending.
#[test]
fn retirement_waits_for_the_idle_ttl_counted_from_the_last_connection() {
    let (_dir, clock, state) = lazy_state();

    assert_eq!(
        retire(&state),
        None,
        "the first decision starts the idle count"
    );
    later(&clock, &state, TTL.as_secs() - 1);
    assert_eq!(retire(&state), None, "one second short of the TTL");
    later(&clock, &state, 1);
    assert_eq!(
        counts(&state).draining,
        None,
        "nothing decides until the decision is asked for"
    );

    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    later(&clock, &state, 2 * TTL.as_secs());
    assert_eq!(retire(&state), None, "a pending connection holds it off");
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    later(&clock, &state, 2 * TTL.as_secs());
    assert_eq!(retire(&state), None, "an admitted connection holds it off");

    state.release().expect("release");
    later(&clock, &state, TTL.as_secs() - 1);
    assert_eq!(retire(&state), None, "the TTL counts from the release");
    later(&clock, &state, 1);
    assert_eq!(retire(&state), Some(StopReason::Idle));
}

/// Spec §4.2: no retirement while a build runs, however long the daemon has been idle.
#[test]
fn a_running_build_holds_retirement_off() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 0);
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));

    assert_eq!(retire(&state), None);
    later(&clock, &state, 2 * TTL.as_secs());
    assert_eq!(retire(&state), None);
    gate.open();
    wait_until("the build to publish", || {
        status(&state).state == BuildState::Ready
    });
    assert_eq!(retire(&state), Some(StopReason::Idle));
}

/// Spec §4.2: an admission and a retirement cannot interleave. Once the decision is taken no
/// connection is counted or admitted, and a connection that was pending is told `RETIRING`.
#[test]
fn after_the_retirement_decision_nothing_is_admitted() {
    let (_dir, clock, state) = lazy_state();
    assert_eq!(retire(&state), None);
    later(&clock, &state, TTL.as_secs());
    assert_eq!(retire(&state), Some(StopReason::Idle));

    assert_eq!(state.enter_handshake(64), Ok(Entry::Retiring));
    assert_eq!(
        state.admit(),
        Ok(Admit::Retiring),
        "a connection that raced into the stage"
    );
    assert_eq!(counts(&state).admitted, 0);
}

/// Spec §3.1 and §4.2: the socket is unlinked inside the critical section that decides to
/// drain, and once. Rules out an unlink after the lock is released, which a connection could be
/// admitted before.
#[test]
fn the_socket_is_unlinked_inside_the_critical_section_and_once() {
    let (_dir, clock, state) = lazy_state();
    let unlinked = AtomicU64::new(0);
    let held = Mutex::new(Vec::new());
    let unlink = || {
        unlinked.fetch_add(1, Ordering::SeqCst);
        held.lock().expect("held").push(state.control_is_held());
    };
    assert_eq!(
        state.retire_if_idle(TTL, || panic!("unlinked too soon")),
        Ok(None)
    );
    later(&clock, &state, TTL.as_secs());
    assert_eq!(
        state.retire_if_idle(TTL, unlink),
        Ok(Some(StopReason::Idle))
    );
    assert_eq!(
        state.retire_if_idle(TTL, || panic!("unlinked twice")),
        Ok(Some(StopReason::Idle))
    );
    assert_eq!(
        state.drain(StopReason::Signal, || panic!("unlinked twice")),
        Ok(StopReason::Idle),
        "a later reason does not replace the first"
    );
    assert_eq!(unlinked.load(Ordering::SeqCst), 1);
    assert_eq!(*held.lock().expect("held"), vec![true]);
}

/// Spec §3.1: a lost root and a signal drain the daemon whatever its counts, and the counts go
/// on being kept for the connections that are still open.
#[test]
fn a_forced_drain_ignores_the_counts_and_refuses_new_connections() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));

    let mut unlinked = 0;
    assert_eq!(
        state.drain(StopReason::RootChanged, || unlinked += 1),
        Ok(StopReason::RootChanged)
    );
    assert_eq!(unlinked, 1);
    assert_eq!(state.enter_handshake(64), Ok(Entry::Retiring));
    assert_eq!(state.admit(), Ok(Admit::Retiring));
    state.release().expect("release");
    assert_eq!(
        counts(&state),
        AdmissionStatus {
            pending: 0,
            admitted: 0,
            draining: Some(StopReason::RootChanged)
        }
    );
}

/// The accept loop sleeps until the idle deadline at most: it exists only while nothing is
/// pending or admitted, and moves with the last release.
#[test]
fn the_idle_deadline_follows_the_last_connection() {
    let (_dir, clock, state) = lazy_state();
    let start = clock.now();
    assert_eq!(state.idle_deadline(TTL), Ok(None), "nothing counted yet");
    assert_eq!(retire(&state), None);
    assert_eq!(state.idle_deadline(TTL), Ok(Some(start + TTL)));
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert_eq!(state.idle_deadline(TTL), Ok(None));
    later(&clock, &state, 7);
    state.leave_handshake().expect("leave");
    assert_eq!(
        state.idle_deadline(TTL),
        Ok(Some(start + Duration::from_secs(7) + TTL))
    );
}

/// Every entry point here is in the control plane: a panic in it ends the daemon while the
/// control lock is still held (D47). Rules out an entry point that takes the lock without the
/// guard, whose panic would leave the lock poisoned for every other client.
#[test]
fn a_panic_inside_the_retirement_decision_ends_the_daemon_with_the_lock_held() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts: Parts = parts_with(builder, &clock);
    let seen: Arc<Mutex<Vec<(Fatal, bool)>>> = Arc::default();
    let target: Arc<OnceLock<Weak<DaemonState>>> = Arc::default();
    let sink = Arc::clone(&seen);
    let watched = Arc::clone(&target);
    parts.on_fatal = Arc::new(move |fatal| {
        let held = watched
            .get()
            .and_then(Weak::upgrade)
            .is_some_and(|state| state.control_is_held());
        sink.lock().expect("seen").push((fatal, held));
    });
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let _ = target.set(Arc::downgrade(&state));
    assert_eq!(state.retire_if_idle(TTL, || {}), Ok(None));
    later(&clock, &state, TTL.as_secs());

    let panicking = Arc::clone(&state);
    let unwound = std::thread::spawn(move || {
        let _ = panicking.retire_if_idle(TTL, || panic!("injected panic in the unlink"));
    })
    .join();
    assert!(unwound.is_err());
    assert_eq!(
        *seen.lock().expect("seen"),
        vec![(Fatal::Panicked(Domain::Control), true)]
    );
}

/// S3a's parity requirement for S4: a connection of a warm daemon whose startup attempt failed
/// before the connection's first call holds that attempt, so its first call hears the failure.
/// A lazy daemon's connection, and one admitted after the retry started, start empty.
#[test]
fn only_a_warm_daemon_s_startup_attempt_is_given_to_a_new_connection() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![fail_now("the startup build failed")]);
    let state = DaemonState::new(builder, StartupMode::Background).expect("a warm daemon");
    wait_until("the startup build to fail", || {
        status(&state).state == BuildState::Failed
    });
    let mut seeded = state.budget_for(&client_settings(0)).expect("a budget");
    assert_eq!(
        seeded.unheard_failure(),
        Some("the startup build failed".to_string())
    );
    assert_eq!(seeded.unheard_failure(), None, "heard once");

    // The retry starts; a connection that arrives now does not hear attempt 1.
    let conn = Conn::open(&state, 600);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    let mut later_one = state.budget_for(&client_settings(0)).expect("a budget");
    assert_eq!(later_one.unheard_failure(), None);

    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![fail_now("a lazy build failed")]);
    let lazy = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon");
    let conn = Conn::open(&lazy, 600);
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Failed(_)));
    let mut fresh = lazy.budget_for(&client_settings(0)).expect("a budget");
    assert_eq!(fresh.unheard_failure(), None);
}

/// A clock that records, each time it is read, whether the control lock was held.
struct WatchingClock {
    inner: Arc<ManualClock>,
    state: OnceLock<Weak<DaemonState>>,
    seen: Mutex<Vec<bool>>,
}

impl Clock for WatchingClock {
    fn now(&self) -> std::time::Instant {
        if let Some(state) = self.state.get().and_then(Weak::upgrade) {
            self.seen
                .lock()
                .expect("seen")
                .push(state.control_is_held());
        }
        self.inner.now()
    }
}

/// Every admission and retirement function reads the clock inside its critical section (review
/// round 1, sol F4). A time read before the lock can be older than a change another thread makes
/// in between: A reads t=0 for its release and is descheduled before it takes the lock; B, still
/// active, releases at t=5; A then records t=0 as the start of the idle time, and the daemon
/// retires 895 s after the last activity with a TTL of 900. Rules out a time read before the
/// lock in any of them.
#[test]
fn every_admission_and_retirement_decision_reads_the_clock_under_the_control_lock() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = Arc::new(WatchingClock {
        inner: ManualClock::new(),
        state: OnceLock::new(),
        seen: Mutex::new(Vec::new()),
    });
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let shared: Arc<dyn Clock> = Arc::clone(&clock) as Arc<dyn Clock>;
    parts.clock = shared;
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let _ = clock.state.set(Arc::downgrade(&state));

    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    state.leave_handshake().expect("leave");
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    state.release().expect("release");
    assert_eq!(state.retire_if_idle(TTL, || {}), Ok(None));

    let seen = clock.seen.lock().expect("seen").clone();
    assert_eq!(
        seen.len(),
        4,
        "one read each by leave, admit, release and retire"
    );
    assert!(seen.iter().all(|held| *held), "{seen:?}");
}
