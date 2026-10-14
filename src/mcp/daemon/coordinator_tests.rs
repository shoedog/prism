//! Tests for the readiness coordinator's states and budgets (spec §4.2 "Readiness semantics",
//! §9 unit list: per-connection, per-attempt budgets, terminal-outcome delivery, single retry).
//! Time is a clock the test moves; see `test_support` for why no verdict depends on the host.
use super::super::test_support::{
    advance, build_after, fail_after, fail_now, fixture, parts_with, scripted, state_with, status,
    wait_until, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::RefreshPolicy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

fn built(calls: &AtomicU64) -> u64 {
    calls.load(Ordering::SeqCst)
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn status_of(state: BuildState, attempt: u64, waiting: u64) -> BuildStatus {
    BuildStatus {
        state,
        attempt,
        generation: 0,
        waiting,
    }
}

/// Rules out building at construction: a client that only handshakes must cost no index.
#[test]
fn nothing_is_built_until_the_first_readiness_requiring_call() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);

    assert_eq!(status(&state), status_of(BuildState::Idle, 0, 0));
    assert_eq!(built(&calls), 0);
    assert_eq!(state.read(|_| ()), Ok(None), "nothing is published");

    // A client that waits zero seconds: the call starts the build and is answered at once.
    let conn = Conn::open(&state, 0);
    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Warming {
            elapsed: Duration::ZERO
        }
    );
    assert_eq!(status(&state), status_of(BuildState::Building, 1, 0));
    wait_until("the build thread to call the builder", || {
        built(&calls) == 1
    });

    // A second call during the same attempt starts nothing.
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));
    assert_eq!(status(&state).attempt, 1);

    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 1);
    assert_eq!(state.read(|_| ()), Ok(Some(())), "the index is published");
}

#[test]
fn a_warm_daemon_starts_attempt_one_at_construction_and_eager_is_refused() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    parts.startup = StartupMode::Background;
    let state = DaemonState::from_parts(parts).expect("a warm daemon state");

    assert!(state.startup_mode() == StartupMode::Background);
    assert_eq!(status(&state), status_of(BuildState::Building, 1, 0));
    wait_until("the build thread to call the builder", || {
        built(&calls) == 1
    });

    // The first call joins the attempt that is already running.
    let conn = Conn::open(&state, 600);
    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    assert_eq!(status(&state).attempt, 1);
    gate.open();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 1);

    // `--eager` is the in-process server's mode; a daemon state refuses it before building.
    let (eager_builder, eager_calls) = scripted(&cfg, vec![]);
    let refused = DaemonState::new(eager_builder, StartupMode::Eager)
        .err()
        .expect("an eager daemon state is refused");
    assert!(refused.to_string().contains("Eager"), "{refused}");
    assert_eq!(built(&eager_calls), 0);
}

/// Spec §4.2, the worked example: A and B join attempt 1 with budgets of 20 s and 60 s; A gets
/// warming at 20 s; B keeps waiting; publication at 35 s serves B directly and A's next call
/// immediately. Rules out one deadline shared by everyone waiting on an attempt.
#[test]
fn the_spec_s_worked_example_two_budgets_on_one_attempt() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let a = Conn::open(&state, 20);
    let b = Conn::open(&state, 60);

    a.call();
    b.call();
    wait_until("both calls to be waiting", || status(&state).waiting == 2);
    wait_until("the build thread to call the builder", || {
        built(&calls) == 1
    });
    assert_eq!(
        status(&state),
        status_of(BuildState::Building, 1, 2),
        "B joined the attempt A started"
    );

    advance(&clock, &state, 20);
    assert_eq!(a.outcome(), Outcome::Warming { elapsed: secs(20) });
    wait_until("B to be the only call waiting", || {
        status(&state).waiting == 1
    });
    assert!(!b.answered(), "A's deadline is not B's");
    assert_eq!(
        status(&state).state,
        BuildState::Building,
        "a warming answer is not a state change"
    );
    assert_eq!(built(&calls), 1, "and it cancels nothing");

    advance(&clock, &state, 15);
    gate.open();
    assert_eq!(
        b.outcome(),
        Outcome::Ready,
        "B is served by the publication at 35 s"
    );
    a.call();
    assert_eq!(
        a.outcome(),
        Outcome::Ready,
        "A's next call is served at once"
    );
    assert_eq!(status(&state), status_of(BuildState::Ready, 1, 0));
    assert_eq!(built(&calls), 1);
}

/// Rules out a fresh deadline for every call: a client that sends four calls during one build
/// would otherwise wait four budgets.
#[test]
fn later_calls_in_the_same_attempt_wait_only_what_is_left_of_the_budget() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 20);

    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    wait_until("the build thread to call the builder", || {
        built(&calls) == 1
    });
    advance(&clock, &state, 12);
    advance(&clock, &state, 8);
    assert_eq!(conn.outcome(), Outcome::Warming { elapsed: secs(20) });

    // The budget for this attempt is spent. Every later call returns without waiting.
    for _ in 0..3 {
        conn.call();
        assert_eq!(conn.outcome(), Outcome::Warming { elapsed: secs(20) });
    }
    assert_eq!(status(&state), status_of(BuildState::Building, 1, 0));

    advance(&clock, &state, 100);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Warming { elapsed: secs(120) });
    assert_eq!(built(&calls), 1);

    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
}

/// Rules out waking one waiter, and rules out a call that answers a failure by starting the
/// retry itself (a build that keeps failing would then keep one call forever).
#[test]
fn a_failed_attempt_is_delivered_to_every_call_waiting_on_it_and_nothing_retries_by_itself() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, calls) = scripted(&cfg, vec![fail_after(&gate, "the repository is gone")]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conns = [
        Conn::open(&state, 600),
        Conn::open(&state, 60),
        Conn::open(&state, 20),
    ];

    for conn in &conns {
        conn.call();
    }
    wait_until("all three calls to be waiting", || {
        status(&state).waiting == 3
    });
    gate.open();

    for conn in &conns {
        assert_eq!(
            conn.outcome(),
            Outcome::Failed("the repository is gone".to_string())
        );
    }
    assert_eq!(status(&state), status_of(BuildState::Failed, 1, 0));
    assert_eq!(
        built(&calls),
        1,
        "a failure is answered; the retry waits for the next call"
    );
}

/// Rules out a retry per caller. Eight calls arrive after a failure; the builder runs once
/// more, not eight times.
#[test]
fn the_next_call_after_a_failure_starts_exactly_one_retry_however_many_arrive_together() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let retry_gate = Gate::closed();
    let (builder, calls) = scripted(
        &cfg,
        vec![
            fail_now("first attempt failed"),
            build_after(&retry_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);

    let first = Conn::open(&state, 600);
    first.call();
    assert_eq!(
        first.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
    assert_eq!(status(&state), status_of(BuildState::Failed, 1, 0));

    let conns: Vec<Conn> = (0..8).map(|_| Conn::open(&state, 600)).collect();
    for conn in &conns {
        conn.call();
    }
    wait_until("all eight calls to be waiting on the retry", || {
        status(&state).waiting == 8
    });
    wait_until("the retry's build thread to call the builder", || {
        built(&calls) == 2
    });
    assert_eq!(status(&state), status_of(BuildState::Building, 2, 8));

    retry_gate.open();
    for conn in &conns {
        assert_eq!(conn.outcome(), Outcome::Ready);
    }
    assert_eq!(status(&state), status_of(BuildState::Ready, 2, 0));
    assert_eq!(built(&calls), 2);
}

/// Rules out a deadline kept for the connection's life: its budget for attempt 1 is spent, and
/// the retry must still be waited for in full.
#[test]
fn a_retry_gives_a_connection_a_fresh_full_budget() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let retry_gate = Gate::closed();
    let (builder, calls) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            build_after(&retry_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 20);

    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    advance(&clock, &state, 20);
    assert_eq!(conn.outcome(), Outcome::Warming { elapsed: secs(20) });

    first_gate.open();
    wait_until("the first attempt to fail", || {
        status(&state).state == BuildState::Failed
    });

    // This call starts the retry. With a fresh budget it waits; with the old deadline it would
    // be answered at once.
    conn.call();
    wait_until("the call to wait on the retry, or to return", || {
        status(&state).waiting == 1 || conn.answered()
    });
    assert!(
        !conn.answered(),
        "the retry's budget is fresh, so the call waits"
    );
    assert_eq!(status(&state), status_of(BuildState::Building, 2, 1));

    advance(&clock, &state, 20);
    assert_eq!(
        conn.outcome(),
        Outcome::Warming { elapsed: secs(20) },
        "elapsed counts from the retry's start"
    );

    retry_gate.open();
    wait_until("the retry to be announced", || {
        status(&state).state == BuildState::Ready
    });
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 2, "one build per attempt");
}

/// The production clock path: nobody moves a clock and nobody wakes the call. Rules out a wait
/// with no timeout. Asserts a lower bound only, so a slow host cannot fail it.
#[test]
fn on_the_real_clock_a_call_returns_warming_when_its_budget_ends_unprompted() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let conn = Conn::open(&state, 1);

    conn.call();
    match conn.outcome() {
        Outcome::Warming { elapsed } => assert!(
            elapsed >= secs(1),
            "the call returned before its budget ended: {elapsed:?}"
        ),
        other => panic!("expected warming, got {other:?}"),
    }
    assert_eq!(status(&state), status_of(BuildState::Building, 1, 0));
    gate.open();
}
