use super::coordinator::DaemonState;
use super::idle_tests::*;
use super::test_support::*;
use super::wire_support::*;
use crate::mcp::framing::{ReadOutcome, Transport};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::RefreshPolicy;
use serde_json::Value;
use std::sync::{mpsc, Arc};

fn pinned_at(response: bool) {
    let (_dir, cfg) = fixture(if response {
        RefreshPolicy::AutoFull
    } else {
        RefreshPolicy::WarnOnly
    });
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let (hook, reached, gate) = barrier();
    if response {
        parts.hooks.response = Some(hook);
    } else {
        parts.hooks.dispatch = Some(hook);
    }
    let state = DaemonState::from_parts(parts).unwrap();
    if response {
        build(&state);
        std::fs::write(
            cfg.repo_root.join("util.py"),
            "def helper():\n    return 22\n",
        )
        .unwrap();
    }
    let (mut a, worker) = connect(&state, client_settings(600));
    let (mut silent, silent_worker) = connect(&state, client_settings(600));
    silent.handshake();
    a.handshake();
    a.send(CALLEES);
    reached.recv_timeout(HANG).unwrap();
    let (other_builder, _) = scripted(&cfg, vec![]);
    let independent = DaemonState::from_parts(enabled(parts_with(other_builder, &clock))).unwrap();
    build(&independent);
    advance(&clock, &state, N * 2);
    state.unload_tick().unwrap();
    independent.unload_tick().unwrap();
    assert!(
        !resident(&independent),
        "a busy provider must not pin an independent provider"
    );
    disposed(&independent);
    let counts = state.activity_counts();
    let pinned = resident(&state);
    gate.open();
    assert!(
        pinned,
        "request must pin provider until its owned response is built"
    );
    assert_eq!(
        counts.0, 0,
        "pending admission must transfer to the serving lease"
    );
    assert_eq!(
        counts.1, 1,
        "serving lease must cover readiness through owned response"
    );
    assert!(
        resident(&state),
        "request must pin provider across dispatch boundary"
    );
    assert_eq!(a.recv()["id"], 7);
    wait_until("last release", || {
        state.activity_counts().0 + state.activity_counts().1 == 0
    });
    advance(&clock, &state, N - 1);
    state.unload_tick().unwrap();
    assert!(
        resident(&state),
        "idle origin must be the last release, not acquisition"
    );
    advance(&clock, &state, 1);
    state.unload_tick().unwrap();
    assert!(
        !resident(&state),
        "released request must permit idle unload"
    );
    disposed(&state);
    close(a, worker);
    close(silent, silent_worker);
}
#[test]
fn daemon_idle_never_interrupts_request() {
    pinned_at(false);
}
#[test]
fn daemon_idle_owned_response_lease() {
    pinned_at(true);
}

/// The frame writer blocks with a completely owned tool response. No slot lock or lease
/// may span this call. The same serve_bounded → serve_client route serves Unix sockets.
struct BlockWriter {
    input: std::collections::VecDeque<String>,
    blocked: mpsc::Sender<()>,
    gate: Arc<Gate>,
}
impl Transport for BlockWriter {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        Ok(self.input.pop_front().map(ReadOutcome::Message))
    }
    fn write_message(&mut self, value: Value) -> anyhow::Result<()> {
        if value["id"] == 7 {
            self.blocked.send(()).unwrap();
            assert!(self.gate.pass());
        }
        Ok(())
    }
}
#[test]
fn daemon_idle_blocked_writer_releases() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
    let (tx, rx) = mpsc::channel();
    let gate = Gate::closed();
    let held = gate.clone();
    let serving = state.clone();
    let worker = std::thread::spawn(move || {
        let mut output = BlockWriter {
            input: [INIT, INITED, CALLEES].map(str::to_string).into(),
            blocked: tx,
            gate: held,
        };
        super::connection::serve_bounded(
            &serving,
            &ToolRegistry::all_v1(),
            &mut output,
            &client_settings(600),
        )
    });
    rx.recv_timeout(HANG).unwrap();
    let counts = state.activity_counts();
    advance(&clock, &state, N);
    state.unload_tick().unwrap();
    let detached = !resident(&state);
    gate.open();
    worker.join().unwrap().unwrap();
    assert_eq!(
        (counts.0, counts.1),
        (0, 0),
        "socket write must not retain a request lease"
    );
    assert!(detached, "blocked output must not prevent idle unload");
    disposed(&state);
}

#[test]
fn daemon_idle_refresh_shutdown_races() {
    let (_dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let (hook, reached, gate) = barrier();
    parts.hooks.exclusive = Some(hook);
    let state = DaemonState::from_parts(parts).unwrap();
    build(&state);
    std::fs::write(
        cfg.repo_root.join("util.py"),
        "def helper():\n    return 22\n",
    )
    .unwrap();
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    client.send(CALLEES);
    reached.recv_timeout(HANG).unwrap();
    advance(&clock, &state, N * 2);
    state.unload_tick().unwrap();
    assert!(
        resident(&state),
        "auto-refresh read/write gap must retain serving lease"
    );
    gate.open();
    assert_eq!(client.recv()["id"], 7);
    unload(&clock, &state);
    disposed(&state);
    state
        .drain(super::control::StopReason::Signal, || {})
        .unwrap();
    let answer = client.request(REFRESH);
    assert!(
        answer.to_string().contains("drain"),
        "draining daemon must reject reload: {answer}"
    );
    assert!(
        !resident(&state),
        "shutdown must never resurrect an unloaded provider"
    );
    close(client, worker);
}

#[test]
fn daemon_idle_explicit_refresh_and_rollback() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    let (hook, reached, gate) = barrier();
    parts.hooks.dispatch = Some(hook);
    let state = DaemonState::from_parts(parts).unwrap();
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    client.send(REFRESH);
    reached.recv_timeout(HANG).unwrap();
    advance(&clock, &state, N * 2);
    state.unload_tick().unwrap();
    assert_eq!(state.activity_counts().1, 1);
    assert!(resident(&state));
    gate.open();
    assert_eq!(client.recv()["id"], 8);
    let before = state.read(|p| p.idle_publication()).unwrap().unwrap();
    state
        .write(|p| p.force_next_refresh_error_for_tests("rollback witness"))
        .unwrap();
    let failed = client.request(REFRESH);
    assert!(failed.to_string().contains("rollback witness"));
    assert_eq!(
        state.read(|p| p.idle_publication()).unwrap().unwrap(),
        before,
        "resident refresh failure must preserve the snapshot"
    );
    assert_eq!(
        (state.activity_counts().0, state.activity_counts().1),
        (0, 0)
    );
    unload(&clock, &state);
    disposed(&state);
    assert_eq!(client.request(REFRESH)["id"], 8);
    assert_eq!(
        builds.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "unloaded refresh bootstraps before FullOnly refresh"
    );
    assert_eq!(
        state.read(|p| p.generation()).unwrap().unwrap(),
        before.generation + 1,
        "unloaded refresh also performs the explicit refresh build"
    );
    close(client, worker);
}

#[test]
fn daemon_idle_unwind_releases() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = enabled(parts_with(builder, &clock));
    parts.hooks.returned = Some(Arc::new(|| panic!("owned response construction panic")));
    let state = DaemonState::from_parts(parts).unwrap();
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    client.send(CALLEES);
    client.expect_closed();
    assert!(ended(worker).is_err());
    assert_eq!(
        (state.activity_counts().0, state.activity_counts().1),
        (0, 0),
        "connection unwind must release the serving lease"
    );
    unload(&clock, &state);
    disposed(&state);
}

#[test]
fn daemon_idle_reload_after_stamp() {
    use crate::mcp::SessionProvider;
    for policy in [
        RefreshPolicy::WarnOnly,
        RefreshPolicy::AutoFull,
        RefreshPolicy::AutoIncremental,
    ] {
        let (dir, cfg) = fixture(policy);
        std::fs::remove_file(dir.path().join("main.py")).unwrap();
        let edit = dir.path().join("util.py");
        let reload_cfg = cfg.clone();
        let (builder, _) = scripted(
            &cfg,
            vec![
                Box::new({
                    let cfg = cfg.clone();
                    move || SessionProvider::bootstrap(&cfg)
                }),
                Box::new(move || {
                    crate::mcp::freshness::capture_tests::on("source-stamped", {
                        let edit = edit.clone();
                        move || {
                            std::fs::write(edit, "def newone():\n    return 2\n").unwrap();
                        }
                    });
                    SessionProvider::bootstrap(&reload_cfg)
                }),
            ],
        );
        let clock = ManualClock::new();
        let state = DaemonState::from_parts(enabled(parts_with(builder, &clock))).unwrap();
        build(&state);
        unload(&clock, &state);
        disposed(&state);
        build(&state);
        assert!(
            state
                .read(|p| p.idle_publication().diverged)
                .unwrap()
                .unwrap(),
            "post-stamp edit must remain divergent on reload"
        );
        let (mut client, worker) = connect(&state, client_settings(600));
        client.handshake();
        let response = client.request(r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"util.py","line":1}}}"#);
        let meta = &response["result"]["_meta"];
        if policy == RefreshPolicy::WarnOnly {
            assert_eq!(meta["prism/index_freshness"], "stale");
        } else {
            assert_eq!(meta["prism/auto_refresh"], "refreshed");
            assert!(response.to_string().contains("newone"));
        }
        close(client, worker);
    }
}
