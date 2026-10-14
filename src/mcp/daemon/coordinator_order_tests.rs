//! Tests for the two steps of the build thread at which another actor can interleave, each
//! played through a test hook, and for exclusive access to the published provider.
use super::super::test_support::{
    build_after, fail_after, fixture, parts_with, scripted, state_with, status, wait_until, Conn,
    Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::RefreshPolicy;
use std::sync::atomic::{AtomicU64, Ordering};

fn built(calls: &AtomicU64) -> u64 {
    calls.load(Ordering::SeqCst)
}

/// A hook that reports it was reached and then holds the build thread until the test lets it
/// go, for the attempt numbered `only`.
fn stop_at(reached: &Arc<Gate>, resume: &Arc<Gate>, only: u64) -> AttemptHook {
    let reached = Arc::clone(reached);
    let resume = Arc::clone(resume);
    Arc::new(move |attempt: u64| {
        if attempt == only {
            reached.open();
            let _opened = resume.pass();
        }
    })
}

/// "Publication before Ready" (spec §4.2). The test stands on the build thread between the two
/// steps and looks from outside. Rules out announcing first: a call told `Ready` would then
/// find the slot empty.
#[test]
fn the_index_is_in_the_slot_before_ready_is_announced() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let reached = Gate::closed();
    let resume = Gate::closed();
    parts.hooks.published = Some(stop_at(&reached, &resume, 1));
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

    conn.call();
    assert!(
        reached.pass(),
        "the build thread reaches the publication hook"
    );

    // The build thread is between publishing and announcing.
    assert_eq!(
        state.read(|_| ()),
        Ok(Some(())),
        "the provider is already in the slot"
    );
    assert_eq!(
        status(&state).state,
        BuildState::Building,
        "and Ready has not been announced"
    );
    assert!(
        !conn.answered(),
        "so no call has been told the index is ready"
    );

    resume.open();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).state, BuildState::Ready);
}

/// Attempt 1 fails. Before its waiter is notified, another connection calls: it is the next
/// call after the failure, so it starts the retry. The waiter then wakes to find attempt 2
/// building. Its own attempt's failure is stale by then, so it is told to retry (the owner's
/// decision, D26): it is not given that failure, it does not join the retry with the budget
/// of another attempt, and it starts no third attempt. Rules out a waiter that reports a
/// failure a retry has already superseded, and one that waits on whatever is building.
#[test]
fn a_waiter_whose_failed_attempt_was_superseded_by_a_building_retry_is_told_to_retry() {
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
    let mut parts = parts_with(builder, &clock);
    let recorded = Gate::closed();
    let notify = Gate::closed();
    parts.hooks.settled = Some(stop_at(&recorded, &notify, 1));
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");

    let early = Conn::open(&state, 600);
    early.call();
    wait_until("the early call to be waiting on attempt 1", || {
        status(&state).waiting == 1
    });

    first_gate.open();
    assert!(recorded.pass(), "attempt 1's failure is recorded");
    // Attempt 1's failure is recorded and its waiter has not been notified. Play the other
    // client now.
    let late = Conn::open(&state, 600);
    late.call();
    wait_until("the retry to be building", || {
        let now = status(&state);
        now.state == BuildState::Building && now.attempt == 2
    });

    notify.open();
    assert_eq!(
        early.outcome(),
        Outcome::Superseded,
        "attempt 1 failed and attempt 2 is building: the early call is told to retry"
    );
    wait_until("the retry's build thread to call the builder", || {
        built(&calls) == 2
    });
    assert_eq!(status(&state).attempt, 2, "and it started no third attempt");

    retry_gate.open();
    assert_eq!(late.outcome(), Outcome::Ready);
    early.call();
    assert_eq!(early.outcome(), Outcome::Ready);
    assert_eq!(built(&calls), 2);
}

/// Rules out a control plane that keeps reporting the generation the index was published
/// with: `READY` and the status probe would then name a snapshot nobody is served any more.
#[test]
fn exclusive_access_records_the_provider_s_generation_in_the_control_plane() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);

    assert_eq!(
        state.write(|_| ()),
        Ok(None),
        "nothing is published, so there is nothing to change"
    );
    assert_eq!(status(&state).state, BuildState::Idle);

    let conn = Conn::open(&state, 600);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).generation, 0);

    let refreshed = state
        .write(|provider| provider.refresh().map(|summary| summary.generation))
        .expect("the provider lock is not poisoned")
        .expect("an index is published");
    assert_eq!(refreshed.expect("the refresh succeeds"), 1);
    assert_eq!(
        status(&state).generation,
        1,
        "the control plane reports the refreshed generation"
    );
    assert_eq!(state.read(|provider| provider.generation()), Ok(Some(1)));

    // Access that changes nothing leaves the generation where it is.
    assert_eq!(state.write(|_| ()), Ok(Some(())));
    assert_eq!(status(&state).generation, 1);
}

fn poison_the_provider_slot(state: &Arc<DaemonState>) {
    let state = Arc::clone(state);
    let panicked = std::thread::spawn(move || {
        let _slot = state.provider.write();
        panic!("poisoning the provider slot for a test");
    })
    .join();
    assert!(panicked.is_err());
}

/// Rules out announcing `Ready` when the provider could not be put in the slot, and `unwrap`
/// on the slot's lock.
#[test]
fn a_poisoned_provider_slot_fails_the_attempt_and_is_an_error_for_every_reader() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let conn = Conn::open(&state, 600);

    conn.call();
    wait_until("the call to be waiting", || status(&state).waiting == 1);
    poison_the_provider_slot(&state);
    gate.open();

    assert_eq!(conn.outcome(), Outcome::Failed(SLOT_POISONED.to_string()));
    assert_eq!(
        status(&state).state,
        BuildState::Failed,
        "Ready was not announced over an empty slot"
    );
    assert_eq!(state.read(|_| ()), Err(Poisoned { lock: "provider" }));
    assert_eq!(state.write(|_| ()), Err(Poisoned { lock: "provider" }));
}

/// What the three tests below share: attempt 1 is parked on by `early`, fails with "first
/// attempt failed", and its build thread is held between recording that and notifying. The
/// caller then lets other attempts come and go before `early` is allowed to hear anything from
/// attempt 1's own notification. Returns the state, the early connection, the gate that
/// releases attempt 1's notification, and the builder's call counter.
fn a_waiter_held_behind_its_failed_attempt(
    cfg: &crate::mcp::ServerConfig,
    later_attempts: Vec<super::super::test_support::BuildStep>,
) -> (Arc<DaemonState>, Conn, Arc<Gate>, Arc<AtomicU64>) {
    let first_gate = Gate::closed();
    let mut steps = vec![fail_after(&first_gate, "first attempt failed")];
    steps.extend(later_attempts);
    let (builder, calls) = scripted(cfg, steps);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = Gate::closed();
    let notify = Gate::closed();
    parts.hooks.settled = Some(stop_at(&recorded, &notify, 1));
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");

    let early = Conn::open(&state, 600);
    early.call();
    wait_until("the early call to be waiting on attempt 1", || {
        status(&state).waiting == 1
    });
    first_gate.open();
    assert!(recorded.pass(), "attempt 1's failure is recorded");
    (state, early, notify, calls)
}

/// The retry has already succeeded when the waiter of attempt 1 runs. Its own attempt failed,
/// and a later one has published the index since, so it is answered `Ready` and served at once
/// (the owner's decision of 2026-10-13, amending D26), and so is its next call. Rules out
/// reporting the superseded failure, and telling the call to retry with nothing building.
#[test]
fn a_waiter_whose_failed_attempt_was_superseded_by_a_ready_index_is_served() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (state, early, notify, calls) = a_waiter_held_behind_its_failed_attempt(&cfg, vec![]);

    // The next call after the failure starts attempt 2, which builds and is announced. That
    // announcement wakes everybody parked, the waiter of attempt 1 included.
    let late = Conn::open(&state, 600);
    late.call();
    assert_eq!(late.outcome(), Outcome::Ready);
    assert_eq!(status(&state).state, BuildState::Ready);

    assert_eq!(
        early.outcome(),
        Outcome::Ready,
        "attempt 1 failed and the index is ready: the waiter of attempt 1 is served"
    );
    notify.open();
    assert_eq!(status(&state).attempt, 2);
    assert_eq!(built(&calls), 2);

    // Its next call finds the index ready.
    early.call();
    assert_eq!(early.outcome(), Outcome::Ready);
}

/// Here the retry failed too, for another reason, and nothing is building: no retry has
/// superseded attempt 1's failure, so its waiter is told of it, with attempt 1's own text.
/// Rules out one remembered error for all attempts (the waiter of attempt 1 would be told
/// attempt 2's) and telling every waiter of a failed attempt to retry.
#[test]
fn a_waiter_of_a_failed_attempt_hears_that_failure_although_the_retry_failed_differently() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (state, early, notify, _calls) = a_waiter_held_behind_its_failed_attempt(
        &cfg,
        vec![super::super::test_support::fail_now(
            "second attempt failed",
        )],
    );

    let late = Conn::open(&state, 600);
    late.call();
    assert_eq!(
        late.outcome(),
        Outcome::Failed("second attempt failed".to_string()),
        "the call that started attempt 2 hears attempt 2's"
    );

    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string()),
        "the waiter of attempt 1 hears attempt 1's"
    );
    notify.open();
    assert_eq!(status(&state).attempt, 2);
}

// The third-attempt case is in `coordinator_edge_tests.rs`, where both earlier notifications
// are held so that the waiter of attempt 1 cannot run before attempt 3 is building.

/// Exclusive access can land after the provider is in the slot and before `Ready` is
/// announced. The generation the control plane then reports must still be the slot's. Rules
/// out announcing the generation that was read before publication, and recording a generation
/// only once the phase is `Ready`.
#[test]
fn exclusive_access_between_publication_and_ready_keeps_the_generation_current() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let reached = Gate::closed();
    let resume = Gate::closed();
    parts.hooks.published = Some(stop_at(&reached, &resume, 1));
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

    conn.call();
    assert!(
        reached.pass(),
        "the build thread reaches the publication hook"
    );
    assert_eq!(status(&state).state, BuildState::Building);

    let refreshed = state
        .write(|provider| provider.refresh().map(|summary| summary.generation))
        .expect("the provider lock is not poisoned")
        .expect("an index is in the slot");
    assert_eq!(refreshed.expect("the refresh succeeds"), 1);
    assert_eq!(
        status(&state).generation,
        1,
        "the generation is the slot's before Ready is announced too"
    );

    resume.open();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(status(&state).state, BuildState::Ready);
    assert_eq!(
        status(&state).generation,
        1,
        "the announcement did not bring back the generation read before publication"
    );
    assert_eq!(state.read(|provider| provider.generation()), Ok(Some(1)));
}
