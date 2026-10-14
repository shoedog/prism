//! The panic boundaries of a daemon connection (spec §3.1 "Fatal", §4.2 "Panic policy", §10):
//! where a panic is caught, and what it ends. Spec §9 test 12 runs the daemon in a process of
//! its own, because what it checks is that the process ends.
use super::super::clock::Clock;
use super::super::coordinator::{DaemonState, Parts};
use super::super::fatal::{Domain, Fatal};
use super::super::test_support::{
    client_settings, fixture, recording_state, scripted, wait_until, FatalLog,
};
use super::super::wire_support::{
    close, connect, connect_with, ended, Client, CALLEES, INIT, INITED, LIST, PING,
};
use super::{serve_admitted, serve_bounded};
use crate::mcp::output::McpToolResult;
use crate::mcp::registry::{ToolAnnotations, ToolContext, ToolDescriptor, ToolRegistry};
use crate::mcp::transport::{InMemoryTransport, ReadOutcome, Transport};
use crate::mcp::{RefreshPolicy, StartupMode};
use serde_json::{json, Value};
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

const PANICKING_TOOL: &str = "panic_in_dispatch";
const PANIC_CALL: &str = r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"panic_in_dispatch","arguments":{}}}"#;
const HANDLER_PANIC: &str = "injected panic in a tool handler";

/// Every tool, and one more whose handler panics. A tool handler runs under the provider
/// slot's read lock, so its panic is a panic in provider dispatch.
fn registry_with_a_panicking_tool() -> Arc<ToolRegistry> {
    let mut registry = ToolRegistry::all_v1();
    registry.register(ToolDescriptor {
        name: PANICKING_TOOL,
        description: String::new(),
        input_schema: json!({ "type": "object" }),
        annotations: ToolAnnotations::read_only("panics"),
        runtime_behavior: None,
        handler: Box::new(
            |_context: &ToolContext<'_>, _arguments: &Value| -> McpToolResult {
                panic!("{HANDLER_PANIC}")
            },
        ),
    });
    Arc::new(registry)
}

/// A panic in a tool handler is caught where it is raised, in provider dispatch, and ends the
/// daemon; under a recording `on_fatal` the call's connection then ends. Rules out a handler
/// panic that ends only its own connection (the boundary of `serve_bounded`, which a running
/// daemon must not reach for a panic under a lock).
#[test]
fn a_panic_in_a_query_ends_the_daemon() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let (state, log) = recording_state(builder);
    let registry = registry_with_a_panicking_tool();
    let (mut a, a_worker) = connect_with(&state, &registry, client_settings(600));
    a.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    a.send(PANIC_CALL);
    a.expect_closed();
    assert!(ended(a_worker).is_err());
    assert_eq!(log.events(), vec![Fatal::Panicked(Domain::Dispatch)]);
}

/// A clock that panics once it is armed. The coordinator reads the clock under the control
/// lock when it starts an attempt.
struct ArmedClock {
    armed: AtomicBool,
}

impl Clock for ArmedClock {
    fn now(&self) -> Instant {
        assert!(
            !self.armed.load(Ordering::SeqCst),
            "injected panic under the control lock"
        );
        Instant::now()
    }
}

/// A panic under the control lock ends the daemon. Rules out a control plane without a
/// boundary of its own, whose panic would close one connection and leave the lock poisoned
/// for every other client.
#[test]
fn a_panic_in_the_control_plane_ends_the_daemon() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let clock = Arc::new(ArmedClock {
        armed: AtomicBool::new(false),
    });
    let mut parts = Parts::production(builder, StartupMode::Lazy);
    let shared: Arc<dyn Clock> = Arc::clone(&clock) as Arc<dyn Clock>;
    parts.clock = shared;
    let log = FatalLog::install(&mut parts);
    let state = DaemonState::from_parts(parts).expect("a lazy daemon state");
    let (mut a, a_worker) = connect(&state, client_settings(600));
    a.handshake();

    clock.armed.store(true, Ordering::SeqCst);
    a.send(CALLEES);
    a.expect_closed();
    assert!(ended(a_worker).is_err());
    assert_eq!(log.events(), vec![Fatal::Panicked(Domain::Control)]);
}

/// A transport that panics while it writes its second response.
struct PanicsOnTheSecondWrite {
    inner: InMemoryTransport,
    writes: usize,
}

impl Transport for PanicsOnTheSecondWrite {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        self.inner.read_message()
    }

    fn write_message(&mut self, v: Value) -> anyhow::Result<()> {
        self.writes += 1;
        assert!(
            self.writes != 2,
            "injected panic while a response is written"
        );
        self.inner.write_message(v)
    }
}

/// Spec §4.2: a panic in a connection's framing or output, outside both domains, closes that
/// connection only. The connection's serve loop returns an error instead of unwinding into the
/// accept loop's worker, nothing ends the daemon, and the next client is served. Rules out a
/// connection without a boundary (its panic would reach whatever runs the worker) and a
/// boundary that ends the daemon for a panic that touched none of its state.
#[test]
fn a_panic_outside_both_domains_closes_that_connection_only() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let (state, log) = recording_state(builder);
    let mut transport = PanicsOnTheSecondWrite {
        inner: InMemoryTransport::new(vec![INIT, INITED, PING]),
        writes: 0,
    };

    let served = serve_bounded(
        &state,
        &ToolRegistry::all_v1(),
        &mut transport,
        &client_settings(600),
    );
    let error = served.expect_err("the connection ends with an error");
    assert!(error.to_string().contains("the daemon goes on"), "{error}");
    assert_eq!(log.events(), vec![]);

    let (mut b, b_worker) = connect(&state, client_settings(600));
    b.handshake();
    let answer = b.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    close(b, b_worker);
    assert_eq!(log.events(), vec![]);
}

/// Where the daemon process of the test below listens.
const DAEMON_SOCKET: &str = "PRISM_MCP_TEST_PANIC_DAEMON_SOCKET";

/// Not a test of its own: the daemon process of
/// `a_panic_in_dispatch_ends_the_daemon_process_and_every_attached_client_sees_its_connection_end`,
/// started as a child with `DAEMON_SOCKET` set. In an ordinary run the variable is unset and
/// this returns at once. It serves every connection it accepts on a thread of its own with the
/// production `on_fatal`, the abort. It exits when its standard input reaches end of file, so
/// that it ends with the test that started it even when that test is killed and cannot reap it.
#[test]
fn a_daemon_process_for_the_panic_test() {
    let Some(socket) = std::env::var_os(DAEMON_SOCKET) else {
        return;
    };
    std::thread::spawn(|| {
        let mut ignored = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut ignored);
        std::process::exit(0);
    });
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let registry = registry_with_a_panicking_tool();
    let listener = UnixListener::bind(&socket).expect("bind the test socket");
    for stream in listener.incoming() {
        let stream = stream.expect("accept a connection");
        let state = Arc::clone(&state);
        let registry = Arc::clone(&registry);
        std::thread::spawn(move || {
            serve_admitted(&state, &registry, stream, &client_settings(600))
        });
    }
}

/// The daemon process. It is killed and reaped when the test ends, however the test ends, so
/// that a run in which the daemon does not abort (a defect, or a mutant) leaves no process
/// behind holding an index.
struct DaemonProcess(Child);

impl Drop for DaemonProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Start the daemon process listening at `socket`, with a pipe to its standard input that it
/// holds open, and a thread that reads its standard error from the start (so that the child
/// never blocks on a full pipe) and returns it when the child ends.
fn start_daemon_process(socket: &Path) -> (DaemonProcess, JoinHandle<String>) {
    let (_crate, module) = module_path!()
        .split_once("::")
        .expect("a module path below the crate");
    let daemon_test = format!("{module}::a_daemon_process_for_the_panic_test");
    let mut daemon = DaemonProcess(
        Command::new(std::env::current_exe().expect("the test binary"))
            .args([
                daemon_test.as_str(),
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(DAEMON_SOCKET, socket)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start the daemon process"),
    );
    let mut stderr = daemon.0.stderr.take().expect("the daemon's standard error");
    let drained = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    (daemon, drained)
}

/// Wait for the daemon process to end and return how it ended.
fn ended_process(daemon: &mut DaemonProcess) -> ExitStatus {
    let mut ended_with = None;
    wait_until("the daemon process to end", || {
        ended_with = daemon.0.try_wait().expect("the daemon process's status");
        ended_with.is_some()
    });
    ended_with.expect("the daemon process ended")
}

/// The daemon process ends by itself when the test that started it is gone: its standard input
/// reaches end of file. Rules out a child that, when its parent is killed without unwinding
/// (and so without `DaemonProcess`'s kill), goes on serving for ever.
#[test]
fn the_daemon_process_ends_when_its_standard_input_is_closed() {
    let dir = tempfile::tempdir().expect("a directory for the socket");
    let socket = dir.path().join("daemon.sock");
    let (mut daemon, drained) = start_daemon_process(&socket);
    let a = attach(&socket);

    drop(daemon.0.stdin.take());
    let ended_with = ended_process(&mut daemon);
    assert!(ended_with.success(), "{ended_with:?}");
    drop(a);
    let _ = drained.join();
}

fn attach(socket: &Path) -> Client {
    let mut stream = None;
    wait_until("the daemon process to listen", || {
        stream = UnixStream::connect(socket).ok();
        stream.is_some()
    });
    let mut client = Client::new(stream.expect("a connection to the daemon process"));
    client.handshake();
    client
}

/// Spec §9 test 12, as far as it can run before the shim exists: an injected panic in
/// dispatch aborts the daemon process, and every attached client sees its connection end,
/// the one whose call panicked and one that was idle. (That the next shim starts a new daemon
/// is slice S5a's.) Rules out a production `on_fatal` that does not end the process, and a
/// panic in dispatch caught by the connection's own boundary.
#[test]
fn a_panic_in_dispatch_ends_the_daemon_process_and_every_attached_client_sees_its_connection_end() {
    let dir = tempfile::tempdir().expect("a directory for the socket");
    let socket = dir.path().join("daemon.sock");
    let (mut daemon, drained) = start_daemon_process(&socket);

    let mut a = attach(&socket);
    let mut b = attach(&socket);
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");
    let listed = b.request(LIST);
    assert!(listed["result"]["tools"].is_array(), "{listed}");

    a.send(PANIC_CALL);
    a.expect_closed();
    b.expect_closed();

    let ended_with = ended_process(&mut daemon);
    assert_eq!(ended_with.signal(), Some(libc::SIGABRT), "{ended_with:?}");
    let stderr = drained.join().expect("the daemon's standard error is read");
    assert!(stderr.contains(HANDLER_PANIC), "{stderr}");
    assert!(
        stderr.contains("prism-mcp daemon: a panic in provider dispatch"),
        "{stderr}"
    );
}
