//! Tests for the byte pipe after `READY` (spec §4.1 steps 8 and 9). The daemon and the client
//! are each the far end of a socket pair.
use super::super::shim_support::HANG;
use super::*;
use std::sync::mpsc;

/// Run the pump on a thread; return the client's input end, the daemon's end, and a receiver of
/// how the pump ended with what it wrote to the client.
fn running() -> (UnixStream, UnixStream, mpsc::Receiver<(PumpEnd, Vec<u8>)>) {
    let (client, input) = UnixStream::pair().expect("the client's input");
    let (daemon, shim_side) = UnixStream::pair().expect("the daemon's connection");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let end = pump(input, &mut output, shim_side);
        let _ = tx.send((end, output));
    });
    (client, daemon, rx)
}

/// Spec §4.1 step 8: bytes go both ways unchanged, a partial line and a frame larger than the
/// pump's buffer included; when the client closes its input the daemon sees the end of its
/// input, its last response still reaches the client, and the session ends cleanly.
#[test]
fn bytes_pass_unchanged_and_the_client_closing_ends_the_session_cleanly() {
    let (mut client, mut daemon, ended) = running();
    let big: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    // The daemon reads while the client writes: a socket holds far less than the frame.
    daemon
        .set_read_timeout(Some(HANG))
        .expect("a bound on the test");
    let reader = {
        let mut daemon = daemon.try_clone().expect("the daemon's end");
        std::thread::spawn(move || {
            let mut received = Vec::new();
            let saw_end = daemon.read_to_end(&mut received).is_ok();
            (saw_end, received)
        })
    };
    client.write_all(b"{\"partial").expect("write");
    client.write_all(&big).expect("write");
    client
        .shutdown(Shutdown::Write)
        .expect("the client closes its input");
    let (saw_end, received) = reader.join().expect("the daemon's reader");
    assert!(
        saw_end,
        "the daemon saw the end of its input, not a timeout"
    );
    assert_eq!(&received[..9], b"{\"partial");
    assert_eq!(&received[9..], &big[..]);
    daemon.write_all(b"the last response\n").expect("write");
    drop(daemon);
    let (end, output) = ended.recv_timeout(HANG).expect("the pump ends");
    assert!(matches!(end, PumpEnd::ClientClosed), "{end:?}");
    assert_eq!(output, b"the last response\n");
}

/// Spec §4.1 steps 8 and 9 and §9 test 11: a daemon that closes the connection while the client
/// is still there ends the session as an error; what it sent before is delivered.
#[test]
fn a_daemon_that_closes_while_the_client_is_there_ends_the_session_as_an_error() {
    let (_client, mut daemon, ended) = running();
    daemon.write_all(b"one response\n").expect("write");
    drop(daemon);
    let (end, output) = ended.recv_timeout(HANG).expect("the pump ends");
    assert!(matches!(end, PumpEnd::DaemonClosed), "{end:?}");
    assert_eq!(output, b"one response\n");
}
