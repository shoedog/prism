//! Real shared-daemon production hooks, captured through existing test support.
use super::super::{
    coordinator::{DaemonState, Parts},
    idle_tests::{build, enabled, unload},
    test_support::*,
};
use crate::mcp::{CacheMode, RefreshPolicy, SessionProvider};
use crate::navigation::queries::{callees, nodes_at};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
#[derive(Default)]
struct Capture {
    records: Mutex<Vec<String>>,
    changed: std::sync::Condvar,
    completed: Mutex<Vec<usize>>,
    completion: std::sync::Condvar,
}
type Lines = Arc<Capture>;
impl Capture {
    fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'_, Vec<String>>> {
        self.records.lock()
    }
    fn push(&self, s: &str) {
        self.records.lock().unwrap().push(s.to_owned());
        self.changed.notify_all();
    }
    fn done(&self, attempt: usize) {
        let (done, timeout) = self
            .completion
            .wait_timeout_while(self.completed.lock().unwrap(), HANG, |done| {
                !done.contains(&attempt)
            })
            .unwrap();
        assert!(
            done.contains(&attempt),
            "unfinished build attempt {attempt}: {timeout:?}"
        );
    }
}
fn capture(parts: &mut Parts) -> Lines {
    let lines = Arc::new(Capture::default());
    let out = lines.clone();
    parts.hooks.lifecycle = Some(Arc::new(move |s| out.push(s)));
    parts.lifecycle.context.observe = parts.hooks.lifecycle.clone();
    let original = std::mem::replace(&mut parts.spawner, Box::new(|_| unreachable!()));
    let out = lines.clone();
    let attempts = std::sync::atomic::AtomicUsize::new(0);
    parts.spawner = Box::new(move |job| {
        let attempt = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let out = out.clone();
        original(Box::new(move || {
            job();
            out.completed.lock().unwrap().push(attempt);
            out.completion.notify_all();
        }))
    });
    lines
}
fn disposed(state: &DaemonState) {
    let (control, _) = state
        .changed
        .wait_timeout_while(state.control.lock().unwrap(), HANG, |c| c.idle.disposing)
        .unwrap();
    assert!(!control.idle.disposing, "unfinished disposal");
}
fn line(lines: &Lines, event: &str) -> String {
    let (logs, _) = lines
        .changed
        .wait_timeout_while(lines.lock().unwrap(), HANG, |logs| {
            !logs.iter().any(|s| value(s, "event") == event)
        })
        .unwrap();
    logs.iter()
        .rev()
        .find(|s| value(s, "event") == event)
        .unwrap_or_else(|| panic!("missing event={event}"))
        .clone()
}
use crate::mcp::serving_events::build_tests::{number, value};
#[test]
fn o_sidecar_attempt_only() {
    for query in [false, true] {
        for mode in ["hit", "miss", "io_failure", "ineligible"] {
            let (_dir, mut cfg) = fixture(RefreshPolicy::WarnOnly);
            cfg.cache = CacheMode::Dir(cfg.repo_root.join("cache"));
            let p = SessionProvider::bootstrap(&cfg).unwrap();
            callees(p.session(), Some("run"), None, None, 1).unwrap();
            let cache = crate::navigation::cache::nav_cache_subdir(
                &cfg.repo_root.join("cache"),
                &p.session().repo,
            );
            drop(p);
            let path = cache.join("resolved-call-edge-index.bin");
            if mode == "miss" {
                std::fs::remove_file(&path).unwrap();
            }
            if mode == "io_failure" {
                std::fs::remove_file(&path).unwrap();
                std::fs::create_dir(&path).unwrap();
            }
            if mode == "ineligible" {
                std::fs::remove_dir_all(cache).unwrap();
            }
            let (builder, _) = scripted(&cfg, vec![]);
            let clock = ManualClock::new();
            let mut parts = enabled(parts_with(builder, &clock));
            let lines = capture(&mut parts);
            let state = DaemonState::from_parts(parts).unwrap();
            build(&state);
            lines.done(1);
            assert_eq!(
                value(&line(&lines, "publication"), "sidecar"),
                "not_attempted"
            );
            state
                .read(|p| {
                    assert_eq!(p.session().index.call_edges_loaded_from_cache(), None);
                    if query {
                        callees(p.session(), Some("run"), None, None, 1).unwrap();
                    } else {
                        assert!(!nodes_at(p.session(), "main.py", 3).items.is_empty());
                    }
                })
                .unwrap();
            unload(&clock, &state);
            disposed(&state);
            let expected = if !query || mode == "ineligible" {
                "not_attempted"
            } else if crate::cpg_cache::binary_input_dirty() {
                "refused_dirty"
            } else {
                mode
            };
            assert_eq!(
                value(&line(&lines, "unload"), "sidecar"),
                expected,
                "late outcome exports in daemon"
            );
            for event in ["disposal_start", "disposal_end", "relief"] {
                assert_eq!(
                    value(&line(&lines, event), "sidecar"),
                    "not_applicable",
                    "{event} must not contradict unload {expected}"
                );
                assert_eq!(value(&line(&lines, event), "reason"), "idle");
            }
        }
    }
}
#[test]
fn o_lifecycle_events() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    lines.done(1);
    unload(&clock, &state);
    disposed(&state);
    build(&state);
    lines.done(2);
    for e in "build_start build_finish publication initial_build reload last_release unload disposal_start disposal_end relief".split_whitespace() {
        line(&lines, e);
    }
    let logs = lines.lock().unwrap();
    assert_eq!(
        logs.iter()
            .filter(|s| s.contains("event=initial_build "))
            .count(),
        1,
        "no duplicate daemon emitter"
    );
    drop(logs);
    assert!([1, 2].into_iter().all(|attempt| {
        let logs = lines.lock().unwrap();
        let find = |event| {
            logs.iter()
                .find(|s| value(s, "event") == event && number(s, "attempt") == attempt)
                .unwrap()
        };
        number(find("build_finish"), "mono_ns") <= number(find("publication"), "mono_ns")
    }));
}
#[test]
fn o_event_identity_and_clock() {
    if std::env::var_os("PRISM_O_STARTUP_CHILD").is_some() {
        let (_dir, mut cfg) = fixture(RefreshPolicy::WarnOnly);
        cfg.cache = CacheMode::Dir(cfg.repo_root.join("cache"));
        super::super::run::run_daemon(super::super::run::DaemonOptions {
            config: cfg,
            idle_policy: crate::mcp::idle::IdlePolicy::default(),
            idle_ttl: Duration::ZERO,
            max_clients: 1,
            expected_key: None,
        })
        .unwrap();
        return;
    }
    let base = tempfile::Builder::new()
        .prefix("or1")
        .tempdir_in("/tmp")
        .unwrap();
    let before = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u128;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "mcp::daemon::idle::serving_event_tests::o_event_identity_and_clock",
            "--exact",
            "--nocapture",
        ])
        .env("PRISM_O_STARTUP_CHILD", "1")
        .env(
            crate::mcp::shared::runtime_dir::RUNTIME_DIR_ENV,
            base.path(),
        )
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "startup child: {stderr}");
    let startup = stderr
        .lines()
        .find(|s| s.contains(" event=startup "))
        .unwrap();
    let after = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u128;
    let instance = number(startup, "instance_start");
    assert!(
        instance >= before * 1_000_000_000 && instance <= after * 1_000_000_000,
        "daemon instance_start must use epoch nanoseconds: {startup}"
    );
    crate::mcp::serving_events::build_tests::sidecar_scope(startup);
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let wall = Arc::new(std::sync::atomic::AtomicU64::new(123));
    let now = wall.clone();
    parts.hooks.wall_time = Some(Arc::new(move || {
        UNIX_EPOCH + Duration::from_secs(now.load(std::sync::atomic::Ordering::SeqCst))
    }));
    parts.hooks.dispose = Some(Arc::new(move || {
        wall.store(456, std::sync::atomic::Ordering::SeqCst);
    }));
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    lines.done(1);
    unload(&clock, &state);
    disposed(&state);
    let mut logs = lines.lock().unwrap().clone();
    logs.sort_by_key(|s| number(s, "sequence"));
    let mut seq = 0;
    for s in logs.iter() {
        crate::mcp::serving_events::build_tests::sidecar_scope(s);
        let expected = if matches!(value(s, "event"), "disposal_end" | "relief") {
            456
        } else {
            123
        };
        assert_eq!(number(s, "epoch_ns"), expected * 1_000_000_000);
        assert!(number(s, "instance_start") > 0);
        assert_eq!(number(s, "pid"), std::process::id() as u128);
        assert!(number(s, "sequence") > seq);
        seq = number(s, "sequence");
    }
}
#[test]
fn o_relief_pair_after_drop() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let lines = capture(&mut parts);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    lines.done(1);
    let weak = state
        .read(|p| Arc::downgrade(&p.session().repo))
        .unwrap()
        .unwrap();
    unload(&clock, &state);
    disposed(&state);
    assert!(weak.upgrade().is_none());
    let s = line(&lines, "relief");
    assert!(number(&s, "sample_before_ns") <= number(&s, "sample_after_ns"));
    for k in "heap_before heap_after footprint_before footprint_after".split_whitespace() {
        assert!(!value(&s, k).is_empty());
    }
    assert_eq!(value(&s, "released_bytes"), "0");
}
#[test]
fn o_event_fields_preserve_serving() {
    use super::super::wire_support::{close, connect, CALLEES, REFRESH};
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let lines = capture(&mut parts);
    let owner = Arc::new(Mutex::new(std::sync::Weak::<DaemonState>::new()));
    let check = owner.clone();
    let seen = lines.clone();
    parts.hooks.lifecycle = Some(Arc::new(move |s| {
        if s.contains("event=refresh ") {
            let state = check.lock().unwrap().upgrade().unwrap();
            assert_eq!(state.activity_counts(), (0, 0, false));
            assert!(state.provider.try_write().is_ok());
        }
        seen.push(s);
    }));
    let state = DaemonState::from_parts(parts).unwrap();
    *owner.lock().unwrap() = Arc::downgrade(&state);
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    let before = client.request(CALLEES);
    client.request(REFRESH);
    let after = client.request(CALLEES);
    close(client, worker);
    lines.done(1);
    assert_eq!(before["result"]["isError"], after["result"]["isError"]);
    assert_eq!(value(&line(&lines, "refresh"), "outcome"), "complete");
    unload(&clock, &state);
    disposed(&state);
}

#[test]
fn o_daemon_publication_facts() {
    for startup in [
        crate::mcp::StartupMode::Lazy,
        crate::mcp::StartupMode::Background,
    ] {
        let (_dir, mut cfg) = fixture(RefreshPolicy::WarnOnly);
        cfg.cache = CacheMode::Dir(cfg.repo_root.join("cache"));
        let (builder, _) = scripted(&cfg, vec![]);
        let clock = ManualClock::new();
        let mut parts = enabled(parts_with(builder, &clock));
        parts.startup = startup;
        let lines = capture(&mut parts);
        let state = DaemonState::from_parts(parts).unwrap();
        for (attempt, expected) in [(1, "miss"), (2, "hit")] {
            build(&state);
            lines.done(attempt);
            let s = line(&lines, "publication");
            assert_eq!(number(&s, "attempt"), attempt as u128);
            crate::mcp::serving_events::build_tests::publication_facts(&s, expected);
            unload(&clock, &state);
            disposed(&state);
        }
    }
}

#[test]
fn o_build_start_before_builder() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder.clone(), &clock));
    let lines = capture(&mut parts);
    let seen = lines.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    parts.builder = Arc::new(move || {
        tx.send(
            seen.lock()
                .unwrap()
                .iter()
                .any(|s| value(s, "event") == "build_start"),
        )
        .unwrap();
        builder()
    });
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    lines.done(1);
    assert!(
        rx.recv_timeout(HANG).unwrap(),
        "build_start must be written before builder entry"
    );
}

#[test]
fn o_reordered_callbacks() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let lines = capture(&mut parts);
    let seen = lines.clone();
    let gate = Gate::closed();
    let held = gate.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    parts.hooks.lifecycle = Some(Arc::new(move |s| {
        if value(s, "event") == "callback_first" {
            tx.send(()).unwrap();
            assert!(held.pass());
        }
        seen.push(s);
    }));
    let state = DaemonState::from_parts(parts).unwrap();
    let other = state.clone();
    let worker = std::thread::spawn(move || {
        other.lifecycle_event(other.build_boundary(
            1,
            other.build_stamp(other.now()),
            "callback_first",
        ));
    });
    rx.recv_timeout(HANG).unwrap();
    state.lifecycle_event(state.build_boundary(
        1,
        state.build_stamp(state.now()),
        "callback_second",
    ));
    gate.open();
    worker.join().unwrap();
    let mut logs = lines.lock().unwrap().clone();
    assert_eq!(value(&logs[0], "event"), "callback_second");
    assert!(number(&logs[0], "sequence") > number(&logs[1], "sequence"));
    logs.sort_by_key(|s| number(s, "sequence"));
    assert_eq!(value(&logs[0], "event"), "callback_first");
    assert!(number(&logs[0], "sequence") < number(&logs[1], "sequence"));
}
