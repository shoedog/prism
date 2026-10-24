//! A held lifecycle sink must not hold serving locks or ownership awaiting destruction.
use super::coordinator::{DaemonState, Parts};
use super::idle_tests::{build, disposed, enabled, N};
use super::test_support::*;
use super::wire_support::{close, connect, CALLEES, REFRESH};
use crate::mcp::RefreshPolicy;
use std::sync::{mpsc, Arc};
use std::time::Duration;

// Always release the sink before an assertion can unwind, including a failed base witness.
struct HeldSink(Arc<Gate>);
impl Drop for HeldSink {
    fn drop(&mut self) {
        self.0.open();
    }
}
fn sink(parts: &mut Parts, event: &'static str) -> (mpsc::Receiver<()>, HeldSink) {
    let (tx, rx) = mpsc::channel();
    let gate = Gate::closed();
    let held = gate.clone();
    let once = std::sync::atomic::AtomicBool::new(false);
    parts.hooks.lifecycle = Some(Arc::new(move |line| {
        if line.contains(&format!("event={event} "))
            && !once.swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            tx.send(()).unwrap();
            let _ = held.pass();
        }
    }));
    (rx, HeldSink(gate))
}
fn probe_control(state: &Arc<DaemonState>) -> (mpsc::Receiver<()>, std::thread::JoinHandle<()>) {
    let state = state.clone();
    let (tx, rx) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        drop(state.control_plane().unwrap());
        tx.send(()).unwrap();
    });
    (rx, thread)
}
fn probe_read(state: &Arc<DaemonState>) -> (mpsc::Receiver<()>, std::thread::JoinHandle<()>) {
    let state = state.clone();
    let (tx, rx) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let c = Conn::open(&state, 0);
        c.call();
        assert_eq!(c.outcome(), Outcome::Ready);
        let (mut client, worker) = connect(&state, client_settings(0));
        client.handshake();
        assert_eq!(client.request(CALLEES)["result"]["isError"], false);
        close(client, worker);
        tx.send(()).unwrap();
    });
    (rx, thread)
}
fn serving(event: &'static str) {
    let (dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock); // exercise idle-off serving parity
    let (reached, held) = sink(&mut parts, event);
    let state = DaemonState::from_parts(parts).unwrap();
    let thread = if event == "initial_build" {
        let state = state.clone();
        std::thread::spawn(move || build(&state))
    } else {
        build(&state);
        if event == "auto_refresh" {
            std::fs::write(
                dir.path().join("util.py"),
                "def helper():\n    return 1234567\n",
            )
            .unwrap();
        }
        let state = state.clone();
        std::thread::spawn(move || {
            let (mut client, worker) = connect(&state, client_settings(600));
            client.handshake();
            client.request(if event == "refresh" { REFRESH } else { CALLEES });
            close(client, worker);
        })
    };
    reached.recv_timeout(HANG).unwrap();
    let (control, control_thread) = probe_control(&state);
    let (read, read_thread) = probe_read(&state);
    let control_ok = control.recv_timeout(Duration::from_secs(3)).is_ok();
    let read_ok = read.recv_timeout(Duration::from_secs(3)).is_ok();
    drop(held);
    thread.join().unwrap();
    control_thread.join().unwrap();
    read_thread.join().unwrap();
    assert!(
        control_ok,
        "{event}: control proceeds while lifecycle sink is held"
    );
    assert!(read_ok, "{event}: another client's readiness and read dispatch proceed while lifecycle sink is held");
}
#[test]
fn daemon_idle_sink_initial_build() {
    serving("initial_build");
}
#[test]
fn daemon_idle_sink_explicit_refresh() {
    serving("refresh");
}
#[test]
fn daemon_idle_sink_auto_refresh() {
    serving("auto_refresh");
}
#[test]
fn daemon_idle_sink_spawn_failure() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    parts.spawner = Box::new(|_| Err(std::io::Error::other("injected spawn failure")));
    let (reached, held) = sink(&mut parts, "initial_build");
    let state = DaemonState::from_parts(parts).unwrap();
    let conn = Conn::open(&state, 0);
    conn.call();
    reached.recv_timeout(HANG).unwrap();
    let (control, thread) = probe_control(&state);
    let other = Conn::open(&state, 0);
    other.call();
    let control_ok = control.recv_timeout(Duration::from_secs(3)).is_ok();
    // This call's spawner also fails, but the sink holds only its first event.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !other.answered() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    let readiness_ok = other.answered();
    drop(held);
    thread.join().unwrap();
    assert!(matches!(conn.outcome(), Outcome::Failed(_)));
    assert!(matches!(other.outcome(), Outcome::Failed(_)));
    assert!(
        control_ok,
        "spawn failure: control proceeds while lifecycle sink is held"
    );
    assert!(
        readiness_ok,
        "spawn failure: another readiness call proceeds while lifecycle sink is held"
    );
}
fn destruction(event: &'static str) {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let (reached, held) = sink(&mut parts, event);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    let weak = state
        .read(|p| Arc::downgrade(&p.session().repo))
        .unwrap()
        .unwrap();
    advance(&clock, &state, N);
    let unloading = {
        let state = state.clone();
        std::thread::spawn(move || state.unload_tick().unwrap())
    };
    reached.recv_timeout(HANG).unwrap();
    let (control, thread) = probe_control(&state);
    let other = Conn::open(&state, 0);
    other.call();
    let readiness = other.outcome(); // unloaded/disposal has no provider to read
    let control_ok = control.recv_timeout(Duration::from_secs(3)).is_ok();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while weak.upgrade().is_some() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    let dead = weak.upgrade().is_none();
    drop(held);
    unloading.join().unwrap();
    thread.join().unwrap();
    disposed(&state);
    assert!(
        control_ok,
        "{event}: control proceeds while lifecycle sink is held"
    );
    assert!(
        matches!(readiness, Outcome::Ready | Outcome::Warming { .. }),
        "{readiness:?}"
    );
    assert!(
        dead,
        "{event}: provider Weak is dead while lifecycle sink is held"
    );
}
#[test]
fn daemon_idle_sink_unload() {
    destruction("unload");
}
#[test]
fn daemon_idle_sink_disposal_start() {
    destruction("disposal_start");
}

// Test both rejection before slot publication and retraction during settlement.
fn rejected_publication(publication_first: bool, disposal_failed: bool) {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let resume = Gate::closed();
    let held_return = resume.clone();
    let (tx, weak_rx) = mpsc::channel();
    let build_cfg = cfg.clone();
    let (builder, _) = scripted(
        &cfg,
        vec![Box::new(move || {
            let provider = crate::mcp::SessionProvider::bootstrap(&build_cfg)?;
            tx.send(Arc::downgrade(&provider.session().repo)).unwrap();
            let _ = held_return.pass();
            Ok(provider)
        })],
    );
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let published = Gate::closed();
    let settle = Gate::closed();
    if publication_first {
        let (reached, held) = (published.clone(), settle.clone());
        parts.hooks.published = Some(Arc::new(move |_| {
            reached.open();
            let _ = held.pass();
        }));
    }
    let (reached, held) = sink(&mut parts, "initial_build");
    let state = DaemonState::from_parts(parts).unwrap();
    let conn = Conn::open(&state, 600);
    conn.call();
    let weak = weak_rx.recv_timeout(HANG).unwrap();
    if publication_first {
        resume.open();
        assert!(published.pass(), "provider published before drain");
    }
    if disposal_failed {
        state.control_plane().unwrap().idle.disposal_failed = true;
    } else {
        state
            .drain(super::control::StopReason::Signal, || {})
            .unwrap();
    }
    resume.open();
    settle.open();
    reached.recv_timeout(HANG).unwrap();
    let (control, thread) = probe_control(&state);
    let control_ok = control.recv_timeout(Duration::from_secs(3)).is_ok();
    let empty = state.provider.try_read().is_ok_and(|slot| slot.is_none());
    let dead = weak.upgrade().is_none();
    let answer = conn.outcome();
    drop(held);
    thread.join().unwrap();
    assert!(
        control_ok,
        "rejected publication: control proceeds while terminal sink is held"
    );
    assert!(
        empty,
        "rejected publication: slot empty while terminal sink is held"
    );
    assert!(
        matches!(answer, Outcome::Failed(_)),
        "attempt remains failed: {answer:?}"
    );
    assert!(
        dead,
        "rejected publication: provider Weak is dead while terminal sink is held"
    );
}
#[test]
fn daemon_idle_sink_rejected_publication() {
    rejected_publication(false, false);
}
#[test]
fn daemon_idle_sink_disposal_failed_publication() {
    rejected_publication(false, true);
}
#[test]
fn daemon_idle_sink_retired_publication() {
    rejected_publication(true, false);
}
