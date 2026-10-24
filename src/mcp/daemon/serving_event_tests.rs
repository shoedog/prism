//! Real shared-daemon production hooks, captured through existing test support.
use super::super::{
    coordinator::{DaemonState, Parts},
    idle_tests::{build, disposed, enabled, unload},
    test_support::*,
};
use crate::mcp::{CacheMode, RefreshPolicy, SessionProvider};
use crate::navigation::queries::{callees, nodes_at};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
type Lines = Arc<Mutex<Vec<String>>>;
fn capture(parts: &mut Parts) -> Lines {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let out = lines.clone();
    parts.hooks.lifecycle = Some(Arc::new(move |s| out.lock().unwrap().push(s.to_owned())));
    lines
}
fn line(lines: &Lines, event: &str) -> String {
    wait_until(event, || {
        lines
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.contains(&format!("event={event} ")))
    });
    lines
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|s| s.contains(&format!("event={event} ")))
        .unwrap()
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
    unload(&clock, &state);
    disposed(&state);
    build(&state);
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
    assert!(
        number(&line(&lines, "build_finish"), "mono_ns")
            <= number(&line(&lines, "publication"), "mono_ns")
    );
}
#[test]
fn o_event_identity_and_clock() {
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
    unload(&clock, &state);
    disposed(&state);
    let logs = lines.lock().unwrap();
    let mut seq = 0;
    for s in logs.iter() {
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
        seen.lock().unwrap().push(s.to_owned());
    }));
    let state = DaemonState::from_parts(parts).unwrap();
    *owner.lock().unwrap() = Arc::downgrade(&state);
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    let before = client.request(CALLEES);
    client.request(REFRESH);
    let after = client.request(CALLEES);
    close(client, worker);
    assert_eq!(before["result"]["isError"], after["result"]["isError"]);
    assert_eq!(value(&line(&lines, "refresh"), "outcome"), "complete");
    unload(&clock, &state);
    disposed(&state);
}
