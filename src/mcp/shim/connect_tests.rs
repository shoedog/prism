//! Tests for one attempt to reach a daemon (spec §4.1 step 5, §4.4, §10): what is sent, and how
//! each answer is sorted into reached, transient or refused. Each fake daemon is a listener of
//! the test's own; nothing here sleeps to order two events.
use super::super::shim_support::{
    another_user, hello_line, key, other_key, read_line, read_to_end, ready_line, runtime,
    this_user, HANG,
};
use super::*;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::Reply;
use crate::mcp::shared::runtime_dir::current_uid;
use std::os::unix::net::UnixListener;
use std::sync::mpsc;
use std::thread::JoinHandle;

const LONG: Duration = Duration::from_secs(60);

/// Attempt to reach the daemon of `paths` with this test's `HELLO`, on a thread of its own, and
/// give up after [`HANG`]: a regression that blocks must fail the test, not hang it.
fn attempt(
    paths: &DaemonPaths,
    within: Duration,
    peer_uid: PeerUid,
    hook: Option<Hook>,
) -> Reached {
    let socket = paths.socket().to_path_buf();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let hello = hello_line(key());
        let target = Target {
            socket: &socket,
            key: key(),
            hello: &hello,
            deadline: Instant::now() + within,
            peer_uid,
            uid: current_uid(),
            hook: hook.as_ref(),
        };
        let _ = tx.send(reach(&target));
    });
    rx.recv_timeout(HANG)
        .expect("the attempt ends within the hang limit")
}

/// A daemon that takes one connection and runs `answer` on it; the thread returns what
/// `answer` returns.
fn serve_one<T: Send + 'static>(
    paths: &DaemonPaths,
    answer: impl FnOnce(UnixStream) -> T + Send + 'static,
) -> JoinHandle<T> {
    let listener = UnixListener::bind(paths.socket()).expect("bind the fake daemon's socket");
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        answer(stream)
    })
}

/// Reads the `HELLO`, answers `reply`, and keeps the connection until the shim closes it, as a
/// daemon does.
fn answering(reply: String) -> impl FnOnce(UnixStream) -> String {
    move |mut stream| {
        let hello = read_line(&stream);
        let _ = stream.write_all(reply.as_bytes());
        let _ = read_to_end(&stream);
        hello
    }
}

/// Spec §4.4: the shim sends exactly its `HELLO` line and is admitted by `READY` for its key; the
/// stream it gets has no timeout left from the preamble. Rules out timeouts that outlive the
/// preamble (a quiet MCP session would end at the first silence longer than the deadline).
#[test]
fn a_ready_for_this_key_is_reached_and_the_stream_has_no_timeouts() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering(ready_line(key(), 77)));
    let reached = attempt(&paths, LONG, this_user, None);
    let Reached::Ready(stream, ready) = reached else {
        panic!("not reached")
    };
    assert_eq!(ready.pid, 77);
    assert_eq!(stream.read_timeout().expect("read timeout"), None);
    assert_eq!(stream.write_timeout().expect("write timeout"), None);
    drop(stream);
    assert_eq!(daemon.join().expect("the daemon"), hello_line(key()));
}

/// Spec §4.1 step 5: `RETIRING`, and a connection that ends before any reply, are transient:
/// arbitration starts over.
#[test]
fn retiring_and_a_close_before_any_reply_are_transient() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering(Reply::Retiring.encode()));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Transient(Transient::Retiring)
    ));
    daemon.join().expect("the daemon");
    std::fs::remove_file(paths.socket()).expect("remove the socket");

    let daemon = serve_one(&paths, |stream| drop(read_line(&stream)));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Transient(Transient::Closed)
    ));
    daemon.join().expect("the daemon");
}

/// Spec §4.1 step 5: no socket (`ENOENT`) and a socket nobody listens on (`ECONNREFUSED`) are
/// transient.
#[test]
fn nothing_listening_is_transient() {
    let (_base, paths) = runtime();
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Transient(Transient::Connect(_))
    ));
    drop(UnixListener::bind(paths.socket()).expect("a stale socket"));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Transient(Transient::Connect(_))
    ));
}

/// Spec §10: `BUSY` and `MISMATCH` are refusals the shim fails closed on, and the reply is read
/// even when the daemon closed before the `HELLO` could be written (S4's interface: the daemon
/// answers `BUSY` at accept and closes; the shim's write then fails with `EPIPE`). Rules out
/// taking a failed write as the end of the attempt, which would retry to the deadline instead of
/// failing closed.
#[test]
fn busy_and_mismatch_are_refused_even_when_the_hello_cannot_be_written() {
    let (_base, paths) = runtime();
    let (closed_tx, closed_rx) = mpsc::channel::<()>();
    let closed_rx = std::sync::Mutex::new(closed_rx);
    let daemon = serve_one(&paths, move |mut stream| {
        let _ = stream.write_all(Reply::Busy { max_clients: 64 }.encode().as_bytes());
        drop(stream);
        let _ = closed_tx.send(());
    });
    // The shim waits, after connecting, until the daemon has answered and closed.
    let hook: Hook = Arc::new(move |point| {
        if point == Point::AfterConnect {
            let _ = closed_rx.lock().expect("rx").recv_timeout(HANG);
        }
    });
    assert!(matches!(
        attempt(&paths, LONG, this_user, Some(hook)),
        Reached::Refused(Refusal::Busy { max_clients: 64 })
    ));
    daemon.join().expect("the daemon");
    std::fs::remove_file(paths.socket()).expect("remove the socket");

    let daemon = serve_one(
        &paths,
        answering(
            Reply::Mismatch {
                expected: other_key(),
            }
            .encode(),
        ),
    );
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Refused(Refusal::Mismatch { expected }) if expected == other_key()
    ));
    daemon.join().expect("the daemon");
}

/// The lane README's S2b note: the shim compares `READY`'s identity with its own key, so a
/// listener that answers `READY` for another identity is never pumped to.
#[test]
fn a_ready_for_another_identity_is_refused() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering(ready_line(other_key(), 77)));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Refused(Refusal::ReadyForAnother { identity }) if identity == other_key()
    ));
    daemon.join().expect("the daemon");
}

/// The lane's S5a design item: a socket whose peer is another user is refused before anything
/// is sent to it, even when it would answer `READY`. Rules out a shim without the check, and one
/// that checks after sending its `HELLO`.
#[test]
fn a_peer_of_another_user_is_refused_before_anything_is_sent() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, |mut stream| {
        let _ = stream.write_all(ready_line(key(), 77).as_bytes());
        read_to_end(&stream)
    });
    assert!(matches!(
        attempt(&paths, LONG, another_user, None),
        Reached::Refused(Refusal::ForeignPeer { uid }) if uid == current_uid().wrapping_add(1)
    ));
    assert_eq!(daemon.join().expect("the daemon"), Vec::<u8>::new());
}

/// Spec §4.1 steps 4 and 6: a daemon that accepts and does not answer, or answers part of a line
/// and stops, holds the attempt only until the deadline. Rules out reads with no bound, and a
/// bound on each silence instead of the line.
#[test]
fn a_daemon_that_does_not_finish_its_reply_is_no_reply_at_the_deadline() {
    let (_base, paths) = runtime();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let daemon = serve_one(&paths, move |mut stream| {
        let _ = read_line(&stream);
        let _ = stream.write_all(b"RETIR");
        let _ = stop_rx.recv_timeout(HANG);
    });
    assert!(matches!(
        attempt(&paths, Duration::from_millis(300), this_user, None),
        Reached::Transient(Transient::NoReply)
    ));
    let _ = stop_tx.send(());
    daemon.join().expect("the daemon");
    std::fs::remove_file(paths.socket()).expect("remove the socket");

    // A listener that never accepts: the connection waits in its backlog.
    let _listener = UnixListener::bind(paths.socket()).expect("bind");
    assert!(matches!(
        attempt(&paths, Duration::from_millis(300), this_user, None),
        Reached::Transient(Transient::NoReply)
    ));
}

/// Review round 1, sol 1: the deadline bounds the whole reply, not each silence. A daemon that
/// drips a whole `RETIRING` line one byte every 30 ms (about 360 ms in all) never lets a read
/// time out, yet the reply ends after a 150 ms deadline: the attempt is `NoReply`, not
/// `Retiring`. And one that drips bytes without end (no newline, one every 20 ms, 80 s before the
/// 4 KiB limit) holds the attempt only until its deadline, well inside the hang limit. The drips
/// order nothing a right implementation depends on.
#[test]
fn a_reply_dripped_past_the_deadline_is_no_reply() {
    for line in [Some(Reply::Retiring.encode()), None] {
        let (_base, paths) = runtime();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let daemon = serve_one(&paths, move |mut stream| {
            let _ = read_line(&stream);
            let bytes: Vec<u8> = match &line {
                Some(line) => line.clone().into_bytes(),
                None => vec![b'R'; 4096],
            };
            let gap = Duration::from_millis(if line.is_some() { 30 } else { 20 });
            for byte in bytes {
                if stream.write_all(&[byte]).is_err() || stop_rx.recv_timeout(gap).is_ok() {
                    return;
                }
            }
        });
        assert!(matches!(
            attempt(&paths, Duration::from_millis(150), this_user, None),
            Reached::Transient(Transient::NoReply)
        ));
        let _ = stop_tx.send(());
        daemon.join().expect("the daemon");
    }
}

/// Review round 1, sol 1: a line whose read ends at the deadline is not accepted. The hook,
/// called after a whole line is read, holds the shim until the deadline has passed (the read
/// itself was quick); the attempt is then `NoReply`, not `Retiring`.
#[test]
fn a_reply_whose_read_ends_at_the_deadline_is_not_accepted() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering(Reply::Retiring.encode()));
    let deadline = Instant::now() + Duration::from_millis(300);
    let hook: Hook = Arc::new(move |point| {
        if point == Point::ReplyRead {
            while Instant::now() <= deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    });
    let hello = hello_line(key());
    let target = Target {
        socket: paths.socket(),
        key: key(),
        hello: &hello,
        deadline,
        peer_uid: this_user,
        uid: current_uid(),
        hook: Some(&hook),
    };
    assert!(matches!(
        reach(&target),
        Reached::Transient(Transient::NoReply)
    ));
    daemon.join().expect("the daemon");
}

/// A reply this build cannot read is a refusal, not a retry: the daemon at this path will not
/// start making sense.
#[test]
fn an_unreadable_reply_is_refused() {
    let (_base, paths) = runtime();
    let daemon = serve_one(&paths, answering("READY {}\n".to_string()));
    assert!(matches!(
        attempt(&paths, LONG, this_user, None),
        Reached::Refused(Refusal::BadReply(_))
    ));
    daemon.join().expect("the daemon");
}

/// Review round 2 (Opus R2-S1): a read that starts after the deadline is refused even when a byte
/// is waiting. A read that starts before the deadline can return a byte that lands within one
/// timer tick after it (Linux rounds `SO_RCVTIMEO` up to whole ticks), or the shim can be
/// descheduled between two partial reads; the next read then starts late. Rules out a read that
/// falls back to a fixed bound once no time is left (mutant N1).
#[test]
fn a_read_after_the_deadline_is_refused_even_with_a_byte_waiting() {
    use std::io::{Read as _, Write as _};
    let (mut ours, mut theirs) = UnixStream::pair().expect("a pair");
    theirs.write_all(b"R").expect("a byte waiting");
    let deadline = Instant::now();
    std::thread::sleep(Duration::from_millis(2));
    let mut source = Deadlined {
        stream: &mut ours,
        deadline,
    };
    let mut byte = [0u8; 1];
    let refused = source
        .read(&mut byte)
        .expect_err("a read after the deadline is refused");
    assert_eq!(refused.kind(), std::io::ErrorKind::TimedOut);
}

/// Review round 2 (Opus R2-S2): the connected stream is close-on-exec, so a daemon this shim (or
/// any process of the test binary) spawns does not inherit it, and it is returned blocking. On
/// Linux the socket is created with both flags at once, as `std` does; macOS sets them after.
#[test]
fn a_connected_stream_is_close_on_exec_and_blocking() {
    use std::os::fd::AsRawFd as _;
    let (_base, paths) = runtime();
    let _listener = UnixListener::bind(paths.socket()).expect("bind");
    let stream = connect_within(paths.socket(), Instant::now() + LONG).expect("connected");
    // SAFETY: `F_GETFD` on an open descriptor reads its flags and touches no memory.
    let fd_flags = unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFD) };
    // SAFETY: as above, with `F_GETFL`.
    let status = unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFL) };
    assert!(fd_flags >= 0 && status >= 0, "fcntl failed");
    assert_ne!(fd_flags & libc::FD_CLOEXEC, 0, "close-on-exec");
    assert_eq!(status & libc::O_NONBLOCK, 0, "blocking");
}
