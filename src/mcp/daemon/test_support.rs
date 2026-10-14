//! Pieces the daemon's tests share: a clock the test moves, gates that hold a build until the
//! test lets it go, a builder that plays a script, and a stand-in for one connection.
//!
//! No test here sleeps to order two events. A test waits for a condition that must become true
//! (`wait_until`) up to [`HANG`], and moves time itself (`advance`).
use super::budget::ConnectionBudget;
use super::clock::Clock;
use super::coordinator::{BuildStatus, DaemonState, Parts, Poisoned};
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::lazy::{Readiness, SessionBuilder};
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, SessionProvider, StartupMode};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// How long a test waits for something that must happen before it calls the run hung. It is
/// generous on purpose: a verdict depends on whether the thing happens, never on how soon.
pub(super) const HANG: Duration = Duration::from_secs(30);

/// The settings of a client that waits `wait_secs` for the index and always gets
/// `structuredContent`.
pub(super) fn client_settings(wait_secs: u64) -> ClientSettings {
    client_settings_with(wait_secs, StructuredContentMode::Always)
}

pub(super) fn client_settings_with(
    wait_secs: u64,
    structured_content: StructuredContentMode,
) -> ClientSettings {
    let output = ConnectionSettings {
        result_cap: 80_000,
        structured_content,
        concise_shape: ConciseShapeMode::Slim,
    };
    ClientSettings::new(output, wait_secs, false).expect("settings within the accepted bounds")
}

/// Poll `condition` until it holds; panic, naming `what`, after [`HANG`].
pub(super) fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let give_up = Instant::now() + HANG;
    while !condition() {
        assert!(
            Instant::now() < give_up,
            "gave up after {HANG:?} waiting for: {what}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// A clock that moves only when a test moves it.
pub(super) struct ManualClock {
    now: Mutex<Instant>,
}

impl ManualClock {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(Instant::now()),
        })
    }

    fn advance(&self, by: Duration) {
        let mut now = self.now.lock().expect("manual clock");
        *now += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self.now.lock().expect("manual clock")
    }
}

/// Move `clock` forward and make every parked call look at it. `DaemonState::wake` takes the
/// control lock, so a call that has read the old time and not yet parked is not missed.
pub(super) fn advance(clock: &ManualClock, state: &DaemonState, secs: u64) {
    clock.advance(Duration::from_secs(secs));
    state.wake();
}

/// Something a thread waits at until a test opens it.
pub(super) struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    pub(super) fn closed() -> Arc<Self> {
        Arc::new(Self {
            open: Mutex::new(false),
            changed: Condvar::new(),
        })
    }

    pub(super) fn open(&self) {
        *self.open.lock().expect("gate") = true;
        self.changed.notify_all();
    }

    /// Wait until the gate is open and return `true`. Gives up after four times [`HANG`] and
    /// returns `false`, so a test that fails before it opens its gate does not leave a thread
    /// parked for the rest of the run. A test thread asserts on the result; a build step or a
    /// hook has nobody to tell and goes on.
    pub(super) fn pass(&self) -> bool {
        let open = self.open.lock().expect("gate");
        let (open, _timed_out) = self
            .changed
            .wait_timeout_while(open, HANG * 4, |open| !*open)
            .expect("gate");
        *open
    }
}

/// A two-file Python repository and a cache-free configuration for it.
pub(super) fn fixture(refresh_policy: RefreshPolicy) -> (tempfile::TempDir, ServerConfig) {
    let dir = tempfile::tempdir().expect("temp repository");
    std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n")
        .expect("write util.py");
    std::fs::write(
        dir.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .expect("write main.py");
    let mut cfg = ServerConfig::new(dir.path().to_path_buf());
    cfg.cache = CacheMode::NoCache;
    cfg.refresh_policy = refresh_policy;
    (dir, cfg)
}

/// What one build attempt does in a test.
pub(super) type BuildStep = Box<dyn FnOnce() -> anyhow::Result<SessionProvider> + Send>;

/// Build the fixture once `gate` is open.
pub(super) fn build_after(gate: &Arc<Gate>, cfg: &ServerConfig) -> BuildStep {
    let gate = Arc::clone(gate);
    let cfg = cfg.clone();
    Box::new(move || -> anyhow::Result<SessionProvider> {
        let _opened = gate.pass();
        SessionProvider::bootstrap(&cfg)
    })
}

/// Fail with `message` once `gate` is open.
pub(super) fn fail_after(gate: &Arc<Gate>, message: &'static str) -> BuildStep {
    let gate = Arc::clone(gate);
    Box::new(move || -> anyhow::Result<SessionProvider> {
        let _opened = gate.pass();
        Err(anyhow::anyhow!(message))
    })
}

/// Fail with `message` at once.
pub(super) fn fail_now(message: &'static str) -> BuildStep {
    Box::new(move || -> anyhow::Result<SessionProvider> { Err(anyhow::anyhow!(message)) })
}

/// Panic on the build thread.
pub(super) fn panic_now() -> BuildStep {
    Box::new(|| -> anyhow::Result<SessionProvider> { panic!("injected builder panic") })
}

/// A builder that plays `steps` in order, one per build attempt, and builds the fixture for
/// every attempt after the last step. Also returns how many times the builder has been called.
pub(super) fn scripted(
    cfg: &ServerConfig,
    steps: Vec<BuildStep>,
) -> (SessionBuilder, Arc<AtomicU64>) {
    let calls = Arc::new(AtomicU64::new(0));
    let counted = Arc::clone(&calls);
    let steps = Mutex::new(VecDeque::from(steps));
    let cfg = cfg.clone();
    let builder: SessionBuilder = Arc::new(move || {
        counted.fetch_add(1, Ordering::SeqCst);
        // The lock is released at the end of this statement, before the step runs.
        let step = steps.lock().expect("build script").pop_front();
        match step {
            Some(step) => step(),
            None => SessionProvider::bootstrap(&cfg),
        }
    });
    (builder, calls)
}

/// Production parts for a lazy daemon, on a manual clock.
pub(super) fn parts_with(builder: SessionBuilder, clock: &Arc<ManualClock>) -> Parts {
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let concrete: Arc<ManualClock> = Arc::clone(clock);
    let shared: Arc<dyn Clock> = concrete;
    parts.clock = shared;
    parts
}

/// A lazy daemon state on a manual clock.
pub(super) fn state_with(builder: SessionBuilder, clock: &Arc<ManualClock>) -> Arc<DaemonState> {
    DaemonState::from_parts(parts_with(builder, clock)).expect("a lazy daemon state")
}

pub(super) fn status(state: &DaemonState) -> BuildStatus {
    state.status().expect("the control lock is not poisoned")
}

/// What a readiness call returned, in a form a test can compare and print.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Ready,
    Warming { elapsed: Duration },
    Failed(String),
    Poisoned,
}

impl Outcome {
    fn of(result: Result<Readiness, Poisoned>) -> Self {
        match result {
            Ok(Readiness::Ready) => Self::Ready,
            Ok(Readiness::Warming { elapsed }) => Self::Warming { elapsed },
            Ok(Readiness::Failed { error }) => Self::Failed(error),
            Err(_) => Self::Poisoned,
        }
    }
}

/// A stand-in for one connection: a thread that owns one `ConnectionBudget` and makes one
/// readiness call each time the test asks, the way a connection's worker makes them one after
/// another.
pub(super) struct Conn {
    ask: mpsc::Sender<()>,
    answers: mpsc::Receiver<Outcome>,
    early: RefCell<Option<Outcome>>,
}

impl Conn {
    pub(super) fn open(state: &Arc<DaemonState>, wait_secs: u64) -> Self {
        let (ask, asked) = mpsc::channel::<()>();
        let (answer, answers) = mpsc::channel();
        let state = Arc::clone(state);
        std::thread::spawn(move || {
            let mut budget = ConnectionBudget::new(&client_settings(wait_secs));
            while asked.recv().is_ok() {
                let outcome = Outcome::of(state.ensure_ready(&mut budget));
                if answer.send(outcome).is_err() {
                    break;
                }
            }
        });
        Self {
            ask,
            answers,
            early: RefCell::new(None),
        }
    }

    /// Start one readiness call. It runs on the connection's thread; collect it with
    /// [`Conn::outcome`].
    pub(super) fn call(&self) {
        self.ask.send(()).expect("the connection thread is alive");
    }

    /// Whether the call has returned. Does not wait.
    pub(super) fn answered(&self) -> bool {
        let mut early = self.early.borrow_mut();
        if early.is_none() {
            *early = self.answers.try_recv().ok();
        }
        early.is_some()
    }

    /// What the call returned. Panics if it has not returned within [`HANG`].
    pub(super) fn outcome(&self) -> Outcome {
        if let Some(outcome) = self.early.borrow_mut().take() {
            return outcome;
        }
        self.answers
            .recv_timeout(HANG)
            .expect("the call returns within the hang limit")
    }
}
