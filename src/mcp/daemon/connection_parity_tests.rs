//! Tests that a daemon connection is the in-process server for one client (spec §5
//! "Preserved"), that the stale evidence a refresh recorded reaches every client (spec §9
//! test 14), and that a broken daemon ends a connection with an error and not a panic.
use super::super::coordinator::DaemonState;
use super::super::test_support::{client_settings, fixture, scripted, status};
use super::super::wire_support::{
    close, connect, ended, Client, CALLEES, CALLERS, INIT, INITED, LIST, REFRESH,
};
use crate::mcp::framing::UnixStreamTransport;
use crate::mcp::freshness::FreshnessReport;
use crate::mcp::lazy::{LazySessionProvider, SessionBuilder};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::transport::serve_connection;
use crate::mcp::{RefreshPolicy, RefreshVerification, SessionProvider, StartupMode};
use serde_json::json;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const RUN_CALLEES: &str = "callees:run@main.py";

enum Step {
    /// Send this line and read exactly one response line.
    Request(&'static str),
    /// Send this line; nothing answers it.
    Notify(&'static str),
    /// Overwrite a fixture file, so the index becomes stale.
    Edit {
        file: &'static str,
        contents: &'static str,
    },
}

/// Garbage, a call before the handshake, the handshake, listing, a query (the first call that
/// needs the index, so it starts the build and waits for it), every kind of refused call, an
/// edit that makes the index stale, a query on the stale index, a refresh, a query again, an
/// unknown method and a request with no id.
fn session() -> Vec<Step> {
    vec![
        Step::Request("this is not json"),
        Step::Request("[]"),
        Step::Request(r#"{"jsonrpc":"2.0","id":90,"method":"tools/list"}"#),
        Step::Request(INIT),
        Step::Notify(INITED),
        Step::Request(r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#),
        Step::Request(LIST),
        Step::Request(CALLEES),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#,
        ),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"nav_callees","arguments":[]}}"#,
        ),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"bogus"}}}}"#,
        ),
        Step::Request(
            r#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"refresh_index","arguments":{"x":1}}}"#,
        ),
        Step::Edit {
            file: "util.py",
            contents: "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
        },
        Step::Request(CALLERS),
        Step::Request(REFRESH),
        Step::Request(CALLERS),
        Step::Request(r#"{"jsonrpc":"2.0","id":21,"method":"no/such/method"}"#),
        Step::Request(r#"{"jsonrpc":"2.0","method":"tools/list"}"#),
    ]
}

/// Play `steps` as one client and return every response line, in order.
fn play(client: &mut Client, root: &Path, steps: &[Step]) -> String {
    let mut transcript = String::new();
    for step in steps {
        match step {
            Step::Request(line) => {
                client.send(line);
                transcript.push_str(&client.recv_line());
            }
            Step::Notify(line) => client.send(line),
            Step::Edit { file, contents } => {
                std::fs::write(root.join(*file), *contents).expect("edit a fixture file");
            }
        }
    }
    transcript
}

/// Spec §5: for the same snapshot, the same connection settings and the same readiness state,
/// a daemon answers what the in-process server answers. Both sides here are lazy, wait up to
/// 60 s for the build their first query starts, and serve a repository with the same content.
/// Rules out any difference in how a daemon connection composes control, readiness and
/// dispatch, down to one byte of one response.
#[test]
fn a_daemon_connection_answers_a_whole_session_byte_for_byte_like_the_in_process_server() {
    for policy in [RefreshPolicy::WarnOnly, RefreshPolicy::AutoFull] {
        let settings = client_settings(60);
        let steps = session();
        let requests = steps
            .iter()
            .filter(|step| matches!(step, Step::Request(_)))
            .count();

        // In process: the lazy provider behind `serve_connection`, over a socket pair.
        let (in_process_dir, in_process_cfg) = fixture(policy);
        let (near, far) = UnixStream::pair().expect("a connected socket pair");
        let output = *settings.output();
        let in_process = std::thread::spawn(move || {
            let builder_cfg = in_process_cfg.clone();
            let builder: SessionBuilder =
                Arc::new(move || SessionProvider::bootstrap(&builder_cfg));
            let mut provider = LazySessionProvider::with_builder(
                &in_process_cfg,
                Duration::from_secs(60),
                builder,
            )
            .expect("a lazy provider");
            let mut transport = UnixStreamTransport::from_stream(far).expect("a transport");
            serve_connection(
                &mut provider,
                &ToolRegistry::all_v1(),
                &mut transport,
                &output,
            )
        });
        let mut client = Client::new(near);
        let expected = play(&mut client, in_process_dir.path(), &steps);
        drop(client);
        ended(in_process).expect("the in-process server ends cleanly at end of input");

        // The daemon: the same session on a connection served through the coordinator.
        let (daemon_dir, daemon_cfg) = fixture(policy);
        let (builder, builds) = scripted(&daemon_cfg, vec![]);
        let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
        let (mut client, worker) = connect(&state, settings);
        let actual = play(&mut client, daemon_dir.path(), &steps);
        close(client, worker);

        assert_eq!(
            expected.lines().count(),
            requests,
            "policy {policy:?}: one response per request"
        );
        for (index, (want, got)) in expected.lines().zip(actual.lines()).enumerate() {
            assert_eq!(
                got,
                want,
                "policy {policy:?}: response {} differs",
                index + 1
            );
        }
        assert_eq!(actual.len(), expected.len(), "policy {policy:?}");
        assert_eq!(
            builds.load(Ordering::SeqCst),
            1,
            "policy {policy:?}: one build for the whole session"
        );
        // Warn-only: the explicit refresh. Automatic: the stale query refreshed first.
        let refreshes = if policy == RefreshPolicy::WarnOnly {
            1
        } else {
            2
        };
        assert_eq!(status(&state).generation, refreshes, "policy {policy:?}");
    }
}

fn force_the_next_refresh_to_verify_diverged(state: &DaemonState) {
    state
        .write(|provider| {
            provider.force_next_verification_for_tests(RefreshVerification::Diverged(
                FreshnessReport::from_changed_paths(["main.py".to_string()]),
            ));
        })
        .expect("the provider lock is not poisoned")
        .expect("an index is published");
}

/// Spec §9 test 14, first half: a refresh that verifies `Diverged`, followed by a clean stamp
/// probe; B's next query still carries the stale warning. Rules out a read view narrower than
/// the whole provider (the review finding N2 of the spec: a view of the session and the
/// freshness probe alone drops the evidence).
#[test]
fn sticky_stale_evidence_reaches_another_client_although_the_stamp_probe_is_clean() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    // A's refresh verifies Diverged: the tree moved while the snapshot was being built.
    force_the_next_refresh_to_verify_diverged(&state);
    let refreshed = a.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "raced_stale",
        "{refreshed}"
    );
    assert_eq!(refreshed["result"]["structuredContent"]["generation"], 1);
    assert_eq!(status(&state).generation, 1);

    // The stamp probe is clean: nothing on disk differs from what the new snapshot recorded.
    assert_eq!(
        state.read(|provider| provider.freshness().check().stale),
        Ok(Some(false))
    );

    // B refreshed nothing and asked for nothing yet. Its next query carries the warning, and
    // the warning is the refresh's evidence (a clean probe has no path to name).
    let from_b = b.request(CALLEES);
    assert_eq!(from_b["result"]["isError"], false, "{from_b}");
    assert_eq!(from_b["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(
        from_b["result"]["_meta"]["prism/index_freshness"], "stale",
        "{from_b}"
    );
    assert_eq!(
        from_b["result"]["_meta"]["prism/stale_index_paths"],
        json!(["main.py"]),
        "{from_b}"
    );

    // It stays until a refresh verifies clean, for every client.
    let again = a.request(CALLEES);
    assert_eq!(again["result"]["_meta"]["prism/index_freshness"], "stale");

    close(a, a_worker);
    close(b, b_worker);
}

/// Spec §9 test 14, second half: "and auto-refresh retries". Under an automatic policy the
/// sticky evidence is what makes B's query refresh, although the stamp probe is clean.
#[test]
fn under_an_automatic_policy_sticky_stale_evidence_makes_the_next_client_s_query_refresh_again() {
    let (_dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");
    assert!(
        first["result"]["_meta"].get("prism/auto_refresh").is_none(),
        "a fresh index is not refreshed: {first}"
    );

    force_the_next_refresh_to_verify_diverged(&state);
    let refreshed = a.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "raced_stale",
        "{refreshed}"
    );
    assert_eq!(status(&state).generation, 1);
    assert_eq!(
        state.read(|provider| provider.freshness().check().stale),
        Ok(Some(false))
    );

    let from_b = b.request(CALLEES);
    assert_eq!(from_b["result"]["isError"], false, "{from_b}");
    assert_eq!(from_b["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(
        from_b["result"]["_meta"]["prism/auto_refresh"], "refreshed",
        "{from_b}"
    );
    assert_eq!(from_b["result"]["_meta"]["prism/refresh_generation"], 2);
    assert_eq!(status(&state).generation, 2);

    // That refresh verified clean, so the evidence is gone for every client.
    let again = a.request(CALLEES);
    assert!(
        again["result"]["_meta"].get("prism/auto_refresh").is_none(),
        "{again}"
    );
    assert!(
        again["result"]["_meta"]
            .get("prism/index_freshness")
            .is_none(),
        "{again}"
    );
    assert_eq!(status(&state).generation, 2);

    close(a, a_worker);
    close(b, b_worker);
}

/// The coordinator never announces `Ready` over an empty slot. If it ever did, a client's call
/// must not panic the daemon. Rules out `expect` on the slot.
#[test]
fn ready_with_nothing_published_ends_the_connection_with_an_error_not_a_panic() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");
    assert_eq!(status(&state).state, BuildState::Ready);

    state.unpublish_for_tests();
    a.send(CALLEES);
    a.expect_closed();
    let error = ended(a_worker).expect_err("the connection ends with an error");
    assert!(
        error.to_string().contains("no index is published"),
        "{error}"
    );
}

/// In S3a a panic under the write lock poisons the slot (slice S3b makes it fatal). Until
/// then a client's call must end its connection with an error, and the control path, which
/// takes no lock, must keep answering.
#[test]
fn a_poisoned_provider_slot_ends_the_connection_with_an_error_and_handshakes_go_on() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    let poisoner = Arc::clone(&state);
    let panicked = std::thread::spawn(move || {
        let _ = poisoner.write(|_provider| -> () {
            panic!("poisoning the provider slot for a test");
        });
    })
    .join();
    assert!(panicked.is_err());

    a.send(CALLEES);
    a.expect_closed();
    let error = ended(a_worker).expect_err("the connection ends with an error");
    assert!(
        error.to_string().contains("provider lock is poisoned"),
        "{error}"
    );

    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    let listed = b.request(LIST);
    assert_eq!(listed["result"]["tools"].as_array().map(Vec::len), Some(9));
    close(b, b_worker);
}
