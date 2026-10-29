//! Production-default parity for successful builds completing during a drain.
use super::control::StopReason;
use super::coordinator::DaemonState;
use super::test_support::{
    build_after, client_settings, fixture, parts_with, scripted, status, wait_until, Gate,
    ManualClock,
};
use super::wire_support::{close, connect, CALLEES};
use crate::mcp::shared::preamble::BuildState;
use crate::mcp::RefreshPolicy;
use std::sync::{atomic::Ordering, Arc};

fn successful_build_during_drain(publication_first: bool) {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let build_gate = Gate::closed();
    let (builder, builds) = scripted(&cfg, vec![build_after(&build_gate, &cfg)]);
    let clock = ManualClock::new();
    let mut parts = parts_with(builder, &clock); // this helper builds with unloading disabled
    let published = Gate::closed();
    let settle_gate = Gate::closed();
    if publication_first {
        let reached = published.clone();
        let resume = settle_gate.clone();
        parts.hooks.published = Some(Arc::new(move |_| {
            reached.open();
            let _ = resume.pass();
        }));
    }
    let state = DaemonState::from_parts(parts).unwrap();
    let (mut client, worker) = connect(&state, client_settings(600));
    client.handshake();
    client.send(CALLEES);
    wait_until("socket call parked on its successful build", || {
        status(&state).waiting == 1
    });
    if publication_first {
        build_gate.open();
        assert!(published.pass(), "build reaches publication before drain");
    }
    assert_eq!(status(&state).state, BuildState::Building);
    assert_eq!(state.read(|_| ()).unwrap().is_some(), publication_first);
    state.drain(StopReason::Signal, || {}).unwrap();
    // Never advance the manual clock: completion is inside the original wait budget.
    build_gate.open();
    settle_gate.open();
    let response = client.recv();
    close(client, worker);
    assert_eq!(
        response["result"]["isError"], false,
        "idle-off successful build during drain must answer Ready and serve: {response}"
    );
    assert!(response.to_string().contains("helper"), "{response}");
    assert_eq!(response["id"], 7);
    assert_eq!(status(&state).state, BuildState::Ready);
    assert_eq!(status(&state).attempt, 1);
    assert_eq!(builds.load(Ordering::SeqCst), 1, "exactly one build");
    assert!(state
        .read(|p| p.session().repo.files["util.py"]
            .source
            .contains("def helper"))
        .unwrap()
        .unwrap());
}

#[test]
fn daemon_idle_off_drain_before_publication_serves_build() {
    successful_build_during_drain(false);
}

#[test]
fn daemon_idle_off_drain_before_settlement_serves_build() {
    successful_build_during_drain(true);
}
