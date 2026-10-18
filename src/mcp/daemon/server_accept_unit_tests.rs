//! Exercise the production server methods without binding a socket (sandbox compatible).
use super::{fail_next_accepts, AcceptScript};
use crate::mcp::daemon::files::{Claim, DaemonFiles};
use crate::mcp::daemon::server::{Served, Server, IDENTITY_CHECK_INTERVAL};
use crate::mcp::daemon::server_support::{hello_line, options, read_all, send_line, served_key};
use crate::mcp::daemon::test_support::{fixture, scripted, state_with, ManualClock};
use crate::mcp::daemon::wire_support::{Client, PING};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::shared::control::{ControlRequest, DaemonStatus};
use crate::mcp::shared::identity::FsIdentity;
use crate::mcp::RefreshPolicy;
use std::collections::VecDeque;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn with_server(test: impl FnOnce(&mut Server, &ManualClock)) {
    let (repo, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let base = tempfile::tempdir().expect("runtime base");
    let key = served_key();
    let files = match DaemonFiles::claim(&base.path().join("runtime"), &key).expect("claim") {
        Claim::Owned(files) => *files,
        Claim::Taken => panic!("fresh runtime directory"),
    };
    // An unbound Unix socket gives Server a genuine owned listener descriptor, without a
    // filesystem bind. All accepts in these tests are injected; it is never listened on.
    // SAFETY: socket has no pointer arguments and returns a new descriptor on success.
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    assert!(fd >= 0, "socket: {}", io::Error::last_os_error());
    // SAFETY: fd is valid, newly owned, and is transferred exactly once.
    let listener = unsafe { UnixListener::from_raw_fd(fd) };
    let root = std::fs::canonicalize(repo.path()).expect("repository root");
    let served = Served {
        key,
        root_fs: FsIdentity::of(&root).expect("root identity"),
        repo_root: root,
        pid: std::process::id(),
        start_time: 1234,
        build_identity: "build".to_owned(),
    };
    let mut server = Server::new(
        state,
        Arc::new(ToolRegistry::all_v1()),
        files,
        listener,
        served,
        options(64),
    )
    .expect("server");
    server.next_identity_check = server.state.now() + IDENTITY_CHECK_INTERVAL;
    test(&mut server, &clock);
}

#[test]
fn exhaustion_suppresses_accept_until_100ms_and_renews_deadline() {
    for error in [libc::EMFILE, libc::ENFILE] {
        with_server(|server, clock| {
            let script = AcceptScript::install(server);
            fail_next_accepts(&script, error);
            server.accept_all().expect("exhaustion");
            for _ in 0..4 {
                server.accept_all().expect("paused turn");
            }
            clock.advance(Duration::from_millis(99));
            server.accept_all().expect("still paused");
            assert_eq!(script.lock().expect("script").calls, 1);
            clock.advance(Duration::from_millis(1));
            server.accept_all().expect("retry at boundary");
            assert_eq!(script.lock().expect("script").calls, 2);
            clock.advance(Duration::from_millis(99));
            server.accept_all().expect("renewed pause");
            assert_eq!(script.lock().expect("script").calls, 2);
            assert_eq!(
                server.poll_timeout().expect("timeout"),
                Duration::from_millis(1)
            );
        });
    }
}

#[test]
fn exhaustion_logs_once_until_recovery_then_logs_a_new_episode() {
    with_server(|server, clock| {
        let script = AcceptScript::install(server);
        fail_next_accepts(&script, libc::EMFILE);
        server.accept_all().expect("first failure");
        clock.advance(Duration::from_millis(100));
        fail_next_accepts(&script, libc::ENFILE);
        server.accept_all().expect("same episode");
        assert_eq!(script.lock().expect("script").lines.len(), 1);
        assert!(script.lock().expect("script").lines[0].contains("accepting a connection failed"));
        clock.advance(Duration::from_millis(100));
        fail_next_accepts(&script, libc::EAGAIN);
        server.accept_all().expect("backlog empty: episode ends");
        fail_next_accepts(&script, libc::EMFILE);
        server.accept_all().expect("new episode");
        assert_eq!(script.lock().expect("script").lines.len(), 2);
    });
}

#[test]
fn backoff_poll_set_keeps_wakes_and_pending_and_timeout_uses_first_deadline() {
    with_server(|server, clock| {
        let script = AcceptScript::install(server);
        let (_near, far) = UnixStream::pair().expect("pending pair");
        server.enter(far).expect("pending handshake");
        let pending_fd = server.pending[0].stream.as_raw_fd();
        fail_next_accepts(&script, libc::EMFILE);
        server.accept_all().expect("exhaustion");
        let fds: Vec<_> = server.poll_fds().iter().map(|fd| fd.fd).collect();
        assert!(!fds.contains(&server.listener.as_raw_fd()));
        assert!(fds.contains(&server.wakes.as_raw_fd()));
        assert!(fds.contains(&pending_fd));
        assert_eq!(
            server.poll_timeout().expect("retry timeout"),
            Duration::from_millis(100)
        );
        server.pending[0].deadline = server.state.now() + Duration::from_millis(30);
        assert_eq!(
            server.poll_timeout().expect("handshake timeout"),
            Duration::from_millis(30)
        );
        server.pending[0].deadline = server.state.now() + Duration::from_secs(10);
        clock.advance(Duration::from_millis(100));
        assert!(server
            .poll_fds()
            .iter()
            .any(|fd| fd.fd == server.listener.as_raw_fd()));
        assert!(
            server
                .poll_timeout()
                .expect("expired retry is not a zero timeout")
                > Duration::ZERO
        );
    });
}

#[test]
fn recovery_accepts_waiting_connection_and_services_it() {
    with_server(|server, clock| {
        let (near, far) = UnixStream::pair().expect("queued connection");
        let results = Arc::new(Mutex::new(VecDeque::from([
            Err(io::Error::from_raw_os_error(libc::EMFILE)),
            Ok(far),
            Err(io::Error::from(io::ErrorKind::WouldBlock)),
        ])));
        let queue = Arc::clone(&results);
        server.accept_hook = Some(Box::new(move || {
            Some(
                queue
                    .lock()
                    .expect("accept queue")
                    .pop_front()
                    .expect("scripted accept"),
            )
        }));
        server.accept_all().expect("exhaustion");
        send_line(&near, &ControlRequest::Status.encode());
        clock.advance(Duration::from_millis(99));
        server.accept_all().expect("paused");
        assert!(
            server.pending.is_empty(),
            "waiting connection not accepted early"
        );
        clock.advance(Duration::from_millis(1));
        server.accept_all().expect("recovered accept");
        assert!(results.lock().expect("queue").is_empty());
        assert_eq!(server.pending.len(), 1);
        server.advance_handshakes().expect("serve accepted control");
        let reply = read_all(near);
        DaemonStatus::decode(reply.trim_end().as_bytes()).expect("status after recovery");
        assert!(server.poll_timeout().expect("timeout after recovery") > Duration::ZERO);
    });
}

#[test]
fn existing_client_and_pending_control_progress_while_accept_is_paused() {
    with_server(|server, _clock| {
        let (near, far) = UnixStream::pair().expect("client pair");
        server.enter(far).expect("client pending");
        send_line(&near, &hello_line(served_key()));
        server.advance_handshakes().expect("admit client");
        let mut client = Client::new(near);
        assert!(client.recv_line().starts_with("READY "));
        client.handshake();
        let (control, far) = UnixStream::pair().expect("control pair");
        server.enter(far).expect("control pending");
        let script = AcceptScript::install(server);
        fail_next_accepts(&script, libc::EMFILE);
        server.accept_all().expect("exhaustion");
        assert!(client.request(PING)["result"].is_object());
        send_line(&control, &ControlRequest::Status.encode());
        assert!(!server
            .wait(Duration::ZERO)
            .expect("only pending control is readable"));
        server.advance_handshakes().expect("serve pending control");
        let reply = read_all(control);
        let status = DaemonStatus::decode(reply.trim_end().as_bytes()).expect("status");
        assert_eq!(status.admitted, 1);
        server.accept_all().expect("still paused");
        assert_eq!(script.lock().expect("script").calls, 1);
    });
}

#[test]
fn connection_aborted_retries_without_clock_advance() {
    with_server(|server, _clock| {
        let script = AcceptScript::install(server);
        fail_next_accepts(&script, libc::ECONNABORTED);
        server.accept_all().expect("first abort");
        server.accept_all().expect("immediate next turn");
        let script = script.lock().expect("script");
        assert_eq!(script.calls, 2);
        assert_eq!(script.lines.len(), 2);
        assert!(server
            .poll_fds()
            .iter()
            .any(|fd| fd.fd == server.listener.as_raw_fd()));
    });
}
