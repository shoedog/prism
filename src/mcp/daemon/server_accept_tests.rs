//! Descriptor exhaustion affects listener polling only; existing work keeps progressing.
use super::super::server_support::{
    hello_line, options, read_all, send_line, served_key, start, Running,
};
use super::super::test_support::wait_until;
use super::super::wire_support::{Client, PING};
use super::Server;
use crate::mcp::shared::control::{ControlRequest, DaemonStatus};
use std::io;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct AcceptScript {
    error: Option<i32>,
    calls: usize,
    lines: Vec<String>,
}

impl AcceptScript {
    fn install(server: &mut Server) -> Arc<Mutex<Self>> {
        let script = Arc::new(Mutex::new(Self::default()));
        let accepts = Arc::clone(&script);
        server.accept_hook = Some(Box::new(move || {
            let mut script = accepts.lock().expect("accept script");
            script.calls += 1;
            script
                .error
                .map(|error| Err(io::Error::from_raw_os_error(error)))
        }));
        let logs = Arc::clone(&script);
        server.accept_log = Some(Box::new(move |line| {
            logs.lock()
                .expect("accept script")
                .lines
                .push(line.to_owned());
        }));
        script
    }
}

fn scripted_server() -> (Running, Arc<Mutex<AcceptScript>>) {
    let mut script = None;
    let running = start(options(64), |server| {
        script = Some(AcceptScript::install(server));
    });
    (running, script.expect("script installed"))
}

fn fail_next_accepts(script: &Mutex<AcceptScript>, error: i32) -> usize {
    let mut script = script.lock().expect("accept script");
    script.error = Some(error);
    script.calls
}

fn wait_for_error(script: &Mutex<AcceptScript>, before: usize) {
    wait_until("the injected accept error to be logged", || {
        let script = script.lock().expect("accept script");
        script.calls > before && !script.lines.is_empty()
    });
}

fn advance(running: &Running, millis: u64) {
    running.clock.advance(Duration::from_millis(millis));
    running.settle();
}

#[path = "server_accept_unit_tests.rs"]
mod socket_free;

#[test]
fn descriptor_exhaustion_retries_only_at_deadline_and_logs_once_per_episode() {
    for error in [libc::EMFILE, libc::ENFILE] {
        let (running, script) = scripted_server();
        let before = fail_next_accepts(&script, error);
        let _waiting = running.connect();
        wait_for_error(&script, before);
        for _ in 0..4 {
            running.settle();
        }
        advance(&running, 99);
        {
            let script = script.lock().expect("accept script");
            assert_eq!(script.calls, before + 1, "no retry before 100 ms");
            assert_eq!(script.lines.len(), 1, "one line during the episode");
        }
        advance(&running, 1);
        {
            let script = script.lock().expect("accept script");
            assert_eq!(script.calls, before + 2, "retry exactly at 100 ms");
            assert_eq!(
                script.lines.len(),
                1,
                "persistent exhaustion is one episode"
            );
        }
        advance(&running, 99);
        assert_eq!(script.lock().expect("accept script").calls, before + 2);
    }
}

#[test]
fn listener_recovers_at_deadline_and_a_new_episode_is_logged() {
    let (running, script) = scripted_server();
    let before = fail_next_accepts(&script, libc::EMFILE);
    let waiting = running.connect();
    send_line(&waiting, &hello_line(served_key()));
    wait_for_error(&script, before);
    script.lock().expect("accept script").error = None;
    running.settle();
    assert_eq!(script.lock().expect("accept script").calls, before + 1);
    advance(&running, 100);
    let mut client = Client::new(waiting);
    assert!(client.recv_line().starts_with("READY "));
    client.handshake();
    assert!(client.request(PING)["result"].is_object());

    let before = fail_next_accepts(&script, libc::ENFILE);
    let _next = running.connect();
    wait_until("a new exhaustion episode to be logged", || {
        script.lock().expect("accept script").lines.len() == 2
    });
    running.settle();
    assert_eq!(script.lock().expect("accept script").calls, before + 1);
}

#[test]
fn existing_client_and_pending_control_are_served_during_backoff() {
    let (running, script) = scripted_server();
    let mut client = running.admitted();
    let control = running.connect();
    running.settle(); // Control is already accepted, but has not sent its line.
    let before = fail_next_accepts(&script, libc::EMFILE);
    let _waiting = running.connect();
    wait_for_error(&script, before);
    assert!(client.request(PING)["result"].is_object());
    send_line(&control, &ControlRequest::Status.encode());
    let reply = read_all(control);
    let status = DaemonStatus::decode(reply.trim_end().as_bytes()).expect("status reply");
    assert_eq!(status.admitted, 1);
    running.settle();
    assert_eq!(script.lock().expect("accept script").calls, before + 1);
}

#[test]
fn connection_aborted_keeps_immediate_retry_behavior() {
    let (running, script) = scripted_server();
    let before = fail_next_accepts(&script, libc::ECONNABORTED);
    let waiting = running.connect();
    wait_for_error(&script, before);
    script.lock().expect("accept script").error = None;
    // No clock advance: the still-readable listener must be retried now.
    send_line(&waiting, &hello_line(served_key()));
    let mut client = Client::new(waiting);
    assert!(client.recv_line().starts_with("READY "));
    assert!(script.lock().expect("accept script").calls > before + 1);
}

#[test]
fn backoff_excludes_only_listener_and_bounds_poll_timeout() {
    let running = start(options(64), |server| {
        let script = AcceptScript::install(server);
        fail_next_accepts(&script, libc::EMFILE);
        assert!(server.check().expect("check").is_none());
        let _waiting = UnixStream::connect(server.files.paths().socket()).expect("queued client");
        server.accept_all().expect("injected exhaustion");
        assert_eq!(
            server.poll_timeout().expect("timeout"),
            Duration::from_millis(100)
        );
        assert!(!server.wait(Duration::ZERO).expect("listener omitted"));
        server.waker.wake();
        assert!(!server
            .wait(Duration::ZERO)
            .expect("wake is not listener readiness"));
        server.empty_wakes();
        script.lock().expect("accept script").error = None;
    });
    running.settle();
}
