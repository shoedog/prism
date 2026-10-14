//! A daemon connection whose own build attempt failed, served once another client's retry has
//! published the index (the owner's decision of 2026-10-13, amending D26). Kept apart from
//! `connection_tests.rs` to keep both files under 600 lines.
use super::super::coordinator::DaemonState;
use super::super::test_support::{
    client_settings, fail_after, fixture, scripted, status, wait_until, Gate,
};
use super::super::wire_support::{close, connect, CALLEES};
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::{RefreshPolicy, StartupMode};
use std::sync::atomic::{AtomicU64, Ordering};

const RUN_CALLEES: &str = "callees:run@main.py";

fn built(builds: &AtomicU64) -> u64 {
    builds.load(Ordering::SeqCst)
}

/// The owner's decision of 2026-10-13 on the path of D32: a client that stopped waiting on an
/// attempt that then failed, while another client's retry has since published the index, is
/// served on its next call, and so is the call after it. Rules out telling it to retry
/// (`indexing`) with nothing building, and reporting the stale failure later.
#[test]
fn a_client_that_stopped_waiting_is_served_once_another_client_s_retry_has_published() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let first_gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![fail_after(&first_gate, "first attempt failed")]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(0));
    a.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["structuredContent"]["status"], "warming");
    first_gate.open();
    wait_until("attempt 1 to fail", || {
        status(&state).state == BuildState::Failed
    });
    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    let from_b = b.request(CALLEES);
    assert_eq!(from_b["result"]["structuredContent"]["query"], RUN_CALLEES);

    for _ in 0..2 {
        let served = a.request(CALLEES);
        assert_eq!(
            served["result"]["structuredContent"]["query"], RUN_CALLEES,
            "{served}"
        );
        assert!(
            !served.to_string().contains("first attempt failed"),
            "{served}"
        );
    }
    assert_eq!(built(&builds), 2);
    close(a, a_worker);
    close(b, b_worker);
}
