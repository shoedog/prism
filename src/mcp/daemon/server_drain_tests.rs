//! Tests for retirement and Draining in the accept loop (spec §3.1, §4.2 "Admission and
//! retirement", §4.3 root identity; §9 tests 2 and 13 as far as they run before the shim).
use super::super::control::StopReason;
use super::super::files::{is_socket, Claim, DaemonFiles};
use super::super::server_support::{
    gone, hello_line, options, read_all, runtime_base, send_line, served_key, start, start_with,
    TTL_SECS,
};
use super::super::test_support::{build_after, fixture, scripted, state_with, Gate, ManualClock};
use super::super::wire_support::CALLEES;
use super::IDENTITY_CHECK_INTERVAL;
use crate::mcp::shared::preamble::{BuildState, Reply};
use crate::mcp::shared::runtime_dir::current_uid;
use crate::mcp::RefreshPolicy;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn retiring() -> String {
    Reply::Retiring.encode()
}

/// Spec §9 test 2, with an injected clock: after the last client leaves, the daemon stays up
/// for the idle TTL and then drains: its socket and pid file are removed and its owner lock is
/// free for the next daemon. Rules out a TTL that counts from the daemon's start or the
/// client's arrival, and a drain that leaves the key owned.
#[test]
fn an_idle_daemon_retires_after_the_ttl_and_leaves_the_key_free() {
    let mut running = start(options(64), |_| {});
    let client = running.admitted();
    running.advance(2 * TTL_SECS);
    assert!(running.is_running(), "an admitted client holds it up");
    drop(client);
    super::super::test_support::wait_until("the client's admission to be released", || {
        running
            .state
            .admission_status()
            .is_ok_and(|status| status.admitted == 0)
    });
    running.advance(TTL_SECS - 1);
    assert!(running.is_running(), "one second short of the TTL");

    running.advance_and_wake(1);
    assert_eq!(running.stopped(), StopReason::Idle);
    assert!(gone(running.paths.socket()));
    assert!(gone(running.paths.pid_file()));
    assert!(
        UnixStream::connect(running.paths.socket()).is_err(),
        "nothing answers at the socket's path"
    );
    assert!(matches!(
        DaemonFiles::claim(&running.runtime_dir, &served_key()),
        Ok(Claim::Owned(_))
    ));
}

/// A daemon nobody ever reaches (its starter died before connecting) retires after the idle TTL
/// like any other: the count starts with the loop's first turn.
#[test]
fn a_daemon_nobody_reaches_retires_after_the_ttl() {
    let mut running = start(options(64), |_| {});
    running.settle();
    running.advance(TTL_SECS - 1);
    assert!(running.is_running());
    running.advance_and_wake(1);
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// Spec §4.2: no retirement while a build runs, however long nobody has been connected.
#[test]
fn a_running_build_keeps_an_idle_daemon_up_until_it_ends() {
    let (repo_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let gate = Gate::closed();
    let (builder, _builds) = scripted(&cfg, vec![build_after(&gate, &cfg)]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (base, runtime_dir) = runtime_base();
    let mut running = start_with(
        state,
        clock,
        repo_dir,
        base,
        runtime_dir,
        options(64),
        |_| {},
    );
    let mut client = running.admitted_waiting(0);
    let answer = client.request(CALLEES);
    assert_eq!(
        answer["result"]["structuredContent"]["status"], "warming",
        "{answer}"
    );
    drop(client);
    super::super::test_support::wait_until("the client's admission to be released", || {
        running
            .state
            .admission_status()
            .is_ok_and(|status| status.admitted == 0)
    });

    running.advance(2 * TTL_SECS);
    assert!(running.is_running(), "the build holds it up");
    gate.open();
    super::super::test_support::wait_until("the build to be announced ready", || {
        running
            .state
            .status()
            .is_ok_and(|status| status.state == BuildState::Ready)
    });
    running.advance_and_wake(0);
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// Spec §9 test 13, the daemon's half: a shim that has connected but whose connection has not
/// been accepted when the daemon retires is answered `RETIRING`, not dropped, and the key is free
/// for a new daemon that admits it on its next try. The loop is held at the top of a turn while
/// the client connects and the clock passes the TTL, so the retirement decision and the queued
/// connection meet deterministically.
#[test]
fn a_connection_queued_while_the_daemon_retires_is_answered_retiring() {
    let mut running = start(options(64), |_| {});
    running.settle();
    let gate = running.hold();
    let queued = running.connect();
    send_line(&queued, &hello_line(served_key()));
    super::super::test_support::advance(&running.clock, &running.state, TTL_SECS);
    gate.open();
    assert_eq!(read_all(queued), retiring());
    assert_eq!(running.stopped(), StopReason::Idle);

    let (repo_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (base, _unused) = runtime_base();
    let fresh = start_with(
        state,
        clock,
        repo_dir,
        base,
        running.runtime_dir.clone(),
        options(64),
        |_| {},
    );
    let _admitted = fresh.admitted();
}

/// Spec §3.1, §4.3: a repository root that is replaced (renamed away, and a new directory made
/// at its path) drains the daemon at the next check, whatever its connections; a connection in
/// the handshake stage is answered `RETIRING`. Rules out a check of the path's existence alone.
#[test]
fn a_replaced_root_drains_the_daemon_at_the_next_check() {
    let mut running = start(options(64), |_| {});
    let _admitted = running.admitted();
    let pending = running.connect();
    running.settle();

    let moved = running.repo.with_extension("moved");
    std::fs::rename(&running.repo, &moved).expect("move the root away");
    std::fs::create_dir(&running.repo).expect("a new directory at the root's path");
    running.advance_and_wake(IDENTITY_CHECK_INTERVAL.as_secs());
    assert_eq!(running.stopped(), StopReason::RootChanged);
    assert_eq!(read_all(pending), retiring());
    assert!(gone(running.paths.socket()));
    assert!(gone(running.paths.pid_file()));
    std::fs::remove_dir(&running.repo).expect("remove the new directory");
    std::fs::rename(&moved, &running.repo).expect("restore the root for the cleanup");
}

/// Spec §4.3: the root is also checked on every admission, so a `HELLO` that arrives after the
/// root was replaced and before the next periodic check is answered `RETIRING`, not admitted.
#[test]
fn a_replaced_root_is_noticed_at_admission() {
    let mut running = start(options(64), |_| {});
    running.settle();
    let moved = running.repo.with_extension("moved");
    std::fs::rename(&running.repo, &moved).expect("move the root away");

    assert_eq!(running.refusal(served_key()), retiring());
    running.waker.wake();
    assert_eq!(running.stopped(), StopReason::RootChanged);
    std::fs::rename(&moved, &running.repo).expect("restore the root for the cleanup");
}

/// Lane README, notes for S4: a runtime directory replaced under the daemon (a cleaner, the last
/// logout) drains it at the next check, and nothing in the new directory is touched.
#[test]
fn replaced_files_drain_the_daemon_and_the_new_directory_is_left_alone() {
    let mut running = start(options(64), |_| {});
    running.settle();
    let moved = running.runtime_dir.with_extension("moved");
    std::fs::rename(&running.runtime_dir, &moved).expect("move the runtime directory");
    let mut newcomer = match DaemonFiles::claim(&running.runtime_dir, &served_key())
        .expect("claim in the new directory")
    {
        Claim::Owned(files) => *files,
        Claim::Taken => panic!("the new directory's key is free"),
    };
    let _listener = newcomer.bind().expect("bind");

    running.advance_and_wake(IDENTITY_CHECK_INTERVAL.as_secs());
    assert_eq!(running.stopped(), StopReason::FilesReplaced);
    assert!(
        is_socket(newcomer.paths().socket()),
        "the newcomer's socket stays"
    );
}

/// Spec §3.1: SIGTERM and SIGINT go through Draining whatever the connections. A stop request is
/// what the signal handler makes; the daemon drains, and nothing new is admitted.
#[test]
fn a_stop_request_drains_the_daemon_with_a_client_attached() {
    let mut running = start(options(64), |_| {});
    let _admitted = running.admitted();
    running.waker.request_stop();
    assert_eq!(running.stopped(), StopReason::Signal);
    assert!(gone(running.paths.socket()));
    assert!(gone(running.paths.pid_file()));
}

/// The peer is checked before any reply on the drain's `RETIRING` path too (review round 1, Opus
/// S1): of two connections queued while the daemon retires, this user's is answered `RETIRING`
/// and another user's gets nothing.
#[test]
fn a_peer_of_another_user_queued_while_the_daemon_retires_is_not_answered() {
    let seen = Arc::new(AtomicU64::new(0));
    let peers = Arc::clone(&seen);
    let mut running = start(options(64), move |server| {
        server.set_peer_uid(Box::new(move |_| {
            match peers.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(current_uid()),
                _ => Ok(current_uid().wrapping_add(1)),
            }
        }))
    });
    running.settle();
    let gate = running.hold();
    let ours = running.connect();
    let theirs = running.connect();
    super::super::test_support::advance(&running.clock, &running.state, TTL_SECS);
    gate.open();
    assert_eq!(read_all(ours), retiring());
    assert_eq!(read_all(theirs), "");
    assert_eq!(running.stopped(), StopReason::Idle);
}
