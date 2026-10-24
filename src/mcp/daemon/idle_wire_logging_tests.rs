//! Behavioral lifecycle/log/drop witnesses; no assertions on new wire structs.
use super::coordinator::{DaemonState, Parts};
use super::idle_tests::{build, disposed, enabled, unload};
use super::test_support::*;
use super::wire_support::{close, connect, CALLEES, REFRESH};
use crate::mcp::RefreshPolicy;
use std::sync::{Arc, Mutex};

type Lines = Arc<Mutex<Vec<String>>>;
fn capture(parts: &mut Parts) -> Lines {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let out = lines.clone();
    parts.hooks.lifecycle = Some(Arc::new(move |s| out.lock().unwrap().push(s.to_owned())));
    lines
}
fn require_event(lines: &Lines, event: &str, outcome: &str) {
    wait_until("lifecycle event after unlocked notification", || {
        lines.lock().unwrap().iter().any(|s| {
            s.contains(&format!("event={event} ")) && s.contains(&format!("outcome={outcome}"))
        })
    });
    let lines = lines.lock().unwrap();
    assert!(
        lines
            .iter()
            .any(|s| s.contains(&format!("event={event} "))
                && s.contains(&format!("outcome={outcome}"))),
        "missing {event}/{outcome}: {lines:?}"
    );
}

#[test]
fn daemon_idle_logging_and_relief() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    // Tool release remains pending until the timer accepts an unload.
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    client.request(CALLEES);
    state.unload_tick().unwrap();
    unload(&clock, &state);
    assert_eq!(
        status(&state).state.word(),
        "unloaded",
        "unloaded is distinct from initial idle"
    );
    assert_eq!(
        status(&state).generation,
        0,
        "wire generation hides retained G"
    );
    disposed(&state);
    build(&state);
    close(client, worker);
    for event in [
        "initial_build",
        "reload",
        "last_release",
        "unload",
        "disposal_start",
        "disposal_end",
        "relief",
    ] {
        require_event(&lines, event, "complete");
    }
    for s in lines.lock().unwrap().iter() {
        for field in [
            "identity=",
            "pid=",
            "event=",
            "reason=",
            "observed_idle_secs=",
            "configured_idle_secs=10",
            "timestamp_ms=",
            "duration_ns=",
            "attempt=",
            "generation=",
            "outcome=",
            "released_bytes=",
            "mono_ms=",
        ] {
            assert!(s.contains(field), "missing {field}: {s}");
        }
        assert!(
            !s.chars().any(char::is_control),
            "unsanitized lifecycle line: {s:?}"
        );
        assert!(s.len() <= 1000);
    }
    let guard = lines.lock().unwrap();
    for (event, reason) in [
        ("initial_build", "demand_or_startup"),
        ("reload", "same_snapshot"),
    ] {
        let line = guard
            .iter()
            .find(|s| s.contains(&format!("event={event} ")))
            .unwrap();
        assert!(line.contains(&format!("reason={reason} ")), "{line}");
    }
}

#[test]
fn daemon_idle_relief_after_drop() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let weak = Arc::new(Mutex::new(
        None::<std::sync::Weak<crate::repo_loader::LoadedRepo>>,
    ));
    let check = weak.clone();
    let early = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed_early = early.clone();
    let lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let out = lines.clone();
    parts.hooks.lifecycle = Some(Arc::new(move |s| {
        if s.contains("event=relief ") {
            observed_early.store(
                check.lock().unwrap().as_ref().unwrap().upgrade().is_some(),
                std::sync::atomic::Ordering::SeqCst,
            );
        }
        out.lock().unwrap().push(s.to_owned());
    }));
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    *weak.lock().unwrap() = state.read(|p| Arc::downgrade(&p.session().repo)).unwrap();
    unload(&clock, &state);
    disposed(&state);
    assert!(
        !early.load(std::sync::atomic::Ordering::SeqCst),
        "relief ran before whole provider drop"
    );
    require_event(&lines, "relief", "complete");
    assert!(lines
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.contains("event=relief ") && s.contains("released_bytes=0")));
    assert_eq!(
        crate::mcp::idle::pressure_relief(false),
        0,
        "Arm A matched no-op"
    );
    #[cfg(not(target_os = "macos"))]
    assert_eq!(
        crate::mcp::idle::pressure_relief(true),
        0,
        "non-macOS is no-op"
    );
}

#[test]
fn daemon_idle_build_failure_event() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let ready_cfg = cfg.clone();
    let (builder, _) = scripted(
        &cfg,
        vec![
            Box::new(|| anyhow::bail!("build\nfailed\x1b{}", "x".repeat(2000))),
            Box::new(move || crate::mcp::SessionProvider::bootstrap(&ready_cfg)),
            Box::new(|| anyhow::bail!("reload failed")),
        ],
    );
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    let c = Conn::open(&state, 600);
    c.call();
    assert!(matches!(c.outcome(), Outcome::Failed(_)));
    require_event(&lines, "initial_build", "failed");
    let guard = lines.lock().unwrap();
    let s = &guard[0];
    assert!(
        s.contains("build�failed�"),
        "error must be sanitized: {s:?}"
    );
    assert!(s.len() <= 1000 && !s.chars().any(char::is_control));
    assert!(
        s.ends_with(" mono_ms=0"),
        "capped error must retain appended mono_ms: {s}"
    );
    drop(guard);
    build(&state);
    unload(&clock, &state);
    disposed(&state);
    let c = Conn::open(&state, 600);
    c.call();
    assert!(matches!(c.outcome(), Outcome::Failed(_)));
    require_event(&lines, "reload", "failed");
}

#[test]
fn daemon_idle_refresh_events() {
    let (dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    client.request(CALLEES);
    client.request(REFRESH);
    state
        .write(|p| p.force_next_refresh_error_for_tests("explicit failure"))
        .unwrap();
    client.request(REFRESH);
    std::fs::write(dir.path().join("util.py"), "def helper():\n    return 12\n").unwrap();
    let modified = std::fs::metadata(dir.path().join("util.py"))
        .unwrap()
        .modified()
        .unwrap();
    client.request(CALLEES);
    std::fs::write(
        dir.path().join("util.py"),
        "def helper():\n    return 123456\n",
    )
    .unwrap();
    // Equal mtime deliberately models a coarse filesystem clock; the changed size must suffice.
    std::fs::File::options()
        .write(true)
        .open(dir.path().join("util.py"))
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    state
        .write(|p| p.force_next_refresh_error_for_tests("auto failure"))
        .unwrap();
    client.request(CALLEES);
    close(client, worker);
    for event in ["refresh", "auto_refresh"] {
        for outcome in ["complete", "failed"] {
            require_event(&lines, event, outcome);
        }
    }
}

#[test]
fn daemon_idle_log_caps_preserved() {
    use std::sync::atomic::AtomicU64;
    let bytes = AtomicU64::new((8 << 20) - 30);
    let mut sink = Vec::new();
    super::log::write_to(&"x".repeat(2000), &bytes, &mut sink);
    super::log::write_to("later", &bytes, &mut sink);
    let text = String::from_utf8(sink).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains("log reached its cap"));
    let mut sink = Vec::new();
    super::log::write_to(
        &format!("a\nb\x1b{}", "é".repeat(2000)),
        &AtomicU64::new(0),
        &mut sink,
    );
    let text = String::from_utf8(sink).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains("a�b�"));
    assert!(text.len() <= 1020);
}
