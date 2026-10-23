use super::coordinator::DaemonState;
use super::idle_tests::*;
use super::test_support::*;
use crate::mcp::RefreshPolicy;
use std::sync::{mpsc, Arc};

#[test]
fn daemon_idle_rearm_and_dispose() {
    for writer in [false, true] {
        let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
        let (builder, _) = scripted(&cfg, vec![]);
        let clock = ManualClock::new();
        let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
        build(&state);
        let changed = state.clone();
        let (tx, rx) = mpsc::channel();
        let gate = Gate::closed();
        let held = gate.clone();
        let worker = std::thread::spawn(move || {
            let hold = || {
                tx.send(()).unwrap();
                assert!(held.pass());
            };
            if writer {
                changed.write(|_| hold()).unwrap()
            } else {
                changed.read(|_| hold()).unwrap()
            }
        });
        rx.recv_timeout(HANG).unwrap();
        advance(&clock, &state, N);
        state.unload_tick().unwrap();
        let deadline = state
            .unload_deadline()
            .unwrap()
            .expect("contention re-arms");
        assert!(
            deadline > state.now(),
            "re-arm must be strictly in the future"
        );
        assert!(
            deadline <= state.now() + std::time::Duration::from_secs(1),
            "re-arm must stay bounded"
        );
        assert_eq!(
            status(&state).state,
            crate::mcp::shared::preamble::BuildState::Ready,
            "reader/writer contention must preserve Ready without blocking tick"
        );
        gate.open();
        worker.join().unwrap();
        advance(&clock, &state, 1);
        state.unload_tick().unwrap();
        assert!(
            !resident(&state),
            "contended deadline must re-arm and eventually unload"
        );
        disposed(&state);
    }
}

#[test]
fn daemon_idle_disposal_off_loop() {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let (tx, reached) = mpsc::channel();
    let gate = Gate::closed();
    let held = gate.clone();
    parts.hooks.dispose = Some(Arc::new(move || {
        tx.send(std::thread::current().id()).unwrap();
        let _ = held.pass();
    }));
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    let (base, runtime) = super::server_support::runtime_base();
    let mut running = super::server_support::start_with(
        state.clone(),
        clock.clone(),
        dir,
        base,
        runtime,
        super::server_support::options(4),
        |_| {},
    );
    // An admitted silent client prevents daemon retirement without pinning the index.
    let _client = running.admitted();
    running.advance_and_wake(N);
    let disposer = reached.recv_timeout(HANG).expect("disposal worker starts");
    let off_loop = disposer != running.handle.as_ref().unwrap().thread().id();
    if !off_loop {
        gate.open();
    }
    assert!(
        off_loop,
        "provider destruction must not run on the server loop"
    );
    let held = state.control_is_held();
    assert!(!held, "disposal hook must run outside control lock");
    assert!(
        state.read(|_| ()).unwrap().is_none(),
        "slot lock must also be free at disposal"
    );
    running.settle(); // would hang if the destructor ran on the server loop
    gate.open();
    disposed(&state);
    running.waker.request_stop();
    running.stopped();
}

#[test]
fn daemon_idle_disposal_panic_fatal() {
    use super::fatal::{Domain, Fatal};
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let log = FatalLog::install(&mut parts);
    parts.hooks.dispose = Some(Arc::new(|| panic!("injected destructor panic")));
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    unload(&clock, &state);
    wait_until("fatal disposal report", || !log.events().is_empty());
    disposed(&state);
    assert_eq!(log.events(), vec![Fatal::Panicked(Domain::Disposal)]);
    let conn = Conn::open(&state, 0);
    conn.call();
    assert!(
        matches!(conn.outcome(), Outcome::Failed(_)),
        "destructor panic must forbid reload"
    );
}

#[test]
fn daemon_idle_poison_is_fatal() {
    use super::fatal::Fatal;
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let log = FatalLog::install(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    let changed = state.clone();
    assert!(
        std::thread::spawn(move || changed.write::<()>(|_| panic!("poison the provider")))
            .join()
            .is_err()
    );
    advance(&clock, &state, N);
    assert!(
        state.unload_tick().is_err(),
        "poison cannot be treated as retryable contention"
    );
    assert!(log.events().contains(&Fatal::Poisoned { lock: "provider" }));
}

#[test]
fn daemon_idle_spawn_failure_preserves_ready() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let attempts = Arc::new(AtomicU64::new(0));
    let counted = attempts.clone();
    parts.disposal_spawner = Box::new(move |job| {
        if counted.fetch_add(1, Ordering::SeqCst) == 0 {
            Err(std::io::Error::other("injected spawn failure"))
        } else {
            std::thread::spawn(job);
            Ok(())
        }
    });
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    advance(&clock, &state, N);
    state.unload_tick().unwrap();
    assert!(
        resident(&state),
        "spawn failure must preserve the original provider"
    );
    assert!(!state.activity_counts().2);
    assert!(state.unload_deadline().unwrap().unwrap() > state.now());
    advance(&clock, &state, 1);
    state.unload_tick().unwrap();
    assert!(!resident(&state));
    disposed(&state);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[test]
fn daemon_idle_shutdown_rejects_late_build() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    let conn = Conn::open(&state, 0);
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));
    advance(&clock, &state, N * 2);
    state.unload_tick().unwrap();
    state
        .drain(super::control::StopReason::Signal, || {})
        .unwrap();
    gate.open();
    wait_until("late build settlement", || {
        status(&state).state == crate::mcp::shared::preamble::BuildState::Failed
    });
    assert!(!resident(&state), "shutdown must reject late publication");
}

#[test]
fn daemon_idle_shutdown_between_slot_and_ready() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let (hook, reached, gate) = barrier();
    parts.hooks.published = Some(Arc::new(move |_| hook()));
    let (tx, rx) = mpsc::channel();
    parts.hooks.settled = Some(Arc::new(move |_| {
        tx.send(()).unwrap();
    }));
    let state = DaemonState::from_parts(parts).unwrap();
    let conn = Conn::open(&state, 0);
    conn.call();
    assert!(matches!(conn.outcome(), Outcome::Warming { .. }));
    reached.recv_timeout(HANG).unwrap();
    state
        .drain(super::control::StopReason::Signal, || {})
        .unwrap();
    gate.open();
    rx.recv_timeout(HANG).unwrap();
    assert!(
        !resident(&state),
        "a drain between slot publication and Ready must retract the late provider"
    );
    assert_ne!(
        status(&state).state,
        crate::mcp::shared::preamble::BuildState::Ready
    );
}

#[test]
fn daemon_idle_shutdown_during_disposal() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let (hook, reached, gate) = barrier();
    parts.hooks.dispose = Some(hook);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    unload(&clock, &state);
    reached.recv_timeout(HANG).unwrap();
    state
        .drain(super::control::StopReason::Signal, || {})
        .unwrap();
    let conn = Conn::open(&state, 0);
    conn.call();
    let answer = conn.outcome();
    gate.open();
    disposed(&state);
    assert!(
        matches!(answer, Outcome::Failed(_)),
        "draining must reject calls even during disposal: {answer:?}"
    );
    assert_eq!(builds.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(!resident(&state));
}
