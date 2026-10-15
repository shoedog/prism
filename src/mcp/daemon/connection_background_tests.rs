//! A warm daemon (`--warm-at-startup`, `StartupMode::Background`) whose startup build fails
//! before a client's first call: S3a's parity requirement for S4 (the second review of S3a-2,
//! Opus F1). The in-process server tells that call `build_failed` with the cause and retries
//! on the next call; so must the daemon.
use super::super::coordinator::DaemonState;
use super::super::test_support::{
    build_after, client_settings, fail_after, fixture, scripted, status, wait_until, Gate,
};
use super::super::wire_support::{close, connect, ended, Client, CALLEES, INIT, INITED};
use crate::mcp::framing::UnixStreamTransport;
use crate::mcp::lazy::{LazySessionProvider, LazyTestHooks};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::transport::serve_connection;
use crate::mcp::{RefreshPolicy, StartupMode};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// The reviewer's probe, which failed on S3a's code: both sides' startup builds fail before the
/// client's first call; the client then sends two calls. The daemon's bytes must be the
/// in-process server's: `build_failed` with the cause, then the retry's `warming`. Rules out a
/// first call that starts the retry and is answered `warming`, the failure never reported.
#[test]
fn a_warm_daemon_whose_startup_build_failed_before_the_first_call_reports_it_like_the_in_process_server(
) {
    let settings = client_settings(0);

    let (_in_dir, mut in_cfg) = fixture(RefreshPolicy::WarnOnly);
    in_cfg.startup = StartupMode::Background;
    let in_gate = Gate::closed();
    let in_retry = Gate::closed();
    let (in_builder, _in_builds) = scripted(
        &in_cfg,
        vec![
            fail_after(&in_gate, "the repository could not be loaded"),
            build_after(&in_retry, &in_cfg),
        ],
    );
    let delivered = Arc::new(AtomicU64::new(0));
    let hooks = LazyTestHooks {
        published: Some({
            let delivered = Arc::clone(&delivered);
            Arc::new(move || {
                delivered.fetch_add(1, Ordering::SeqCst);
            })
        }),
        before_wait: None,
    };
    let (near, far) = UnixStream::pair().expect("a connected socket pair");
    let output = *settings.output();
    let gate2 = Arc::clone(&in_gate);
    let delivered2 = Arc::clone(&delivered);
    let in_process = std::thread::spawn(move || {
        let mut provider =
            LazySessionProvider::with_builder_and_hooks(&in_cfg, Duration::ZERO, in_builder, hooks)
                .expect("a lazy provider");
        gate2.open();
        wait_until("the startup build to fail", || {
            delivered2.load(Ordering::SeqCst) == 1
        });
        let mut transport = UnixStreamTransport::from_stream(far).expect("a transport");
        serve_connection(
            &mut provider,
            &ToolRegistry::all_v1(),
            &mut transport,
            &output,
        )
    });
    let mut client = Client::new(near);
    let mut expected = String::new();
    client.send(INIT);
    expected.push_str(&client.recv_line());
    client.send(INITED);
    client.send(CALLEES);
    let failed = client.recv_line();
    assert!(failed.contains("build_failed"), "{failed}");
    expected.push_str(&failed);
    client.send(CALLEES);
    let retried = client.recv_line();
    assert!(retried.contains("warming"), "{retried}");
    expected.push_str(&retried);
    drop(client);
    in_retry.open();
    ended(in_process).expect("the in-process server ends cleanly");

    let (_d_dir, d_cfg) = fixture(RefreshPolicy::WarnOnly);
    let d_gate = Gate::closed();
    let d_retry = Gate::closed();
    let (d_builder, _d_builds) = scripted(
        &d_cfg,
        vec![
            fail_after(&d_gate, "the repository could not be loaded"),
            build_after(&d_retry, &d_cfg),
        ],
    );
    let state = DaemonState::new(d_builder, StartupMode::Background).expect("a warm daemon");
    d_gate.open();
    wait_until("the startup build to fail", || {
        status(&state).state == BuildState::Failed
    });
    let (mut client, worker) = connect(&state, settings);
    let mut actual = String::new();
    client.send(INIT);
    actual.push_str(&client.recv_line());
    client.send(INITED);
    client.send(CALLEES);
    actual.push_str(&client.recv_line());
    client.send(CALLEES);
    actual.push_str(&client.recv_line());
    close(client, worker);
    d_retry.open();
    assert_eq!(actual, expected);
}
