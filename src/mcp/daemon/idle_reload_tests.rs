use super::coordinator::DaemonState;
use super::idle_tests::*;
use super::test_support::*;
use super::wire_support::*;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{RefreshPolicy, SessionProvider};
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[test]
fn concurrent_idle_reload_single_build() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(
        &cfg,
        vec![
            Box::new({
                let cfg = cfg.clone();
                move || SessionProvider::bootstrap(&cfg)
            }),
            build_after(&gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    let (mut old, old_worker) = connect(&state, client_settings(600));
    old.handshake();
    assert_eq!(old.request(CALLEES)["id"], 7);
    unload(&clock, &state);
    disposed(&state);
    let (mut new, new_worker) = connect(&state, client_settings(0));
    new.handshake();
    let new_call = CALLEES.replace("\"id\":7", "\"id\":91");
    let warming = new.request(&new_call);
    assert_eq!(warming["id"], 91);
    assert_eq!(warming["result"]["structuredContent"]["status"], "warming");
    old.send(CALLEES);
    wait_until("old client joins reload", || status(&state).waiting == 1);
    // A disconnect cannot cancel the shared reload.
    close(new, new_worker);
    // The reload's builder runs on the build thread: wait for it, don't race it.
    let reload_started = || builds.load(Ordering::SeqCst) == 2;
    wait_until("reload build started", reload_started);
    gate.open();
    let ready = old.recv();
    assert_eq!(ready["id"], 7);
    assert_eq!(ready["result"]["isError"], false);
    let (mut newest, newest_worker) = connect(&state, client_settings(0));
    newest.handshake();
    let ready = newest.request(&new_call);
    assert_eq!(ready["id"], 91);
    assert_eq!(ready["result"]["isError"], false);
    assert_eq!(
        builds.load(Ordering::SeqCst),
        2,
        "one shared reload across old/new clients"
    );
    close(old, old_worker);
    close(newest, newest_worker);
}

#[test]
fn idle_warming_retry_success() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(
        &cfg,
        vec![
            Box::new({
                let cfg = cfg.clone();
                move || SessionProvider::bootstrap(&cfg)
            }),
            build_after(&gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    build(&state);
    unload(&clock, &state);
    disposed(&state);
    let (mut client, worker) = connect(&state, client_settings(0));
    client.handshake();
    let warming = client.request(CALLEES);
    assert_eq!(warming["result"]["structuredContent"]["status"], "warming");
    gate.open();
    wait_until("reload ready", || status(&state).state == BuildState::Ready);
    let ready = client.request(CALLEES);
    assert_eq!(ready["result"]["isError"], false);
    assert_eq!(ready["id"], 7);
    assert_eq!(builds.load(Ordering::SeqCst), 2);
    close(client, worker);
}

#[test]
fn daemon_idle_old_failure_superseded() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![fail_after(&gate, "obsolete failure")]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    let old = Conn::open(&state, 0);
    old.call();
    assert!(matches!(old.outcome(), Outcome::Warming { .. }));
    gate.open();
    wait_until("first failure", || {
        status(&state).state == BuildState::Failed
    });
    build(&state);
    unload(&clock, &state);
    disposed(&state);
    old.call();
    let outcome = old.outcome();
    assert!(
        !matches!(outcome, Outcome::Failed(_)),
        "success then unload supersedes older failure"
    );
    assert_eq!(
        status(&state).attempt,
        3,
        "old failure must not bypass reload admission"
    );
    wait_until("third attempt", || {
        status(&state).state == BuildState::Ready
    });
    old.call();
    assert_eq!(old.outcome(), Outcome::Ready);
    assert_eq!(builds.load(Ordering::SeqCst), 3);
}

#[test]
fn daemon_idle_history_ready_empty() {
    let published = Ok(());
    assert_eq!(
        super::control::answer_for(&published, &super::control::BuildPhase::Idle),
        super::control::Answer::Superseded,
        "historical success must not authorize Ready over an empty slot"
    );
}

#[test]
fn daemon_idle_reload_rechecks_freshness() {
    for policy in [
        RefreshPolicy::WarnOnly,
        RefreshPolicy::AutoFull,
        RefreshPolicy::AutoIncremental,
    ] {
        let (dir, cfg) = fixture(policy);
        let race_cfg = cfg.clone();
        let path = dir.path().join("util.py");
        let edit = path.clone();
        let (builder, builds) = scripted(
            &cfg,
            vec![
                Box::new({
                    let cfg = cfg.clone();
                    move || SessionProvider::bootstrap(&cfg)
                }),
                Box::new(move || {
                    SessionProvider::bootstrap_observed(&race_cfg, &mut |stage| {
                        if stage == crate::mcp::session::BuildStage::RepoLoaded {
                            std::fs::write(&edit, "def replacement():\n    return 2\n").unwrap();
                        }
                    })
                }),
            ],
        );
        let clock = ManualClock::new();
        let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
        build(&state);
        let before = state.read(|p| p.idle_publication()).unwrap().unwrap();
        unload(&clock, &state);
        disposed(&state);
        // This edit must happen even when a mutant skips the bootstrap entirely.
        std::fs::write(&path, "def before_reload():\n    return 3\n").unwrap();
        build(&state);
        let (source, publication) = state
            .read(|p| {
                (
                    p.session().repo.files["util.py"].source.clone(),
                    p.idle_publication(),
                )
            })
            .unwrap()
            .unwrap();
        assert!(
            source.contains("before_reload"),
            "reload must read the independent pre-bootstrap edit, not republish old source"
        );
        assert!(
            publication.diverged,
            "read-A/edit-B/stamp-B on reload must retain divergence"
        );
        assert_eq!(
            publication.generation,
            before.generation + 1,
            "changed loaded fingerprint advances G even when validation diverges"
        );
        let (mut client, worker) = connect(&state, client_settings(600));
        client.handshake();
        let response = client.request(r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"util.py","line":1}}}"#);
        let meta = &response["result"]["_meta"];
        if policy == RefreshPolicy::WarnOnly {
            assert_eq!(meta["prism/index_freshness"], "stale", "{response}");
            assert!(
                response.to_string().contains("before_reload"),
                "captured graph must match captured source"
            );
        } else {
            assert_eq!(meta["prism/auto_refresh"], "refreshed", "{response}");
            assert!(
                response.to_string().contains("replacement"),
                "refreshed graph must match refreshed source"
            );
            assert!(state
                .read(|p| p.session().repo.files["util.py"]
                    .source
                    .contains("replacement"))
                .unwrap()
                .unwrap());
        }
        close(client, worker);
        unload(&clock, &state);
        disposed(&state);
        build(&state);
        assert!(
            !state
                .read(|p| p.idle_publication().diverged)
                .unwrap()
                .unwrap(),
            "fresh bootstrap clears old diagnostic"
        );
        assert_eq!(builds.load(Ordering::SeqCst), 3);
        let _ = Arc::strong_count(&state);
    }
}

#[test]
fn daemon_idle_disposal_budgets() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let build_gate = Gate::closed();
    let (builder, builds) = scripted(
        &cfg,
        vec![
            Box::new({
                let cfg = cfg.clone();
                move || SessionProvider::bootstrap(&cfg)
            }),
            build_after(&build_gate, &cfg),
        ],
    );
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let (hook, reached, dispose_gate) = barrier();
    parts.hooks.dispose = Some(hook);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    unload(&clock, &state);
    reached.recv_timeout(HANG).unwrap();
    let a = Conn::open(&state, 4);
    a.call();
    wait_until("first disposal waiter", || status(&state).waiting == 1);
    advance(&clock, &state, 4);
    assert_eq!(
        a.outcome(),
        Outcome::Warming {
            elapsed: std::time::Duration::ZERO
        }
    );
    assert_eq!(
        builds.load(Ordering::SeqCst),
        1,
        "disposal timeout starts no overlapping build"
    );
    let b = Conn::open(&state, 6);
    b.call();
    wait_until("fresh disposal budget", || status(&state).waiting == 1);
    advance(&clock, &state, 2);
    assert!(
        !b.answered(),
        "one call's timeout must not consume the next call's allowance"
    );
    dispose_gate.open();
    disposed(&state);
    wait_until("reload starts after disposal", || {
        builds.load(Ordering::SeqCst) == 2 && status(&state).waiting == 1
    });
    advance(&clock, &state, 4);
    assert_eq!(
        b.outcome(),
        Outcome::Warming {
            elapsed: std::time::Duration::from_secs(4)
        },
        "same call gets one fixed disposal+build budget, and reports post-wait build age"
    );
    a.call();
    wait_until("retry gets independent build budget", || {
        status(&state).waiting == 1
    });
    advance(&clock, &state, 3);
    assert!(!a.answered());
    build_gate.open();
    assert_eq!(a.outcome(), Outcome::Ready);
}

#[test]
fn daemon_idle_reload_failure_recovery() {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    build(&state);
    unload(&clock, &state);
    disposed(&state);
    std::fs::remove_dir_all(dir.path()).unwrap();
    let client = Conn::open(&state, 600);
    client.call();
    assert!(
        matches!(client.outcome(), Outcome::Failed(_)),
        "missing checkout must fail reload"
    );
    std::fs::create_dir(dir.path()).unwrap();
    std::fs::write(
        dir.path().join("util.py"),
        "def recovered():\n    return 2\n",
    )
    .unwrap();
    client.call();
    assert_eq!(
        client.outcome(),
        Outcome::Ready,
        "heard reload failure permits recovery on the next call"
    );
    assert_eq!(builds.load(Ordering::SeqCst), 3);
    assert!(state
        .read(|p| p.session().repo.files["util.py"]
            .source
            .contains("recovered"))
        .unwrap()
        .unwrap());
}
