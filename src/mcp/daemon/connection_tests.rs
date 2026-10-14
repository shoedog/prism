//! Tests for one admitted connection served through the coordinator and the provider slot,
//! over a connected pair of Unix sockets with a raw MCP client. `two_clients_share_one_build`
//! is spec §9 test 1 as far as it can run before the accept loop and the shim exist; the
//! status probe it reads is `DaemonState::status`, called in process.
use super::super::coordinator::{AttemptHook, BuildStatus, DaemonState};
use super::super::test_support::{
    advance, build_after, client_settings, client_settings_with, fail_after, fail_now, fixture,
    parts_with, scripted, state_with, status, wait_until, Gate, ManualClock,
};
use super::super::wire_support::{
    close, connect, ended, CALLEES, INIT, INITED, LIST, NO_SUCH_TOOL, PING, REFRESH,
};
use crate::mcp::output::StructuredContentMode;
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{RefreshPolicy, StartupMode};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const RUN_CALLEES: &str = "callees:run@main.py";

fn built(builds: &AtomicU64) -> u64 {
    builds.load(Ordering::SeqCst)
}

fn status_of(state: BuildState, attempt: u64, waiting: u64) -> BuildStatus {
    BuildStatus {
        state,
        attempt,
        generation: 0,
        waiting,
    }
}

/// Spec §9 test 1: the builder is blocked on a barrier; the status probe shows `building`,
/// attempt 1; release; B's first call is served from that build. Also: each client's output
/// setting is its own. Rules out a build per client and settings read from the daemon's
/// environment.
#[test]
fn two_clients_share_one_build() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(
        &state,
        client_settings_with(600, StructuredContentMode::Always),
    );
    let (mut b, b_worker) = connect(
        &state,
        client_settings_with(600, StructuredContentMode::OmitDefaultPath),
    );

    a.handshake();
    b.handshake();
    assert_eq!(
        status(&state),
        status_of(BuildState::Idle, 0, 0),
        "two handshakes build nothing"
    );
    assert_eq!(built(&builds), 0);

    a.send(CALLEES);
    wait_until("A's call to be waiting on the build", || {
        status(&state).waiting == 1
    });
    b.send(CALLEES);
    wait_until("B's call to join it", || status(&state).waiting == 2);
    wait_until("the build thread to call the builder", || {
        built(&builds) == 1
    });
    assert_eq!(
        status(&state),
        status_of(BuildState::Building, 1, 2),
        "the status probe shows building, attempt 1"
    );

    gate.open();
    let from_a = a.recv();
    let from_b = b.recv();
    assert_eq!(from_a["id"], 7);
    assert_eq!(from_a["result"]["isError"], false, "{from_a}");
    assert_eq!(from_a["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(from_b["result"]["isError"], false, "{from_b}");
    assert!(
        from_b["result"].get("structuredContent").is_none(),
        "B asked for no structuredContent on the default path, and B's setting is B's: {from_b}"
    );
    assert_eq!(status(&state), status_of(BuildState::Ready, 1, 0));
    assert_eq!(
        built(&builds),
        1,
        "B was served from the build A's call started"
    );

    // A client that arrives after the build is served from it too.
    let (mut c, c_worker) = connect(&state, client_settings(600));
    c.handshake();
    let from_c = c.request(CALLEES);
    assert_eq!(from_c["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 1);

    close(a, a_worker);
    close(b, b_worker);
    close(c, c_worker);
}

/// Spec §4.1: a client's handshake must not wait on another client's build. Rules out asking
/// for readiness before the control path has had the message.
#[test]
fn the_handshake_listing_and_ping_are_answered_while_the_build_is_blocked() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    a.send(CALLEES);
    wait_until("A's call to be waiting on the build", || {
        status(&state).waiting == 1
    });

    // B arrives while the build is blocked. Nothing it does here needs the index.
    let (mut b, b_worker) = connect(&state, client_settings(600));
    let initialized = b.request(INIT);
    assert!(
        initialized["result"]["instructions"]
            .as_str()
            .unwrap_or_default()
            .contains("started by the first tool call"),
        "a lazy daemon advertises how its index is started: {initialized}"
    );
    b.send(INITED);
    let listed = b.request(LIST);
    assert_eq!(listed["result"]["tools"].as_array().map(Vec::len), Some(9));
    assert_eq!(b.request(PING)["result"], json!({}));
    let unknown = b.request(NO_SUCH_TOOL);
    assert_eq!(unknown["result"]["isError"], true, "{unknown}");
    assert_eq!(
        status(&state),
        status_of(BuildState::Building, 1, 1),
        "none of that waited on the build"
    );

    gate.open();
    assert_eq!(
        a.recv()["result"]["structuredContent"]["query"],
        RUN_CALLEES
    );
    close(a, a_worker);
    close(b, b_worker);
}

/// The warming result is the in-process server's (spec §5), because it comes from the same
/// `readiness_response`.
#[test]
fn a_client_with_no_wait_is_told_the_index_is_warming_and_its_retry_is_served() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(0));
    a.handshake();

    let warming = a.request(CALLEES);
    assert_eq!(warming["id"], 7);
    assert_eq!(warming["result"]["isError"], true, "{warming}");
    assert_eq!(warming["result"]["structuredContent"]["status"], "warming");
    assert_eq!(warming["result"]["_meta"]["prism/index_state"], "warming");
    assert_eq!(warming["result"]["_meta"]["prism/retryable"], true);

    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(
        built(&builds),
        1,
        "the warming answer started no second build"
    );
    close(a, a_worker);
}

/// Rules out a budget made per message: the second call would wait a second full budget.
#[test]
fn a_connection_s_budget_spans_its_calls_within_one_attempt() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (mut a, a_worker) = connect(&state, client_settings(20));
    a.handshake();

    a.send(CALLEES);
    wait_until("the first call to be waiting", || {
        status(&state).waiting == 1
    });
    advance(&clock, &state, 20);
    let first = a.recv();
    assert_eq!(first["result"]["structuredContent"]["status"], "warming");
    assert_eq!(first["result"]["structuredContent"]["elapsed_secs"], 20);

    // The budget for this attempt is spent: the next call is answered without waiting. With
    // a budget per message it would wait, and the read below would give up.
    let second = a.request(CALLEES);
    assert_eq!(second["result"]["structuredContent"]["status"], "warming");
    assert_eq!(status(&state).waiting, 0);

    gate.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 1);
    close(a, a_worker);
}

#[test]
fn a_failed_build_is_reported_as_the_in_process_server_reports_it_and_the_next_call_retries() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![fail_now("no such repository")]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();

    let failed = a.request(CALLEES);
    assert_eq!(failed["result"]["isError"], true, "{failed}");
    assert_eq!(
        failed["result"]["structuredContent"]["status"],
        "build_failed"
    );
    assert_eq!(
        failed["result"]["structuredContent"]["cause"],
        "no such repository"
    );
    assert_eq!(failed["result"]["_meta"]["prism/index_state"], "failed");
    assert_eq!(failed["result"]["_meta"]["prism/retryable"], true);
    assert!(failed["result"]["structuredContent"]
        .get("owner_admission")
        .is_none());
    assert_eq!(status(&state), status_of(BuildState::Failed, 1, 0));

    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(status(&state), status_of(BuildState::Ready, 2, 0));
    assert_eq!(built(&builds), 2);
    close(a, a_worker);
}

/// A client that disconnects cancels nothing: the build goes on, its result is kept, and the
/// next client is served from it. Rules out a build tied to the connection that asked for it
/// and a waiter left counted.
#[test]
fn a_client_that_leaves_while_its_call_waits_leaves_no_waiter_and_the_build_is_kept() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    a.send(CALLEES);
    wait_until("A's call to be waiting on the build", || {
        status(&state).waiting == 1
    });

    drop(a);
    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    b.send(CALLEES);
    wait_until("B's call to join the build A's call started", || {
        status(&state).waiting == 2
    });
    assert_eq!(status(&state).attempt, 1);

    gate.open();
    assert_eq!(
        b.recv()["result"]["structuredContent"]["query"],
        RUN_CALLEES
    );
    // A's worker wakes with the build's outcome, finds its peer gone and ends: with a write
    // error, or cleanly if the kernel took the bytes. Either way it ends and does not panic.
    let _how_it_ended = ended(a_worker);
    assert_eq!(status(&state), status_of(BuildState::Ready, 1, 0));
    assert_eq!(built(&builds), 1);
    close(b, b_worker);
}

/// What "does not leak a waiter" is bounded by: a departed client's call stays parked until
/// its attempt ends or its own deadline passes, whichever is first, and no longer.
#[test]
fn a_departed_client_s_call_is_parked_no_longer_than_its_budget() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (mut a, a_worker) = connect(&state, client_settings(20));
    a.handshake();
    a.send(CALLEES);
    wait_until("A's call to be waiting on the build", || {
        status(&state).waiting == 1
    });
    wait_until("the build thread to call the builder", || {
        built(&builds) == 1
    });

    drop(a);
    advance(&clock, &state, 20);
    let _how_it_ended = ended(a_worker);
    assert_eq!(
        status(&state),
        status_of(BuildState::Building, 1, 0),
        "the call is gone and the build goes on"
    );
    assert_eq!(built(&builds), 1);
    gate.open();
}

/// Nothing in the library stops a caller of `DaemonState::read` or `write` from looking into
/// the slot before `Ready` is announced; serving a call is what must not do it. Here the
/// build thread is held after the provider is in the slot and before the announcement. A
/// query and a refresh that arrive then are told the index is warming, and the provider is
/// not touched. Rules out serving from whatever is in the slot, and a `write` reached without
/// `ensure_ready` having returned ready.
#[test]
fn a_call_is_not_served_from_the_slot_before_ready_is_announced() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let reached = Gate::closed();
    let resume = Gate::closed();
    let hook: AttemptHook = {
        let reached = Arc::clone(&reached);
        let resume = Arc::clone(&resume);
        Arc::new(move |_attempt: u64| {
            reached.open();
            let _opened = resume.pass();
        })
    };
    parts.hooks.published = Some(hook);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(0));
    a.handshake();

    // The first call starts the build; this client waits zero seconds, so it is answered.
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["structuredContent"]["status"], "warming");
    assert!(
        reached.pass(),
        "the build thread reaches the publication hook"
    );
    assert_eq!(
        state.read(|_| ()),
        Ok(Some(())),
        "the provider is in the slot"
    );
    assert_eq!(
        status(&state).state,
        BuildState::Building,
        "and Ready has not been announced"
    );

    let query = a.request(CALLEES);
    assert_eq!(
        query["result"]["structuredContent"]["status"], "warming",
        "a query is not answered from a slot that has not been announced: {query}"
    );
    let refresh = a.request(REFRESH);
    assert_eq!(
        refresh["result"]["structuredContent"]["status"], "warming",
        "a refresh does not run on it either: {refresh}"
    );
    assert_eq!(
        state.read(|provider| provider.generation()),
        Ok(Some(0)),
        "the provider was not refreshed"
    );
    assert_eq!(status(&state).generation, 0);

    resume.open();
    wait_until("the build to be announced", || {
        status(&state).state == BuildState::Ready
    });
    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 1);
    close(a, a_worker);
}

/// The owner's decision (D26): a call is not told of a failure that a retry has already
/// superseded. A's call waits on attempt 1, which fails; before A's call runs again, B's call
/// has started the retry and been served from it. A is then told that the repository is being
/// indexed and to retry, in the envelope of the other retryable statuses, and its retry is
/// served. Rules out answering `build_failed` with a stale cause, and a status that clients
/// keyed on the index state would not treat as warming.
#[test]
fn a_call_whose_failed_attempt_was_superseded_is_told_to_retry_and_its_retry_is_served() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![fail_after(&first_gate, "first attempt failed")]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock);
    let recorded = Gate::closed();
    let notify = Gate::closed();
    let hook: AttemptHook = {
        let recorded = Arc::clone(&recorded);
        let notify = Arc::clone(&notify);
        Arc::new(move |attempt: u64| {
            if attempt == 1 {
                recorded.open();
                let _opened = notify.pass();
            }
        })
    };
    parts.hooks.settled = Some(hook);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();
    a.send(CALLEES);
    wait_until("A's call to be waiting on attempt 1", || {
        status(&state).waiting == 1
    });
    first_gate.open();
    assert!(recorded.pass(), "attempt 1's failure is recorded");

    // B's call is the next call after the failure: it starts the retry and is served from it.
    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    let from_b = b.request(CALLEES);
    assert_eq!(from_b["result"]["structuredContent"]["query"], RUN_CALLEES);

    // The retry's announcement woke A's call. Its own attempt failed; the index is ready.
    let told = a.recv();
    assert_eq!(told["id"], 7);
    assert_eq!(told["result"]["isError"], true, "{told}");
    assert_eq!(
        told["result"]["structuredContent"],
        json!({
            "status": "indexing",
            "retry_after_secs": 5,
            "message": "prism-mcp is indexing the repository; retry this call in about 5 seconds. If you already retried and that call succeeded, ignore this message."
        }),
        "{told}"
    );
    let text = told["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .as_ref(),
        Some(&told["result"]["structuredContent"]),
        "the text view carries the same status"
    );
    assert_eq!(told["result"]["_meta"]["prism/index_state"], "warming");
    assert_eq!(told["result"]["_meta"]["prism/retryable"], true);
    assert!(
        !told.to_string().contains("first attempt failed"),
        "the superseded failure is not sent: {told}"
    );
    notify.open();

    let served = a.request(CALLEES);
    assert_eq!(served["result"]["structuredContent"]["query"], RUN_CALLEES);
    assert_eq!(built(&builds), 2);
    close(a, a_worker);
    close(b, b_worker);
}
