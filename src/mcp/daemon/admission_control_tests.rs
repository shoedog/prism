//! Tests for the control exchange in the control plane (spec §4.7; S4's interface for S5b): an
//! operator's request leaves the daemon's idle time as it found it, a status counts everyone
//! but the request itself, and a shutdown request drains the daemon once nothing is admitted or
//! pending, whatever the idle TTL and a running build. Time is a clock the test moves.
use super::super::test_support::{
    advance, build_after, fixture, scripted, state_with, status, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::RefreshPolicy;

const TTL: Duration = Duration::from_secs(900);

fn lazy_state() -> (tempfile::TempDir, Arc<ManualClock>, Arc<DaemonState>) {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    (dir, clock, state)
}

fn retire(state: &DaemonState) -> Option<StopReason> {
    state
        .retire_if_idle(TTL, || {})
        .expect("the control lock is not poisoned")
}

/// A probe: counted as pending when it is accepted, then answered as a control request.
fn probe(state: &DaemonState, shutdown: bool) -> Control {
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    state
        .control_exchange(shutdown)
        .expect("the control lock is not poisoned")
}

fn snapshot(control: Control) -> ControlSnapshot {
    match control {
        Control::Status(snapshot) => snapshot,
        Control::Retiring => panic!("the daemon is not draining"),
    }
}

/// S4's interface for S5b: a status probe is counted as pending while its line is read, and must
/// not restart the idle time when it leaves, or an operator who looks at a daemon keeps it up.
/// Rules out a probe that leaves the way any other connection does (`leave_handshake`).
#[test]
fn a_control_exchange_leaves_the_idle_time_as_it_found_it() {
    let (_dir, clock, state) = lazy_state();
    assert_eq!(
        retire(&state),
        None,
        "the first decision starts the idle count"
    );
    advance(&clock, &state, TTL.as_secs() - 1);
    let answered = snapshot(probe(&state, false));
    assert_eq!(answered.idle_for, Some(TTL - Duration::from_secs(1)));
    assert_eq!(retire(&state), None, "one second short of the TTL");
    advance(&clock, &state, 1);
    assert_eq!(retire(&state), Some(StopReason::Idle));
}

/// A client that comes and goes while a probe is in the handshake stage moves the idle time to
/// when it left, as it would have without the probe. Rules out a probe that puts back the idle
/// time it found when it entered.
#[test]
fn a_client_that_leaves_during_a_control_exchange_moves_the_idle_time() {
    let (_dir, clock, state) = lazy_state();
    assert_eq!(retire(&state), None);
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending), "the probe");
    advance(&clock, &state, 5);
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending), "a client");
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    advance(&clock, &state, 5);
    state.release().expect("release");
    let answered = snapshot(state.control_exchange(false).expect("control"));
    assert_eq!(
        answered.idle_for,
        Some(Duration::ZERO),
        "idle since the client left"
    );
    advance(&clock, &state, TTL.as_secs() - 1);
    assert_eq!(
        retire(&state),
        None,
        "the TTL counts from the client's release"
    );
    advance(&clock, &state, 1);
    assert_eq!(retire(&state), Some(StopReason::Idle));
}

/// The status counts every connection but the request itself, and has no idle time while a
/// client is connected. Rules out a status read before the request has left the pending count.
#[test]
fn a_status_counts_every_connection_but_itself() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    let answered = snapshot(probe(&state, false));
    assert_eq!(
        answered,
        ControlSnapshot {
            build: BuildStatus {
                state: BuildState::Idle,
                attempt: 0,
                generation: 0,
                waiting: 0,
            },
            admitted: 1,
            pending: 1,
            idle_for: None,
            shutdown: false,
        }
    );
    assert_eq!(
        state.admission_status().expect("counts"),
        AdmissionStatus {
            pending: 1,
            admitted: 1,
            draining: None,
        },
        "the probe is no longer counted"
    );
}

/// Spec §4.7: `daemon-shutdown` drains a daemon with no client at once: the idle TTL does not
/// apply, and neither does a running build (S4's SI-15: a build that never returns would keep the
/// daemon up for ever). Rules out a shutdown that only shortens the TTL, or that waits for the
/// build.
#[test]
fn a_shutdown_drains_a_daemon_with_no_client_at_once_whatever_the_ttl_or_a_build() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 0);
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));
    assert_eq!(status(&state).state, BuildState::Building);
    assert_eq!(retire(&state), None);

    let answered = snapshot(probe(&state, true));
    assert!(answered.shutdown);
    let mut unlinked = 0;
    assert_eq!(
        state.retire_if_idle(TTL, || unlinked += 1),
        Ok(Some(StopReason::Shutdown))
    );
    assert_eq!(
        unlinked, 1,
        "the socket is unlinked in the same critical section"
    );
    assert_eq!(
        status(&state).state,
        BuildState::Building,
        "the build still runs"
    );
    gate.open();
}

/// Spec §4.7 and §11 item 3: a shutdown never disconnects a client; the daemon drains when the
/// last one has gone. Until then it goes on admitting (D100), so a client that arrives is served
/// rather than failed closed. Rules out a drain while a client is admitted, and a daemon that
/// refuses new clients after the request.
#[test]
fn a_shutdown_waits_for_the_last_client_and_still_admits_new_ones() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert!(matches!(state.admit(), Ok(Admit::Ready(_))));
    let answered = snapshot(probe(&state, true));
    assert_eq!((answered.admitted, answered.shutdown), (1, true));
    assert_eq!(retire(&state), None, "a client is admitted");

    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert_eq!(retire(&state), None, "a client is pending");
    assert!(
        matches!(state.admit(), Ok(Admit::Ready(_))),
        "a new client is still admitted"
    );
    state.release().expect("release");
    assert_eq!(retire(&state), None, "one client left");
    state.release().expect("release");
    assert_eq!(retire(&state), Some(StopReason::Shutdown));
}

/// A request that arrives once the daemon drains is answered `RETIRING`, leaves the pending
/// count, and records nothing.
#[test]
fn a_control_exchange_after_the_drain_decision_is_answered_retiring() {
    let (_dir, _clock, state) = lazy_state();
    assert_eq!(state.enter_handshake(64), Ok(Entry::Pending));
    assert_eq!(
        state.drain(StopReason::Signal, || {}),
        Ok(StopReason::Signal)
    );
    assert_eq!(state.control_exchange(true), Ok(Control::Retiring));
    assert_eq!(
        state.admission_status().expect("counts"),
        AdmissionStatus {
            pending: 0,
            admitted: 0,
            draining: Some(StopReason::Signal),
        }
    );
    assert!(
        !state
            .control_plane()
            .expect("the control lock")
            .admission
            .shutdown,
        "no shutdown was recorded (review round 1, Opus S2)"
    );
    assert_eq!(
        retire(&state),
        Some(StopReason::Signal),
        "the first reason stands"
    );
}
