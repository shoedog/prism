//! Edges of the coordinator's contract that the second review round showed were stated and
//! not held by a test: each of these passes on the code as it is and fails under a mutant the
//! other tests let through (named in each test's comment).
use super::super::clock::Clock;
use super::super::test_support::{
    build_after, fail_after, fail_now, fixture, parts_with, scripted, status, wait_until,
    BuildStep, Conn, Gate, ManualClock, Outcome,
};
use super::*;
use crate::mcp::RefreshPolicy;
use crate::mcp::ServerConfig;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// A clock that moves without waking anybody (`ManualClock::advance` always wakes).
struct StepClock {
    now: Mutex<Instant>,
}

impl StepClock {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(Instant::now()),
        })
    }
    fn step(&self, secs: u64) {
        *self.now.lock().expect("step clock") += Duration::from_secs(secs);
    }
}

impl Clock for StepClock {
    fn now(&self) -> Instant {
        *self.now.lock().expect("step clock")
    }
}

fn refusing_spawner() -> Spawner {
    Box::new(move |_job: BuildJob| {
        Err(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "no more threads",
        ))
    })
}

/// The waiter of attempt 1 first runs when attempt 3 is building. The notifications of
/// attempts 1 and 2 are both held, so nothing wakes either earlier waiter until then; each is
/// then told its own attempt's failure, and the caller of attempt 3 keeps waiting. Rules out
/// an error kept with only its attempt number: nothing of attempt 1 would be left.
#[test]
fn a_waiter_of_a_failed_attempt_hears_that_failure_although_a_third_attempt_is_building() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let third_gate = Gate::closed();
    let (builder, _calls) = scripted(
        &cfg,
        vec![
            fail_after(&first_gate, "first attempt failed"),
            fail_now("second attempt failed"),
            build_after(&third_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = [Gate::closed(), Gate::closed()];
    let notify = [Gate::closed(), Gate::closed()];
    let (hook_recorded, hook_notify) = (recorded.clone(), notify.clone());
    let hook: AttemptHook = Arc::new(move |attempt: u64| {
        if attempt == 1 || attempt == 2 {
            let at = (attempt - 1) as usize;
            hook_recorded[at].open();
            let _opened = hook_notify[at].pass();
        }
    });
    parts.hooks.settled = Some(hook);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");

    let early = Conn::open(&state, 600);
    early.call();
    wait_until("early to wait on attempt 1", || status(&state).waiting == 1);
    first_gate.open();
    assert!(recorded[0].pass(), "attempt 1's failure is recorded");

    let second = Conn::open(&state, 600);
    second.call();
    assert!(recorded[1].pass(), "attempt 2's failure is recorded");
    let third = Conn::open(&state, 600);
    third.call();
    wait_until("attempt 3 to be building with three calls parked", || {
        let now = status(&state);
        now.state == BuildState::Building && now.attempt == 3 && now.waiting == 3
    });
    assert!(
        !early.answered(),
        "nothing has woken the waiter of attempt 1"
    );
    assert!(!second.answered(), "nor the waiter of attempt 2");

    notify[0].open();
    assert_eq!(
        early.outcome(),
        Outcome::Failed("first attempt failed".to_string())
    );
    assert_eq!(
        second.outcome(),
        Outcome::Failed("second attempt failed".to_string())
    );
    assert_eq!(status(&state).attempt, 3);
    assert_eq!(status(&state).state, BuildState::Building);
    assert!(!third.answered(), "the caller of attempt 3 keeps waiting");

    notify[1].open();
    third_gate.open();
    assert_eq!(third.outcome(), Outcome::Ready);
}

/// A builder may hand over a provider whose generation is not 0. The control plane reports
/// it from the moment of publication. Rules out not recording at publication, which every
/// other test lets through because their builders return a provider at generation 0.
#[test]
fn the_generation_of_a_published_provider_is_recorded_at_publication() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let step_cfg = cfg.clone();
    let step: BuildStep = Box::new(move || -> anyhow::Result<SessionProvider> {
        let mut provider = SessionProvider::bootstrap(&step_cfg)?;
        provider.refresh()?;
        Ok(provider)
    });
    let (builder, _calls) = scripted(&cfg, vec![step]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(parts_with(builder, &clock)).expect("a lazy daemon state");
    let conn = Conn::open(&state, 600);

    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
    assert_eq!(state.read(|provider| provider.generation()), Ok(Some(1)));
    assert_eq!(status(&state).generation, 1);
}

fn one_call_whose_budget_ends_as_its_attempt_ends(
    step_for: impl Fn(&Arc<Gate>, &ServerConfig) -> BuildStep,
) -> Outcome {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _calls) = scripted(&cfg, vec![step_for(&gate, &cfg)]);
    let clock = StepClock::new();
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let shared: Arc<dyn Clock> = clock.clone();
    parts.clock = shared;
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 20);

    conn.call();
    wait_until("the call to be parked", || status(&state).waiting == 1);
    // The deadline passes while the call is parked; nobody wakes it. The only wake-up it
    // gets is the attempt's ending.
    clock.step(20);
    gate.open();
    conn.outcome()
}

/// A waiter whose budget has run out by the time its attempt's ending wakes it is told the
/// outcome, not warming. Rules out checking the deadline before the outcome.
#[test]
fn a_budget_that_ends_as_the_attempt_publishes_is_answered_ready() {
    let outcome = one_call_whose_budget_ends_as_its_attempt_ends(build_after);
    assert_eq!(outcome, Outcome::Ready);
}

/// The same with a failure: the waiter hears it although its deadline has passed. Under the
/// same mutant the failure would reach nobody.
#[test]
fn a_budget_that_ends_as_the_attempt_fails_is_answered_failed() {
    let outcome =
        one_call_whose_budget_ends_as_its_attempt_ends(|gate, _cfg| fail_after(gate, "it failed"));
    assert_eq!(outcome, Outcome::Failed("it failed".to_string()));
}

/// A client that waits zero seconds still hears an attempt that ended before the call could
/// wait: a build thread that cannot be started, and an exhausted counter. Under the same
/// mutant it would be told warming for ever.
#[test]
fn a_zero_wait_client_hears_a_failure_that_is_already_recorded() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, calls) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    parts.spawner = refusing_spawner();
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let conn = Conn::open(&state, 0);

    conn.call();
    assert_eq!(
        conn.outcome(),
        Outcome::Failed("could not start the index build thread: no more threads".to_string())
    );
    state.lock_control().expect("control").attempts = u64::MAX;
    for _ in 0..2 {
        conn.call();
        assert_eq!(
            conn.outcome(),
            Outcome::Failed(ATTEMPTS_EXHAUSTED.to_string())
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
