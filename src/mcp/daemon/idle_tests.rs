//! Lifecycle witnesses shared with the connection, reload and server suites.
use super::clock::Clock;
use super::coordinator::{DaemonState, Parts};
use super::test_support::*;
use crate::mcp::{RefreshPolicy, StartupMode};
use std::sync::{mpsc, Arc};
use std::time::Duration;

pub(super) const N: u64 = 10;
pub(super) fn enabled(mut parts: Parts) -> Parts {
    parts.idle_timeout = Some(Duration::from_secs(N));
    parts
}
pub(super) fn resident(state: &DaemonState) -> bool {
    state.read(|_| ()).unwrap().is_some()
}
pub(super) fn build(state: &Arc<DaemonState>) {
    let conn = Conn::open(state, 600);
    conn.call();
    assert_eq!(conn.outcome(), Outcome::Ready);
}
pub(super) fn unload(clock: &ManualClock, state: &Arc<DaemonState>) {
    advance(clock, state, N);
    state.unload_tick().unwrap();
    assert!(
        !resident(state),
        "idle deadline must detach the whole provider"
    );
}
pub(super) fn disposed(state: &DaemonState) {
    wait_until("disposal completion", || !state.activity_counts().2);
}
type Barrier = (Arc<dyn Fn() + Send + Sync>, mpsc::Receiver<()>, Arc<Gate>);
pub(super) fn barrier() -> Barrier {
    let gate = Gate::closed();
    let held = gate.clone();
    let (tx, rx) = mpsc::channel();
    (
        Arc::new(move || {
            tx.send(()).unwrap();
            let _ = held.pass();
        }),
        rx,
        gate,
    )
}

#[test]
fn daemon_idle_releases_whole_session() {
    for initialize_edges in [false, true] {
        let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
        let (builder, _) = scripted(&cfg, vec![]);
        let clock = ManualClock::new();
        let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
        // Handshake/list/ping never build or establish tool activity.
        let (mut silent, worker) = super::wire_support::connect(&state, client_settings(600));
        silent.handshake();
        silent.request(super::wire_support::LIST);
        silent.request(super::wire_support::PING);
        advance(&clock, &state, 100);
        state.unload_tick().unwrap();
        assert_eq!(status(&state).attempt, 0);
        build(&state);
        if initialize_edges {
            silent.request(super::wire_support::CALLEES);
        }
        let (repo, index) = state
            .read(|p| {
                (
                    Arc::downgrade(&p.session().repo),
                    Arc::downgrade(&p.session().index),
                )
            })
            .unwrap()
            .unwrap();
        advance(&clock, &state, N - 1);
        state.unload_tick().unwrap();
        assert!(resident(&state), "publication + N is a lower bound");
        advance(&clock, &state, 1);
        state.unload_tick().unwrap();
        assert!(
            !resident(&state),
            "silent connected clients must not pin the provider"
        );
        disposed(&state);
        assert!(
            repo.upgrade().is_none(),
            "loaded repository must be released"
        );
        assert!(
            index.upgrade().is_none(),
            "index and lazy edges must be released"
        );
        assert_eq!(silent.request(super::wire_support::PING)["id"], 3);
        super::wire_support::close(silent, worker);
    }
}

#[test]
fn daemon_idle_production_off() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let parts = parts_with(builder, &clock);
    assert!(
        parts.idle_timeout.is_none(),
        "ordinary Parts construction defaults off"
    );
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    advance(&clock, &state, 86400);
    state.unload_tick().unwrap();
    assert!(resident(&state), "production construction must not unload");
}

#[test]
fn daemon_idle_publication_clock() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    parts.startup = StartupMode::Background;
    let state = DaemonState::from_parts(parts).unwrap();
    advance(&clock, &state, N * 4);
    state.unload_tick().unwrap();
    gate.open();
    wait_until("background publication", || {
        status(&state).generation == 0 && resident(&state)
    });
    wait_until("ready announcement", || {
        status(&state).state == crate::mcp::shared::preamble::BuildState::Ready
    });
    advance(&clock, &state, N - 1);
    state.unload_tick().unwrap();
    assert!(
        resident(&state),
        "build completion must start a fresh idle interval"
    );
    advance(&clock, &state, 1);
    state.unload_tick().unwrap();
    assert!(
        !resident(&state),
        "uncollected background completion must unload"
    );
    disposed(&state);
    let _ = clock.now();
}

#[test]
fn daemon_idle_warming_silence_unloads() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    let (mut client, worker) = super::wire_support::connect(&state, client_settings(0));
    client.handshake();
    assert_eq!(
        client.request(super::wire_support::CALLEES)["result"]["structuredContent"]["status"],
        "warming"
    );
    assert_eq!(
        (state.activity_counts().0, state.activity_counts().1),
        (0, 0)
    );
    advance(&clock, &state, N * 2);
    gate.open();
    wait_until("uncollected warming build", || {
        status(&state).state == crate::mcp::shared::preamble::BuildState::Ready
    });
    let repo = state
        .read(|p| Arc::downgrade(&p.session().repo))
        .unwrap()
        .unwrap();
    unload(&clock, &state);
    disposed(&state);
    assert!(
        repo.upgrade().is_none(),
        "uncollected completed attempt must not retain the loaded repository"
    );
    super::wire_support::close(client, worker);
}

#[test]
fn daemon_idle_reload_generation_and_cache() {
    use crate::mcp::{CacheMode, SessionProvider};
    let (dir, mut cfg) = fixture(RefreshPolicy::WarnOnly);
    let cache_dir = tempfile::tempdir().unwrap();
    cfg.cache = CacheMode::Dir(cache_dir.path().to_path_buf());
    std::fs::write(
        dir.path().join("a.ts"),
        "import {x} from '@/dep'; export const y = x;",
    )
    .unwrap();
    std::fs::write(dir.path().join("dep.ts"), "export const x = 1;").unwrap();
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    build(&state);
    let cache = state
        .read(|p| crate::navigation::cache::nav_cache_subdir(cache_dir.path(), &p.session().repo))
        .unwrap()
        .unwrap();
    for operation in [
        "same",
        "touch",
        "edit",
        "add",
        "delete",
        "manifest",
        "resolver",
        "absent-cache",
        "corrupt-cache",
    ] {
        let old = state.read(|p| p.idle_publication()).unwrap().unwrap();
        unload(&clock, &state);
        disposed(&state);
        match operation {
            "touch" => {
                let p = dir.path().join("util.py");
                let bytes = std::fs::read(&p).unwrap();
                std::fs::write(p, bytes).unwrap();
            }
            "edit" => std::fs::write(
                dir.path().join("util.py"),
                "def helper():\n    return 123\n",
            )
            .unwrap(),
            "add" => {
                std::fs::write(dir.path().join("new.py"), "def added():\n    return 7\n").unwrap()
            }
            "delete" => std::fs::remove_file(dir.path().join("new.py")).unwrap(),
            "manifest" => std::fs::write(
                dir.path().join("pyproject.toml"),
                "[project]\nname = 'idle-test'\nversion = '1'\n",
            )
            .unwrap(),
            "resolver" => std::fs::write(
                dir.path().join("tsconfig.json"),
                "{\"compilerOptions\":{\"baseUrl\":\".\",\"paths\":{\"@/*\":[\"*\"]}}}",
            )
            .unwrap(),
            "absent-cache" => {
                std::fs::remove_dir_all(&cache).unwrap();
            }
            "corrupt-cache" => corrupt(cache_dir.path()),
            _ => {}
        }
        build(&state);
        let expected = SessionProvider::bootstrap(&cfg).unwrap();
        let (current, hashes, names) = state
            .read(|p| {
                (
                    p.idle_publication(),
                    p.session().repo.file_hashes.clone(),
                    p.session().index.name_index.clone(),
                )
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            current.fingerprint,
            expected.idle_publication().fingerprint,
            "{operation}: fresh-reference snapshot parity"
        );
        assert_eq!(
            current.generation,
            old.generation + u64::from(old.fingerprint != current.fingerprint),
            "{operation}: same/changed fingerprint generation"
        );
        assert!(!current.diverged);
        assert_eq!(
            hashes,
            expected.session().repo.file_hashes,
            "{operation}: source parity"
        );
        assert_eq!(
            names,
            expected.session().index.name_index,
            "{operation}: graph parity"
        );
    }
    fn corrupt(root: &std::path::Path) {
        for entry in std::fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                corrupt(&entry.path());
            } else {
                std::fs::write(entry.path(), "invalid cache").unwrap();
            }
        }
    }
}

#[test]
fn daemon_idle_prior_divergence_same_generation() {
    use crate::mcp::SessionProvider;
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
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    build(&state);
    let old = state.read(|p| p.idle_publication()).unwrap().unwrap();
    assert!(old.diverged);
    std::fs::write(path, original).unwrap();
    unload(&clock, &state);
    disposed(&state);
    build(&state);
    let new = state.read(|p| p.idle_publication()).unwrap().unwrap();
    assert_eq!(new.fingerprint, old.fingerprint);
    assert_eq!(
        new.generation, old.generation,
        "prior sticky divergence with the same fingerprint keeps G"
    );
    assert!(
        !new.diverged,
        "bootstrap replaces prior sticky diagnostics with fresh evidence"
    );
}
