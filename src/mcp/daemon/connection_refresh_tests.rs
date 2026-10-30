//! Refresh visibility (spec §9 test 6) and the automatic refresh's trigger, checked again once
//! exclusive access is held (spec §4.2): two clients of one daemon, each on its own connection.
use super::super::coordinator::{DaemonState, Parts};
use super::super::test_support::{client_settings, fixture, scripted, status, wait_until, Gate};
use super::super::wire_support::{close, connect, Client, CALLEES, CALLERS, REFRESH};
use crate::mcp::freshness::FreshnessReport;
use crate::mcp::{RefreshPolicy, RefreshVerification, StartupMode};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// What the edit below adds: a second caller of `helper`. A response that names it was
/// computed on a snapshot taken after the edit.
const NEW_CALLER: &str = "extra";

fn add_a_caller(root: &Path) {
    std::fs::write(
        root.join("util.py"),
        "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
    )
    .expect("edit util.py");
}

fn names_the_new_caller(response: &Value) -> bool {
    response["result"].to_string().contains(NEW_CALLER)
}

/// Holds each call that reaches the step between its read and its exclusive access at a gate
/// of its own, in the order the calls arrive there.
struct Turnstile {
    arrived: Mutex<usize>,
    gates: Vec<Arc<Gate>>,
}

impl Turnstile {
    fn new(calls: usize) -> Arc<Self> {
        Arc::new(Self {
            arrived: Mutex::new(0),
            gates: (0..calls).map(|_| Gate::closed()).collect(),
        })
    }

    fn arrived(&self) -> usize {
        *self.arrived.lock().expect("turnstile")
    }

    fn release(&self, call: usize) {
        self.gates[call].open();
    }
}

/// A lazy daemon whose calls stop at `turnstile` before they ask for exclusive access.
fn daemon_with_turnstile(
    cfg: &crate::mcp::ServerConfig,
    turnstile: &Arc<Turnstile>,
) -> Arc<DaemonState> {
    let (builder, _builds) = scripted(cfg, vec![]);
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let turnstile = Arc::clone(turnstile);
    parts.hooks.exclusive = Some(Arc::new(move || {
        let call = {
            let mut arrived = turnstile.arrived.lock().expect("turnstile");
            *arrived += 1;
            *arrived - 1
        };
        if let Some(gate) = turnstile.gates.get(call) {
            let _opened = gate.pass();
        }
    }));
    DaemonState::from_parts(parts).expect("a lazy daemon state")
}

/// Both clients send a query on a stale index, and both get as far as asking for exclusive
/// access before either has it. Returns the two clients with their queries in flight.
fn two_stale_queries_waiting_for_exclusive_access(
    a: &mut Client,
    b: &mut Client,
    turnstile: &Turnstile,
) {
    a.send(CALLERS);
    wait_until("A's query to reach exclusive access", || {
        turnstile.arrived() == 1
    });
    b.send(CALLERS);
    wait_until("B's query to reach exclusive access", || {
        turnstile.arrived() == 2
    });
}

/// Spec §9 test 6: once A's refresh has completed, B's next query is answered from the
/// refreshed index (the changed name, no stale warning) and the daemon reports the new
/// generation. Under a warn-only policy an in-process server of B's own would have answered
/// with the old names and a stale warning; a shared index has one state (spec §5 "Changed").
#[test]
fn a_refresh_by_one_client_is_what_the_next_query_of_another_client_sees() {
    let (dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();

    let before = b.request(CALLERS);
    assert_eq!(before["result"]["isError"], false, "{before}");
    assert!(!names_the_new_caller(&before), "{before}");
    assert_eq!(status(&state).generation, 0);

    add_a_caller(dir.path());
    let stale = b.request(CALLERS);
    assert_eq!(
        stale["result"]["_meta"]["prism/index_freshness"], "stale",
        "{stale}"
    );
    assert!(!names_the_new_caller(&stale), "{stale}");

    let refreshed = a.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "refreshed",
        "{refreshed}"
    );
    assert_eq!(refreshed["result"]["structuredContent"]["generation"], 1);

    let after = b.request(CALLERS);
    assert_eq!(after["result"]["isError"], false, "{after}");
    assert!(names_the_new_caller(&after), "{after}");
    assert!(
        after["result"]["_meta"]
            .get("prism/index_freshness")
            .is_none(),
        "{after}"
    );
    assert_eq!(status(&state).generation, 1);

    close(a, a_worker);
    close(b, b_worker);
}

/// Spec §4.2: automatic refresh re-checks its trigger after acquiring exclusive access. A and B
/// both find the index stale under shared access; A refreshes first. B, once it has exclusive
/// access, finds the index fresh: it is answered from A's refresh, with no refresh of its own
/// and no stale warning, and the daemon is at generation 1, not 2. Without the re-check B
/// rebuilds an index that is already current.
#[test]
fn a_client_that_finds_the_index_refreshed_once_it_has_exclusive_access_does_not_refresh_again() {
    let (dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let turnstile = Turnstile::new(2);
    let state = daemon_with_turnstile(&cfg, &turnstile);
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    add_a_caller(dir.path());
    two_stale_queries_waiting_for_exclusive_access(&mut a, &mut b, &turnstile);

    turnstile.release(0);
    let from_a = a.recv();
    assert_eq!(
        from_a["result"]["_meta"]["prism/auto_refresh"], "refreshed",
        "{from_a}"
    );
    assert_eq!(from_a["result"]["_meta"]["prism/refresh_generation"], 1);
    assert!(names_the_new_caller(&from_a), "{from_a}");

    turnstile.release(1);
    let from_b = b.recv();
    assert_eq!(from_b["result"]["isError"], false, "{from_b}");
    assert!(names_the_new_caller(&from_b), "{from_b}");
    assert!(
        from_b["result"]["_meta"]
            .get("prism/auto_refresh")
            .is_none(),
        "B's query found the index current and did not refresh it: {from_b}"
    );
    assert!(
        from_b["result"]["_meta"]
            .get("prism/index_freshness")
            .is_none(),
        "{from_b}"
    );
    assert_eq!(status(&state).generation, 1, "one refresh, not two");

    close(a, a_worker);
    close(b, b_worker);
}

/// The re-check asks the question the read asked (is the index stale, with the refresh's
/// sticky evidence first), not whether another refresh has happened. A's refresh verifies
/// `Diverged`, so the index is still known stale, and B, once it has exclusive access,
/// refreshes again (spec §9 test 14, "auto-refresh retries"). Rules out a re-check that
/// skips the refresh because the generation moved.
#[test]
fn a_client_that_finds_the_index_still_known_stale_once_it_has_exclusive_access_refreshes_it() {
    let (dir, cfg) = fixture(RefreshPolicy::AutoFull);
    let turnstile = Turnstile::new(2);
    let state = daemon_with_turnstile(&cfg, &turnstile);
    let (mut a, a_worker) = connect(&state, client_settings(600));
    let (mut b, b_worker) = connect(&state, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    state
        .write(|provider| {
            provider.force_next_verification_for_tests(RefreshVerification::Diverged(
                FreshnessReport::from_changed_paths(["main.py".to_string()]),
            ));
        })
        .expect("the provider lock is not poisoned")
        .expect("an index is published");
    add_a_caller(dir.path());
    two_stale_queries_waiting_for_exclusive_access(&mut a, &mut b, &turnstile);

    turnstile.release(0);
    let from_a = a.recv();
    assert_eq!(
        from_a["result"]["_meta"]["prism/auto_refresh"], "raced_stale",
        "{from_a}"
    );
    assert_eq!(status(&state).generation, 1);

    turnstile.release(1);
    let from_b = b.recv();
    assert_eq!(
        from_b["result"]["_meta"]["prism/auto_refresh"], "refreshed",
        "{from_b}"
    );
    assert_eq!(from_b["result"]["_meta"]["prism/refresh_generation"], 2);
    assert!(names_the_new_caller(&from_b), "{from_b}");
    assert_eq!(status(&state).generation, 2);

    close(a, a_worker);
    close(b, b_worker);
}
