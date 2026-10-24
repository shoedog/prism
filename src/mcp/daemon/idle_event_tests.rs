//! Numeric producer-time, reason, volume and locked-fingerprint witnesses.
use super::clock::Clock;
use super::coordinator::{DaemonState, Parts};
use super::idle_tests::{build, disposed, enabled, unload, N};
use super::test_support::*;
use super::wire_support::{close, connect, CALLEES};
use crate::mcp::{RefreshPolicy, SessionProvider};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

type Lines = Arc<Mutex<Vec<String>>>;
fn capture(parts: &mut Parts) -> Lines {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let out = lines.clone();
    parts.hooks.lifecycle = Some(Arc::new(move |line| {
        out.lock().unwrap().push(line.to_owned())
    }));
    lines
}
fn event(lines: &Lines, name: &str) -> String {
    wait_until(name, || {
        lines
            .lock()
            .unwrap()
            .iter()
            .any(|s: &String| s.contains(&format!("event={name} ")))
    });
    lines
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|s| s.contains(&format!("event={name} ")))
        .unwrap()
        .clone()
}
// First occurrence is metadata; the capped-error witness checks this even for token-like errors.
fn number(line: &str, field: &str) -> u128 {
    line.split_once(&format!(" {field}="))
        .unwrap()
        .1
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

fn reason(lines: &Lines, name: &str, expected: &str) {
    let line = event(lines, name);
    assert!(
        line.contains(&format!("reason={expected} ")),
        "{name} reason must be {expected}: {line}"
    );
}
#[test]
fn daemon_idle_snapshot_reasons() {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    reason(&lines, "initial_build", "demand_or_startup");
    unload(&clock, &state);
    disposed(&state);
    build(&state);
    reason(&lines, "reload", "same_snapshot");
    std::fs::write(
        dir.path().join("util.py"),
        "def changed():\n    return 123456\n",
    )
    .unwrap();
    unload(&clock, &state);
    disposed(&state);
    let before = lines.lock().unwrap().len();
    build(&state);
    wait_until("new reload line", || lines.lock().unwrap().len() > before);
    reason(&lines, "reload", "changed_snapshot");
}
#[test]
fn daemon_idle_diverged_same_snapshot_reason() {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let raced_cfg = cfg.clone();
    let path = dir.path().join("util.py");
    let original = std::fs::read(&path).unwrap();
    let edit = path.clone();
    let (builder, _) = scripted(
        &cfg,
        vec![Box::new(move || {
            SessionProvider::bootstrap_observed(&raced_cfg, &mut |stage| {
                if stage == crate::mcp::session::BuildStage::RepoLoaded {
                    std::fs::write(&edit, "def other():\n    return 2\n").unwrap();
                }
            })
        })],
    );
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    let old = state.read(|p| p.idle_publication()).unwrap().unwrap();
    assert!(old.diverged);
    std::fs::write(path, original).unwrap();
    unload(&clock, &state);
    disposed(&state);
    build(&state);
    let new = state.read(|p| p.idle_publication()).unwrap().unwrap();
    assert_eq!(old.fingerprint, new.fingerprint);
    assert_eq!(old.generation, new.generation);
    assert!(!new.diverged);
    reason(&lines, "reload", "changed_snapshot");
}
#[test]
fn daemon_idle_release_volume() {
    for idle in [false, true] {
        let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
        let (builder, _) = scripted(&cfg, vec![]);
        let clock = ManualClock::new();
        let mut parts = parts_with(builder, &clock);
        if idle {
            parts = enabled(parts);
        }
        let lines = capture(&mut parts);
        let state = DaemonState::from_parts(parts).unwrap();
        let (mut client, worker) = connect(&state, client_settings(600));
        client.handshake();
        for cycle in 0..2 {
            for _ in 0..32 {
                client.request(CALLEES);
                state.unload_tick().unwrap();
            }
            assert_eq!(
                lines
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|s| s.contains("event=last_release "))
                    .count(),
                cycle * usize::from(idle),
                "no per-call last_release, including idle off"
            );
            if idle {
                let release = state
                    .control_plane()
                    .unwrap()
                    .idle
                    .last_release_event
                    .unwrap();
                unload(&clock, &state);
                disposed(&state);
                let logs = lines.lock().unwrap();
                let pair: Vec<_> = logs
                    .iter()
                    .filter(|s| s.contains("event=last_release ") || s.contains("event=unload "))
                    .collect();
                assert_eq!(pair.len(), (cycle + 1) * 2, "one release per unload");
                let line = pair[cycle * 2];
                assert!(line.contains("event=last_release "));
                assert!(pair[cycle * 2 + 1].contains("event=unload "));
                assert_eq!(number(line, "attempt"), release.attempt as u128);
                assert_eq!(number(line, "generation"), release.generation as u128);
            }
        }
        close(client, worker);
    }
}
fn producer_timestamps(before_unload: i64, during_disposal: i64) {
    const EPOCH: u64 = 1_800_000_000_000;
    const SLEEP: u64 = 60_000; // wall advances independently of daemon monotonic time
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let origin = clock.now();
    let mut parts = enabled(parts_with(builder, &clock));
    let wall_clock = clock.clone();
    let wall_shift = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let shift = wall_shift.clone();
    parts.hooks.wall_time = Some(Arc::new(move || {
        UNIX_EPOCH
            + Duration::from_millis(EPOCH + shift.load(std::sync::atomic::Ordering::SeqCst))
            + wall_clock.now().duration_since(origin)
    }));
    let dispose_clock = clock.clone();
    let dispose_shift = wall_shift.clone();
    parts.hooks.dispose = Some(Arc::new(move || {
        dispose_clock.advance(Duration::from_secs(7));
        dispose_shift.store(
            (SLEEP as i64 + before_unload + during_disposal) as u64,
            std::sync::atomic::Ordering::SeqCst,
        );
    }));
    let lines = Arc::new(Mutex::new(Vec::new()));
    let out = lines.clone();
    let emit_clock = clock.clone();
    parts.hooks.lifecycle = Some(Arc::new(move |line| {
        out.lock().unwrap().push(line.to_owned());
        if line.contains("event=disposal_start ") {
            emit_clock.advance(Duration::from_secs(3));
        }
    }));
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    assert_eq!(
        number(&event(&lines, "initial_build"), "timestamp_ms"),
        EPOCH as u128
    );
    // Host sleep occurs after log construction and before the measured release window.
    wall_shift.store(SLEEP, std::sync::atomic::Ordering::SeqCst);
    advance(&clock, &state, 2);
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    client.request(CALLEES); // release at mono 2, emitted only after mono 12
    let release = state
        .control_plane()
        .unwrap()
        .idle
        .last_release_event
        .unwrap();
    assert_eq!(release.at.duration_since(origin), Duration::from_secs(2));
    wall_shift.store(
        (SLEEP as i64 + before_unload) as u64,
        std::sync::atomic::Ordering::SeqCst,
    );
    unload(&clock, &state);
    disposed(&state);
    close(client, worker);
    for (name, producer_ms, shift) in [
        ("last_release", 2_000, 0),
        ("disposal_start", 12_000, before_unload),
        ("unload", 12_000, before_unload),
        ("disposal_end", 19_000, before_unload + during_disposal),
        ("relief", 22_000, before_unload + during_disposal),
    ] {
        let line = event(&lines, name);
        assert_eq!(
            number(&line, "timestamp_ms"),
            ((EPOCH + SLEEP) as i64 + shift) as u128 + producer_ms,
            "{name}: captured producer wall survives delayed emission (pre-unload jump={before_unload}, disposal jump={during_disposal})"
        );
        assert_eq!(
            number(&line, "mono_ms"),
            producer_ms,
            "{name}: mono_ms equals producer offset"
        );
    }
    let start = event(&lines, "disposal_start");
    let end = event(&lines, "disposal_end");
    let relief = event(&lines, "relief");
    assert!(number(&start, "timestamp_ms") < number(&end, "timestamp_ms"));
    assert!(number(&end, "timestamp_ms") <= number(&relief, "timestamp_ms"));
    let logs = lines.lock().unwrap();
    let position = |name| {
        logs.iter()
            .position(|s| s.contains(&format!("event={name} ")))
            .unwrap()
    };
    assert!(position("disposal_start") < position("disposal_end"));
    assert!(position("disposal_end") < position("relief"));
}
#[test]
fn daemon_idle_producer_timestamps() {
    producer_timestamps(0, 0); // wall shift before release, then delayed emission
}
#[test]
fn daemon_idle_producer_wall_jump_forward() {
    producer_timestamps(30_000, 0);
}
#[test]
fn daemon_idle_producer_wall_jump_backward() {
    producer_timestamps(-30_000, 0);
}
#[test]
fn daemon_idle_producer_wall_jump_during_disposal() {
    producer_timestamps(0, 30_000);
}
#[test]
fn daemon_idle_fingerprint_duration() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let delay = Duration::from_millis(40);
    parts.hooks.fingerprint = Some(Arc::new(move || std::thread::sleep(delay)));
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    advance(&clock, &state, N);
    state.unload_tick().unwrap();
    disposed(&state);
    let line = event(&lines, "unload");
    assert!(
        number(&line, "duration_ns") >= delay.as_nanos(),
        "unload duration includes locked fingerprint seam: {line}"
    );
}
