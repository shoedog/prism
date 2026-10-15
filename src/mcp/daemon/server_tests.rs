//! Tests for the bounded handshake stage (spec §4.2 "Per connection", §4.4, §7, §10; §9 test
//! 16): what each connection is answered, and what is counted, over a real socket.
use super::super::admission::AdmissionStatus;
use super::super::server::ACCEPTS_PER_TURN;
use super::super::server_support::{
    answered, hello_line, options, other_key, read_all, send_line, served_key, start,
    HANDSHAKE_SECS,
};
use super::super::test_support::{advance, wait_until, Gate};
use super::super::wire_support::{Client, CALLEES};
use crate::mcp::shared::preamble::{BuildState, Ready, Reply, PREAMBLE_MAX_BYTES};
use crate::mcp::shared::runtime_dir::current_uid;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

fn counts(pending: u32, admitted: u32) -> AdmissionStatus {
    AdmissionStatus {
        pending,
        admitted,
        draining: None,
    }
}

/// Spec §4.4: a `HELLO` for the served identity is answered `READY` with the daemon's pid,
/// start time, generation and build state, and MCP traffic follows on the same connection;
/// when the client leaves, its admission is released. Rules out a reader that takes bytes past
/// the `HELLO` line (the `initialize` that follows would be lost) and an admission that is never
/// released (the daemon would never retire).
#[test]
fn a_hello_for_the_served_identity_is_answered_ready_and_mcp_follows() {
    let running = start(options(64), |_| {});
    let stream = running.connect();
    // `HELLO` and `initialize` in one write: the reader must stop at the newline.
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
    send_line(&stream, &format!("{}{init}\n", hello_line(served_key())));
    let mut client = Client::new(stream);
    let reply = client.recv_line();
    let decoded = Reply::decode(reply.trim_end().as_bytes()).expect("a reply");
    assert_eq!(
        decoded,
        Reply::Ready(Ready {
            identity: served_key(),
            pid: std::process::id(),
            start_time: 1234,
            generation: 0,
            build: BuildState::Idle,
        })
    );
    let initialized = client.recv();
    assert!(initialized["result"].is_object(), "{initialized}");
    client.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let answer = client.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    assert_eq!(running.state.admission_status(), Ok(counts(0, 1)));

    drop(client);
    wait_until("the admission to be released", || {
        running.state.admission_status() == Ok(counts(0, 0))
    });
}

/// Spec §4.2 "Refusal", §4.4: a `HELLO` for another identity is answered `MISMATCH` with the
/// served key and is not admitted; so is one of another protocol version, which is another
/// identity (S2's SI-3 note).
#[test]
fn another_identity_or_protocol_is_answered_mismatch_and_not_admitted() {
    let running = start(options(64), |_| {});
    assert_eq!(
        running.refusal(other_key()),
        Reply::Mismatch {
            expected: served_key()
        }
        .encode()
    );

    let stream = running.connect();
    let version_two = hello_line(served_key()).replace("\"protocol\":1", "\"protocol\":2");
    send_line(&stream, &version_two);
    assert_eq!(
        read_all(stream),
        Reply::Mismatch {
            expected: served_key()
        }
        .encode()
    );
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
}

/// SI-3, D54: a line the daemon refuses for any other reason (not a `HELLO`, malformed, refused
/// settings, longer than a preamble line) is closed without a reply and is not counted.
#[test]
fn a_refused_preamble_line_is_closed_without_a_reply() {
    let running = start(options(64), |_| {});
    let too_long = format!("HELLO {}\n", "x".repeat(PREAMBLE_MAX_BYTES + 10));
    let below_floor = hello_line(served_key()).replace("\"result_cap\":80000", "\"result_cap\":1");
    for line in [
        "garbage\n",
        "READY {}\n",
        below_floor.as_str(),
        too_long.as_str(),
    ] {
        let stream = running.connect();
        // A long line may be cut off by the close; that is the refusal too.
        let _ = (&stream).write_all(line.as_bytes());
        assert_eq!(read_all(stream), "", "{line:.40}");
    }
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
}

/// Spec §4.4, §7, §10: the peer's uid is checked on every accepted connection, and a peer of
/// another user is dropped before any reply and never counted, so it cannot take a slot of
/// `--max-clients` either. Rules out a check after the reply, or none.
#[test]
fn a_connection_from_another_user_is_dropped_before_any_reply() {
    let running = start(options(1), |server| {
        server.set_peer_uid(Box::new(|_| Ok(current_uid().wrapping_add(1))))
    });
    assert_eq!(running.refusal(served_key()), "");
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
}

/// The peer is checked before any reply on the `BUSY` path too (review round 1, Opus S1): a
/// peer of another user that arrives when the bound is full is dropped, not answered `BUSY`.
/// Rules out a bound checked before the peer.
#[test]
fn a_peer_of_another_user_is_not_answered_busy_either() {
    let seen = Arc::new(AtomicU64::new(0));
    let peers = Arc::clone(&seen);
    let running = start(options(1), move |server| {
        server.set_peer_uid(Box::new(move |_| {
            // The first connection is this user's; every later one another user's.
            match peers.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(current_uid()),
                _ => Ok(current_uid().wrapping_add(1)),
            }
        }))
    });
    let _stalled = running.connect();
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(1, 0)));
    assert_eq!(running.refusal(served_key()), "");
}

/// Spec §9 test 16: connections stalled before `HELLO` count against `--max-clients`; the one
/// over the bound is answered `BUSY` without being admitted, so no worker is started for it
/// (workers exist only for admitted connections). When the stalled ones reach their handshake
/// deadline they are dropped and their slots are free again. Rules out a bound on admitted
/// connections only, and a handshake stage with no deadline.
#[test]
fn connections_over_the_bound_are_answered_busy_and_stalled_ones_time_out() {
    let running = start(options(2), |_| {});
    let stalled_a = running.connect();
    let stalled_b = running.connect();
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(2, 0)));

    assert_eq!(
        running.refusal(served_key()),
        Reply::Busy { max_clients: 2 }.encode()
    );
    assert_eq!(running.state.admission_status(), Ok(counts(2, 0)));

    running.advance(HANDSHAKE_SECS - 1);
    assert_eq!(
        running.state.admission_status(),
        Ok(counts(2, 0)),
        "not before the deadline"
    );
    running.advance(1);
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
    for stalled in [stalled_a, stalled_b] {
        assert_eq!(read_all(stalled), "");
    }
    let _admitted = running.admitted();
}

/// A `HELLO` that arrives a few bytes at a time is read to its end, and the deadline bounds the
/// whole line, not each silence (lane README, notes for S4 and S5a: a reader with only a read
/// timeout could be held for 4,097 timeouts by one byte at a time).
#[test]
fn the_handshake_deadline_bounds_the_whole_line() {
    let running = start(options(64), |_| {});
    let line = hello_line(served_key());
    let (first, rest) = line.split_at(20);

    let slow = running.connect();
    send_line(&slow, first);
    running.settle();
    running.advance(HANDSHAKE_SECS - 1);
    send_line(&slow, &rest[..10]);
    running.advance(1);
    assert_eq!(read_all(slow), "");

    let patient = running.connect();
    send_line(&patient, first);
    running.settle();
    running.advance(HANDSHAKE_SECS - 1);
    send_line(&patient, rest);
    let mut client = Client::new(patient);
    assert!(client.recv_line().starts_with("READY "));
}

/// The deadline bounds a line that completes after it, not only one still incomplete when the
/// loop looks (review round 1, sol F2: a prefix at t=0 and the rest at t=11 was admitted). The
/// loop is held while the rest arrives and the clock passes the deadline, so the line is whole
/// when the loop next reads it. Rules out a deadline checked only after a read that found no
/// newline.
#[test]
fn a_hello_completed_after_its_deadline_is_not_admitted() {
    let running = start(options(64), |_| {});
    let line = hello_line(served_key());
    let (first, rest) = line.split_at(20);
    let late = running.connect();
    send_line(&late, first);
    running.settle();

    let gate = running.hold();
    send_line(&late, rest);
    advance(&running.clock, &running.state, HANDSHAKE_SECS + 1);
    gate.open();
    assert_eq!(read_all(late), "");
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
}

/// One turn of the loop accepts at most `ACCEPTS_PER_TURN` connections, then services the
/// handshake deadlines (and the next turn's stop and root checks), so a backlog that keeps
/// refilling cannot starve them (review round 1, sol F3). The loop is held while a stalled
/// connection's deadline passes and more connections queue than one turn takes; after one turn
/// exactly `ACCEPTS_PER_TURN` of them have been answered (`BUSY`: the stalled one held the only
/// slot while they were accepted) and the stalled one has been dropped. Rules out accepting
/// until the backlog is empty.
#[test]
fn one_turn_accepts_a_bounded_number_of_connections_then_services_deadlines() {
    let running = start(options(1), |_| {});
    let stalled = running.connect();
    running.settle();
    assert_eq!(running.state.admission_status(), Ok(counts(1, 0)));

    let gate = running.hold();
    let queued: Vec<UnixStream> = (0..ACCEPTS_PER_TURN + 8)
        .map(|_| running.connect())
        .collect();
    advance(&running.clock, &running.state, HANDSHAKE_SECS);
    let (next, holds) = running.arm_hold();
    // Opened however the test ends, so that a failing assertion does not leave the loop held
    // until the gate gives up (review round 2, Opus S4).
    let _next = OpenOnDrop(next);
    gate.open();
    running.wait_held(holds);

    assert_eq!(
        read_all(stalled),
        "",
        "the stalled one is dropped in that turn"
    );
    let busy = queued.iter().filter(|stream| answered(stream)).count();
    assert_eq!(busy, ACCEPTS_PER_TURN);
    for stream in queued.into_iter().skip(ACCEPTS_PER_TURN) {
        drop(stream);
    }
}

/// Opens its gate when dropped.
struct OpenOnDrop(Arc<Gate>);

impl Drop for OpenOnDrop {
    fn drop(&mut self) {
        self.0.open();
    }
}

/// The second deadline check (D72): a line whose last byte is read in the instant the deadline
/// passes is not answered. The loop's line seam moves the clock between the read and that
/// check, which is where a real clock can cross the deadline: before the read the clock is one
/// second short of it, after the read it is at it. Control: the same line with the clock left
/// where it was is admitted. Rules out answering a line on the reading taken before the read.
#[test]
fn a_line_whose_read_ends_at_the_deadline_is_not_answered() {
    for (step_secs, admitted) in [(0, true), (1, false)] {
        type Step = Box<dyn FnOnce() + Send>;
        let armed: Arc<Mutex<Option<Step>>> = Arc::default();
        let slot = Arc::clone(&armed);
        let running = start(options(64), move |server| {
            server.set_line_read_hook(Arc::new(move || {
                let step = slot.lock().expect("armed").take();
                if let Some(step) = step {
                    step();
                }
            }))
        });
        let line = hello_line(served_key());
        let (most, last) = line.split_at(line.len() - 1);
        let stream = running.connect();
        send_line(&stream, most);
        running.settle();
        running.advance(HANDSHAKE_SECS - 1);
        assert_eq!(running.state.admission_status(), Ok(counts(1, 0)));

        let clock = Arc::clone(&running.clock);
        let state = Arc::clone(&running.state);
        *armed.lock().expect("armed") = Some(Box::new(move || {
            advance(&clock, &state, step_secs);
        }));
        send_line(&stream, last);
        if admitted {
            let mut client = Client::new(stream);
            assert!(
                client.recv_line().starts_with("READY "),
                "the control is admitted"
            );
            drop(client);
            wait_until("the control's admission to be released", || {
                running.state.admission_status() == Ok(counts(0, 0))
            });
        } else {
            assert_eq!(read_all(stream), "");
            running.settle();
            assert_eq!(running.state.admission_status(), Ok(counts(0, 0)));
        }
    }
}
