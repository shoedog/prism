//! A daemon's accept loop on a thread of the test, with real files in a runtime directory of
//! the test's own, a manual clock, and a raw client that speaks the preamble. This is what
//! stands in for a daemon process and a shim until the shim exists (S5a).
//!
//! No test sleeps to order events: the loop is woken (`Waker::wake`) after the clock is moved,
//! and a test that needs the loop to have looked at the new time waits for two more turns of it
//! to begin (`Running::settle`).
use super::control::StopReason;
use super::coordinator::DaemonState;
use super::files::{Claim, DaemonFiles};
use super::server::{ServeOptions, Served, Server, Waker, HANDSHAKE_TIMEOUT};
use super::test_support::{
    advance, client_settings, fixture, scripted, state_with, wait_until, HANG,
};
use super::test_support::{Gate, ManualClock};
use super::wire_support::Client;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::shared::identity::{FsIdentity, IdentityKey};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::Hello;
use crate::mcp::RefreshPolicy;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub(super) const TTL_SECS: u64 = 900;
/// The production handshake timeout, in whole seconds.
pub(super) const HANDSHAKE_SECS: u64 = HANDSHAKE_TIMEOUT.as_secs();

pub(super) fn options(max_clients: u32) -> ServeOptions {
    ServeOptions {
        max_clients,
        idle_ttl: Duration::from_secs(TTL_SECS),
        handshake_timeout: HANDSHAKE_TIMEOUT,
    }
}

/// The key the test daemon serves.
pub(super) fn served_key() -> IdentityKey {
    IdentityKey::from_hex(&"cd".repeat(32)).expect("a key")
}

pub(super) fn other_key() -> IdentityKey {
    IdentityKey::from_hex(&"ef".repeat(32)).expect("a key")
}

/// A runtime directory of the test's own under `/tmp` (short enough for a socket path).
pub(super) fn runtime_base() -> (tempfile::TempDir, PathBuf) {
    let base = tempfile::Builder::new()
        .prefix("s4")
        .tempdir_in("/tmp")
        .expect("a temporary directory under /tmp");
    let dir = base.path().join("prism-mcpd");
    (base, dir)
}

/// Where the loop is, for a test that holds it at the top of a turn.
#[derive(Default)]
pub(super) struct Turns {
    count: AtomicU64,
    hold: Mutex<Option<Arc<Gate>>>,
    /// How many times the loop has stopped at a gate.
    held: AtomicU64,
}

/// A daemon's accept loop running on a thread of the test.
pub(super) struct Running {
    pub(super) state: Arc<DaemonState>,
    pub(super) clock: Arc<ManualClock>,
    pub(super) waker: Waker,
    pub(super) paths: DaemonPaths,
    pub(super) runtime_dir: PathBuf,
    pub(super) repo: PathBuf,
    pub(super) turns: Arc<Turns>,
    pub(super) handle: Option<JoinHandle<anyhow::Result<StopReason>>>,
    pub(super) _repo_dir: tempfile::TempDir,
    pub(super) _base: tempfile::TempDir,
}

/// Start a lazy daemon's loop with `options`; `configure` may replace the peer check.
pub(super) fn start(options: ServeOptions, configure: impl FnOnce(&mut Server)) -> Running {
    let (repo_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let clock = ManualClock::new();
    let state = state_with(builder, &clock);
    let (base, runtime_dir) = runtime_base();
    start_with(
        state,
        clock,
        repo_dir,
        base,
        runtime_dir,
        options,
        configure,
    )
}

pub(super) fn start_with(
    state: Arc<DaemonState>,
    clock: Arc<ManualClock>,
    repo_dir: tempfile::TempDir,
    base: tempfile::TempDir,
    runtime_dir: PathBuf,
    options: ServeOptions,
    configure: impl FnOnce(&mut Server),
) -> Running {
    let key = served_key();
    let mut files = match DaemonFiles::claim(&runtime_dir, &key).expect("claim the files") {
        Claim::Owned(files) => *files,
        Claim::Taken => panic!("the key is taken"),
    };
    let listener = files.bind().expect("bind");
    files
        .write_pid(std::process::id(), 1234, "build")
        .expect("pid");
    let paths = files.paths().clone();
    let repo = std::fs::canonicalize(repo_dir.path()).expect("canonical repository root");
    let served = Served {
        key,
        repo_root: repo.clone(),
        root_fs: FsIdentity::of(&repo).expect("the root's identity"),
        pid: std::process::id(),
        start_time: 1234,
        build_identity: "build".to_string(),
    };
    let mut server = Server::new(
        Arc::clone(&state),
        Arc::new(ToolRegistry::all_v1()),
        files,
        listener,
        served,
        options,
    )
    .expect("a server");
    let turns = Arc::new(Turns::default());
    let seen = Arc::clone(&turns);
    server.set_turn_hook(Arc::new(move || {
        seen.count.fetch_add(1, Ordering::SeqCst);
        let hold = seen.hold.lock().expect("hold").take();
        if let Some(gate) = hold {
            seen.held.fetch_add(1, Ordering::SeqCst);
            let _opened = gate.pass();
        }
    }));
    configure(&mut server);
    let waker = server.waker();
    let handle = std::thread::spawn(move || server.run());
    Running {
        state,
        clock,
        waker,
        paths,
        runtime_dir,
        repo,
        turns,
        handle: Some(handle),
        _repo_dir: repo_dir,
        _base: base,
    }
}

impl Running {
    fn turn_count(&self) -> u64 {
        self.turns.count.load(Ordering::SeqCst)
    }

    /// Wake the loop and wait until it has begun one more turn.
    fn one_turn(&self) {
        let before = self.turn_count();
        self.waker.wake();
        wait_until("the accept loop to turn", || self.turn_count() > before);
    }

    /// Make sure the loop has looked at everything that happened before this call: after two
    /// more turns have begun, the first of them has run its checks and handled every readable
    /// connection.
    pub(super) fn settle(&self) {
        self.one_turn();
        self.one_turn();
    }

    /// Move the daemon's clock and let the loop see it.
    pub(super) fn advance(&self, secs: u64) {
        advance(&self.clock, &self.state, secs);
        self.settle();
    }

    /// Move the daemon's clock and wake the loop, without waiting for it (it may stop).
    pub(super) fn advance_and_wake(&self, secs: u64) {
        advance(&self.clock, &self.state, secs);
        self.waker.wake();
    }

    /// Hold the loop at the top of its next turn until the returned gate is opened.
    pub(super) fn hold(&self) -> Arc<Gate> {
        let (gate, holds) = self.arm_hold();
        self.waker.wake();
        self.wait_held(holds);
        gate
    }

    /// Make the loop stop at the top of its next turn, without waking it. Returns the gate and
    /// what `wait_held` waits for.
    pub(super) fn arm_hold(&self) -> (Arc<Gate>, u64) {
        let gate = Gate::closed();
        let holds = self.turns.held.load(Ordering::SeqCst) + 1;
        *self.turns.hold.lock().expect("hold") = Some(Arc::clone(&gate));
        (gate, holds)
    }

    /// Wait until the loop has stopped at the gate `arm_hold` set.
    pub(super) fn wait_held(&self, holds: u64) {
        wait_until("the accept loop to be held", || {
            self.turns.held.load(Ordering::SeqCst) >= holds
        });
    }

    /// Wait for the loop to end and return why it drained.
    pub(super) fn stopped(&mut self) -> StopReason {
        let handle = self.handle.take().expect("the loop is running");
        wait_until("the accept loop to drain", || handle.is_finished());
        handle
            .join()
            .expect("the accept loop must not panic")
            .expect("the accept loop drains without an error")
    }

    pub(super) fn is_running(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
    }

    pub(super) fn connect(&self) -> UnixStream {
        UnixStream::connect(self.paths.socket()).expect("connect to the daemon's socket")
    }

    /// Connect, send a `HELLO` for `key`, and return everything the daemon sends until it
    /// closes the connection. The daemon can answer and close before the `HELLO` is written
    /// (`BUSY`, a peer of another user), so a failed write is not an error: the reply is read
    /// anyway, as a shim must read it (review round 1).
    pub(super) fn refusal(&self, key: IdentityKey) -> String {
        let stream = self.connect();
        let _ = stream.set_read_timeout(Some(HANG));
        let _ = (&stream).write_all(hello_line(key).as_bytes());
        read_all(stream)
    }

    /// An admitted, initialized MCP client that waits up to 600 s for the index.
    pub(super) fn admitted(&self) -> Client {
        self.admitted_waiting(600)
    }

    /// An admitted, initialized MCP client that waits up to `wait_secs` for the index.
    pub(super) fn admitted_waiting(&self, wait_secs: u64) -> Client {
        let mut client = Client::new(self.connect());
        client.send(hello_line_waiting(served_key(), wait_secs).trim_end());
        let reply = client.recv_line();
        assert!(reply.starts_with("READY "), "{reply}");
        client.handshake();
        client
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // A test that failed with the loop still running stops it, so that no thread is left
        // polling a directory that is about to be removed.
        if let Some(handle) = self.handle.take() {
            self.waker.request_stop();
            let _ = handle.join();
        }
    }
}

pub(super) fn hello_line(key: IdentityKey) -> String {
    hello_line_waiting(key, 600)
}

pub(super) fn hello_line_waiting(key: IdentityKey, wait_secs: u64) -> String {
    Hello {
        identity: key,
        settings: client_settings(wait_secs),
    }
    .encode()
}

pub(super) fn send_line(stream: &UnixStream, line: &str) {
    (&*stream)
        .write_all(line.as_bytes())
        .expect("write to the daemon");
}

/// Everything the daemon sends on `stream` until it closes it. Use this, not `Client`, on a
/// connection the daemon may already have closed: on macOS setting a read timeout on a socket
/// whose peer has closed fails with `EINVAL`, so a failure to set it is taken to mean that.
///
/// A close is end of stream or, on Linux, a reset: Linux reports `ECONNRESET` to a peer whose
/// connection was closed with bytes of that peer's still unread (review round 1), after the
/// bytes that were sent. Every byte received before it is kept and returned; any other error,
/// a timeout included, fails the test.
pub(super) fn read_all(mut stream: UnixStream) -> String {
    let _ = stream.set_read_timeout(Some(HANG));
    let mut received = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => received.extend_from_slice(buffer.get(..n).unwrap_or_default()),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) if error.kind() == ErrorKind::ConnectionReset => break,
            Err(error) => panic!("the daemon closes the connection within the hang limit: {error}"),
        }
    }
    String::from_utf8(received).expect("the daemon sends UTF-8")
}

/// Whether the daemon has sent anything on `stream` yet. Does not wait.
pub(super) fn answered(stream: &UnixStream) -> bool {
    stream
        .set_nonblocking(true)
        .expect("make the client's socket non-blocking");
    let mut byte = [0u8; 1];
    let answered = match (&*stream).read(&mut byte) {
        Ok(n) => n > 0,
        Err(error) if error.kind() == ErrorKind::WouldBlock => false,
        Err(error) => panic!("read the daemon's answer: {error}"),
    };
    stream
        .set_nonblocking(false)
        .expect("make the client's socket blocking again");
    answered
}

/// Whether nothing named `path` exists.
pub(super) fn gone(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_err()
}
