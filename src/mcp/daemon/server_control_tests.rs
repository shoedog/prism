//! Tests for the control exchange in the accept loop (spec §4.7): a `STATUS` or `SHUTDOWN` line
//! is answered with the daemon's state and closed, never admitted; a probe does not keep an idle
//! daemon up; a shutdown is for one instance, never disconnects a client, and drains the daemon
//! once the last client has gone.
use super::super::control::StopReason;
use super::super::server_support::{
    gone, options, read_all, send_line, served_key, start, Running, HANDSHAKE_SECS, TTL_SECS,
};
use super::super::test_support::{advance, wait_until};
use super::super::wire_support::PING;
use crate::mcp::shared::control::{ControlRequest, DaemonStatus, Instance};
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::preamble::{BuildState, Reply};
use std::io::Write as _;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn this_instance() -> Instance {
    Instance {
        identity: served_key(),
        pid: std::process::id(),
        start_time: 1234,
    }
}

/// Send `request` on a fresh connection and return everything the daemon sends until it closes.
fn exchange(running: &Running, line: &str) -> String {
    exchange_on(running.connect(), line)
}

/// Send `line` on `stream` and return everything the daemon sends until it closes. A failed
/// write is not an error: a daemon at its bound answers `BUSY` and closes without reading, and
/// that reply is still there to read (as `Running::refusal` does).
fn exchange_on(stream: UnixStream, line: &str) -> String {
    let _ = (&stream).write_all(line.as_bytes());
    read_all(stream)
}

/// Send `request` and decode the one status line it is answered with.
fn status_after(running: &Running, request: ControlRequest) -> DaemonStatus {
    let reply = exchange(running, &request.encode());
    let line = reply
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("one whole line: {reply:?}"));
    assert!(!line.contains('\n'), "one line, then the close: {reply:?}");
    DaemonStatus::decode(line.as_bytes()).unwrap_or_else(|error| panic!("{error}: {reply:?}"))
}

/// Spec §4.7: the status names the instance (identity, pid, start time), its build and root, the
/// clients admitted and pending (the probe not counted), the build's state and attempt, the
/// generation and the idle time; then the connection is closed. Rules out a probe that is
/// admitted, and a status with the probe counted as pending.
#[test]
fn a_status_request_is_answered_with_the_daemon_s_state_and_closed() {
    let running = start(options(64), |_| {});
    let client = running.admitted();
    let status = status_after(&running, ControlRequest::Status);
    assert_eq!(
        status,
        DaemonStatus {
            instance: this_instance(),
            build_identity: "build".to_string(),
            repo_root: running.repo.display().to_string(),
            admitted: 1,
            pending: 0,
            max_clients: 64,
            build: BuildState::Idle,
            attempt: 0,
            generation: 0,
            idle_secs: None,
            idle_ttl_secs: TTL_SECS,
            shutdown: false,
        }
    );
    drop(client);
    wait_until("the client's admission to be released", || {
        running
            .state
            .admission_status()
            .is_ok_and(|counts| counts.admitted == 0 && counts.pending == 0)
    });
    running.settle();
    let idle = status_after(&running, ControlRequest::Status);
    assert_eq!((idle.admitted, idle.idle_secs), (0, Some(0)));
}

/// S4's interface for S5b: an operator who looks at an idle daemon does not keep it up. Rules out
/// a probe that leaves the handshake stage as a client does, restarting the idle time.
#[test]
fn a_status_probe_does_not_keep_an_idle_daemon_up() {
    let mut running = start(options(64), |_| {});
    running.settle();
    running.advance(TTL_SECS - 1);
    let status = status_after(&running, ControlRequest::Status);
    assert_eq!(status.idle_secs, Some(TTL_SECS - 1));
    running.advance_and_wake(1);
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// A line with a control verb that this build refuses (another version of the exchange) is
/// closed without a reply, and does not restart the idle time either. Rules out a refused
/// control line that leaves as a client does.
#[test]
fn a_refused_control_line_is_closed_without_a_reply_and_leaves_the_idle_time() {
    let mut running = start(options(64), |_| {});
    running.settle();
    running.advance(TTL_SECS - 1);
    assert_eq!(exchange(&running, "STATUS {\"control\":2}\n"), "");
    running.advance_and_wake(1);
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// Spec §4.7: `daemon-shutdown` asks the instance to drain when its admitted count reaches 0; a
/// daemon with no client drains at once, without waiting for the idle TTL, and leaves its key
/// free. Rules out a shutdown that is recorded and never acted on.
#[test]
fn a_shutdown_for_this_instance_drains_an_idle_daemon() {
    let mut running = start(options(64), |_| {});
    let status = status_after(&running, ControlRequest::Shutdown(this_instance()));
    assert!(status.shutdown);
    assert_eq!(running.stopped(), StopReason::Shutdown);
    assert!(gone(running.paths.socket()));
    assert!(gone(running.paths.pid_file()));
}

/// Spec §11 item 3: the shutdown is instance-bound. A request naming another identity, pid or
/// start time (a daemon that has since been replaced, say) is answered with the status and
/// records nothing. Rules out a check of any one of the three alone.
#[test]
fn a_shutdown_for_another_instance_records_nothing() {
    let running = start(options(64), |_| {});
    let other_key = IdentityKey::from_hex(&"ef".repeat(32)).expect("a key");
    let others = [
        Instance {
            identity: other_key,
            ..this_instance()
        },
        Instance {
            pid: std::process::id().wrapping_add(1),
            ..this_instance()
        },
        Instance {
            start_time: 1235,
            ..this_instance()
        },
    ];
    for other in others {
        let status = status_after(&running, ControlRequest::Shutdown(other));
        assert!(!status.shutdown, "{other:?}");
        assert_eq!(status.instance, this_instance());
    }
    running.settle();
    assert!(running.is_running());
    assert!(!status_after(&running, ControlRequest::Status).shutdown);
}

/// Spec §4.7 and §11 item 3: a shutdown never disconnects a client. The admitted client goes on
/// being served, a client that arrives after the request is admitted too (D100), and the daemon
/// drains once the last of them has gone. Rules out a drain while a client is admitted, and a
/// daemon that turns new clients away after the request.
#[test]
fn a_shutdown_lets_every_client_finish_and_drains_when_the_last_leaves() {
    let mut running = start(options(64), |_| {});
    let mut first = running.admitted();
    let status = status_after(&running, ControlRequest::Shutdown(this_instance()));
    assert_eq!((status.admitted, status.shutdown), (1, true));
    running.settle();
    assert!(running.is_running());
    assert!(first.request(PING)["result"].is_object());

    let mut second = running.admitted();
    assert!(second.request(PING)["result"].is_object());
    drop(first);
    running.settle();
    assert!(running.is_running(), "one client is left");
    drop(second);
    assert_eq!(running.stopped(), StopReason::Shutdown);
}

/// A control request is a connection like any other until its line is read: it counts against
/// `--max-clients`, and a daemon at its bound answers it `BUSY` (spec §4.2). Recorded behavior,
/// not a choice: the bound protects the daemon from connections it has not read yet.
#[test]
fn a_control_request_at_the_client_bound_is_answered_busy() {
    let running = start(options(1), |_| {});
    let _client = running.admitted();
    // The daemon answers `BUSY` and closes without reading, so it can do so before the request
    // is written; wait long enough that it has (review of #416, Opus W1 / sol): the write then
    // fails, and the reply must still be read.
    let stream = running.connect();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        exchange_on(stream, &ControlRequest::Status.encode()),
        Reply::Busy { max_clients: 1 }.encode()
    );
}

/// Review round 1, sol 3: a control line whose last byte arrives at its handshake deadline is
/// not answered, records no shutdown, and leaves the idle time as it found it. The probe enters
/// one second before the idle TTL ends and its line completes at its deadline, ten seconds later
/// (the loop's line seam moves the clock between the read and the deadline check, D72); the
/// daemon then retires for being idle since before the probe. Rules out an expired control line
/// that leaves as a client does (the TTL would restart: the loop would not stop) and one that
/// records its shutdown (it would stop for `Shutdown`).
#[test]
fn a_control_line_completed_after_its_deadline_moves_nothing() {
    type Step = Box<dyn FnOnce() + Send>;
    let armed: Arc<Mutex<Option<Step>>> = Arc::default();
    let slot = Arc::clone(&armed);
    let mut running = start(options(64), move |server| {
        server.set_line_read_hook(Arc::new(move || {
            let step = slot.lock().expect("armed").take();
            if let Some(step) = step {
                step();
            }
        }))
    });
    running.settle();
    running.advance(TTL_SECS - 1);
    let (clock, state) = (Arc::clone(&running.clock), Arc::clone(&running.state));
    *armed.lock().expect("armed") = Some(Box::new(move || {
        advance(&clock, &state, HANDSHAKE_SECS);
    }));
    let reply = exchange(
        &running,
        &ControlRequest::Shutdown(this_instance()).encode(),
    );
    assert_eq!(reply, "", "an expired line is not answered");
    assert_eq!(running.stopped(), StopReason::Idle);
}

/// What the daemon does with a connection that is not a control exchange it can read: a line
/// that is neither a `HELLO` nor a control line, or a connection that ends before its line is
/// whole, is treated as a client that left, and the idle time restarts (it cannot tell them
/// from a client's). This is also what a daemon of a build before S5b does with an S5b
/// operator's `STATUS`, whose verb it does not know (review round 1, Opus W2, sol 1: accepted
/// and disclosed). Pins the behavior, so that a change to it is a decision, not an accident.
#[test]
fn a_line_the_daemon_cannot_read_restarts_the_idle_time_as_a_client_s_does() {
    for (sent, then_close) in [("PING {}\n", false), ("STATUS {\"cont", true)] {
        let mut running = start(options(64), |_| {});
        running.settle();
        running.advance(TTL_SECS - 1);
        let stream = running.connect();
        send_line(&stream, sent);
        if then_close {
            wait_until("the connection to be accepted", || {
                running
                    .state
                    .admission_status()
                    .is_ok_and(|counts| counts.pending == 1)
            });
            drop(stream);
        } else {
            assert_eq!(read_all(stream), "", "{sent:?} is not answered");
        }
        wait_until("the connection to leave the handshake stage", || {
            running
                .state
                .admission_status()
                .is_ok_and(|counts| counts.pending == 0)
        });
        running.advance(1);
        assert!(running.is_running(), "{sent:?}: the idle time restarted");
        running.advance_and_wake(TTL_SECS - 1);
        assert_eq!(running.stopped(), StopReason::Idle, "{sent:?}");
    }
}
