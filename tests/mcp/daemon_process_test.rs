//! `prism-mcp daemon` as a process (spec §3.1, §4.2–§4.4; the daemon's halves of §9 tests 2, 3,
//! 4, 8 and 15, which the shim's process tests complete; they share this file's helpers). Each test runs the real binary with a
//! runtime directory of its own and speaks the preamble to it as a shim will.
#![cfg(feature = "mcp")]

use prism::mcp::concise_shape::ConciseShapeMode;
use prism::mcp::output::StructuredContentMode;
use prism::mcp::settings::{ClientSettings, ConnectionSettings};
use prism::mcp::shared::identity::{DaemonIdentity, IdentityKey};
use prism::mcp::shared::paths::DaemonPaths;
use prism::mcp::shared::preamble::{BuildState, Hello, Reply};
use prism::mcp::shared::runtime_dir::{validate_runtime_dir, RUNTIME_DIR_ENV, RUNTIME_DIR_NAME};
use prism::mcp::{CacheMode, ServerConfig};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(crate) const HANG: Duration = Duration::from_secs(60);
pub(crate) const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
pub(crate) const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
pub(crate) const CALLEES: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#;

pub(crate) fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let give_up = Instant::now() + HANG;
    while !condition() {
        assert!(Instant::now() < give_up, "gave up waiting for: {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// A repository, a cache base and a runtime base of the test's own, and the key and paths a
/// daemon for them has.
pub(crate) struct Setup {
    pub(crate) repo: tempfile::TempDir,
    pub(crate) cache: tempfile::TempDir,
    pub(crate) base: tempfile::TempDir,
    pub(crate) key: IdentityKey,
    pub(crate) paths: DaemonPaths,
}

pub(crate) fn setup() -> Setup {
    let repo = tempfile::tempdir().expect("a repository");
    std::fs::write(repo.path().join("util.py"), "def helper():\n    return 1\n")
        .expect("write util.py");
    std::fs::write(
        repo.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .expect("write main.py");
    let cache = tempfile::tempdir().expect("a cache base");
    let base = tempfile::Builder::new()
        .prefix("s4")
        .tempdir_in("/tmp")
        .expect("a runtime base under /tmp");
    let mut cfg = ServerConfig::new(repo.path().to_path_buf());
    cfg.cache = CacheMode::Dir(cache.path().to_path_buf());
    let key = DaemonIdentity::resolve(&cfg).expect("an identity").key();
    let paths = DaemonPaths::new(&base.path().join(RUNTIME_DIR_NAME), &key).expect("paths");
    Setup {
        repo,
        cache,
        base,
        key,
        paths,
    }
}

/// A daemon process, killed and reaped when the test ends however it ends.
pub(crate) struct Daemon {
    child: Child,
    stderr: Option<JoinHandle<String>>,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Daemon {
    /// Start a daemon whose standard error the test reads.
    pub(crate) fn start(setup: &Setup, extra: &[&str]) -> Self {
        let mut daemon = Self::spawn(setup, extra, Stdio::piped());
        let mut stderr = daemon
            .child
            .stderr
            .take()
            .expect("the daemon's standard error");
        daemon.stderr = Some(std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        }));
        daemon
    }

    /// Start a daemon whose standard error is `log`, as the spawning shim points it at the
    /// key's log file.
    fn start_logging_to(setup: &Setup, extra: &[&str], log: File) -> Self {
        Self::spawn(setup, extra, Stdio::from(log))
    }

    /// The process, held by a `Daemon` from the moment it exists, so that it is killed and
    /// reaped on every path (review round 1, sol S6).
    fn spawn(setup: &Setup, extra: &[&str], stderr: Stdio) -> Self {
        let child = Command::new(env!("CARGO_BIN_EXE_prism-mcp"))
            .arg("daemon")
            .arg("--repo")
            .arg(setup.repo.path())
            .arg("--cache-dir")
            .arg(setup.cache.path())
            .args(extra)
            .env(RUNTIME_DIR_ENV, setup.base.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()
            .expect("start the daemon");
        Self {
            child,
            stderr: None,
        }
    }

    pub(crate) fn pid(&self) -> u32 {
        self.child.id()
    }

    pub(crate) fn exited(&mut self) -> Option<ExitStatus> {
        self.child.try_wait().expect("the daemon's status")
    }

    /// Wait for the process to end; return how it ended and its standard error.
    pub(crate) fn ended(&mut self) -> (ExitStatus, String) {
        let mut status = None;
        wait_until("the daemon process to end", || {
            status = self.exited();
            status.is_some()
        });
        let stderr = self
            .stderr
            .take()
            .map(|drained| drained.join().expect("standard error is read"))
            .unwrap_or_default();
        (status.expect("ended"), stderr)
    }
}

/// One shim's end of a connection, after its `HELLO`.
struct Shim {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Shim {
    /// Connect (waiting for the socket to appear), send a `HELLO` for `key`, and return the
    /// reply line too. As a shim does (spec §4.1 step 5), a connection that is refused or that
    /// ends before any reply is transient, and the shim connects again: on macOS a listener can
    /// outlive its owner in a child another test spawned at the moment it was created (it is made
    /// close-on-exec only after it exists), so a stale socket can accept and then close.
    fn hello(paths: &DaemonPaths, key: IdentityKey) -> (Self, String) {
        let give_up = Instant::now() + HANG;
        loop {
            if let Some(answered) = Self::try_hello(paths, key) {
                return answered;
            }
            assert!(
                Instant::now() < give_up,
                "no reply to HELLO within {HANG:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn try_hello(paths: &DaemonPaths, key: IdentityKey) -> Option<(Self, String)> {
        let writer = UnixStream::connect(paths.socket()).ok()?;
        writer.set_read_timeout(Some(HANG)).ok()?;
        let settings = ClientSettings::new(
            ConnectionSettings {
                result_cap: 80_000,
                structured_content: StructuredContentMode::Always,
                concise_shape: ConciseShapeMode::Slim,
            },
            600,
            false,
        )
        .expect("settings");
        let mut shim = Self {
            reader: BufReader::new(writer.try_clone().ok()?),
            writer,
        };
        let hello = Hello {
            identity: key,
            settings,
        }
        .encode();
        // A daemon that answers `BUSY` or `RETIRING` may close before the `HELLO` is written;
        // its reply is read anyway (review round 1).
        let _ = shim.writer.write_all(hello.as_bytes());
        let mut reply = String::new();
        shim.reader.read_line(&mut reply).ok()?;
        (!reply.is_empty()).then_some((shim, reply))
    }

    fn send_raw(&mut self, text: &str) {
        self.writer.write_all(text.as_bytes()).expect("write");
    }

    fn line(&mut self) -> String {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .expect("a line within the hang limit");
        line
    }

    fn request(&mut self, line: &str) -> serde_json::Value {
        self.send_raw(&format!("{line}\n"));
        serde_json::from_str(&self.line()).expect("a JSON response")
    }

    /// `initialize`, then `notifications/initialized`.
    fn handshake(&mut self) {
        let initialized = self.request(INIT);
        assert!(initialized["result"].is_object(), "{initialized}");
        self.send_raw(&format!("{INITED}\n"));
    }

    /// The daemon has closed the connection and sent nothing more. On Linux a connection closed
    /// with bytes of this shim's still unread is reported as a reset, not end of stream (review
    /// round 1); a shim must take both as the daemon gone.
    fn expect_end_of_stream(&mut self) {
        let mut rest = String::new();
        let read = self.reader.read_to_string(&mut rest);
        let closed = match &read {
            Ok(0) => true,
            Err(error) => error.kind() == std::io::ErrorKind::ConnectionReset && rest.is_empty(),
            Ok(_) => false,
        };
        assert!(closed, "{read:?} {rest}");
    }
}

fn ready_pid(reply: &str) -> u32 {
    match Reply::decode(reply.trim_end().as_bytes()) {
        Ok(Reply::Ready(ready)) => ready.pid,
        other => panic!("expected READY, got {other:?} from {reply:?}"),
    }
}

pub(crate) fn gone(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_err()
}

/// Spec §9 test 2's real-clock smoke test: a daemon process serves an admitted shim from a real
/// index, and once the shim leaves and its idle TTL (1 s here) passes, it drains, removes its
/// socket and pid file, and exits 0.
#[test]
fn a_daemon_process_serves_a_shim_and_retires_after_its_idle_ttl() {
    let setup = setup();
    let mut daemon = Daemon::start(&setup, &["--idle-ttl", "1"]);
    let (mut shim, reply) = Shim::hello(&setup.paths, setup.key);
    assert_eq!(ready_pid(&reply), daemon.pid());
    let pid_file = std::fs::read_to_string(setup.paths.pid_file()).expect("the pid file");
    assert!(
        pid_file.starts_with(&format!("{}\n", daemon.pid())),
        "{pid_file}"
    );
    shim.handshake();
    let answer = shim.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    drop(shim);

    let (status, stderr) = daemon.ended();
    assert!(status.success(), "{status:?} {stderr}");
    assert!(stderr.contains("stopped: idle"), "{stderr}");
    assert!(gone(setup.paths.socket()));
    assert!(gone(setup.paths.pid_file()));
}

/// Spec §9 test 3, the daemon's half: a socket file left by a dead daemon, with no owner lock
/// held, is replaced by the next daemon, which answers `READY` on it.
#[test]
fn a_daemon_replaces_a_stale_socket_left_by_a_dead_one() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    drop(UnixListener::bind(setup.paths.socket()).expect("a stale socket"));
    // Not `connect(...).is_err()`: on macOS a socket is made close-on-exec only after it is
    // created, so a child another test spawns in that moment can inherit the listener and keep
    // it accepting. That the file is there is the precondition; the daemon replaces it either way.
    assert!(std::fs::symlink_metadata(setup.paths.socket())
        .is_ok_and(|metadata| metadata.file_type().is_socket()));

    let daemon = Daemon::start(&setup, &[]);
    let (_shim, reply) = Shim::hello(&setup.paths, setup.key);
    assert_eq!(ready_pid(&reply), daemon.pid());
}

/// Spec §9 test 4, the daemon's half, and §3.1 "exit 0 silently": a daemon started while another
/// process holds the owner lock (and a socket that accepts and never answers) exits 0 without a
/// word and leaves the socket and the pid file alone. That the shim then fails closed naming the
/// pid is S5a's half.
#[test]
fn a_daemon_that_finds_a_live_owner_exits_silently_and_touches_nothing() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(setup.paths.owner_lock())
        .expect("the owner lock file");
    owner.try_lock().expect("the test owns the key");
    let wedged = UnixListener::bind(setup.paths.socket()).expect("a wedged owner's socket");
    let socket_inode = std::fs::symlink_metadata(setup.paths.socket())
        .expect("socket")
        .ino();

    let mut daemon = Daemon::start(&setup, &[]);
    let (status, stderr) = daemon.ended();
    assert!(status.success(), "{status:?}");
    assert_eq!(stderr, "");
    assert_eq!(
        std::fs::symlink_metadata(setup.paths.socket())
            .expect("the socket is still there")
            .ino(),
        socket_inode
    );
    assert!(gone(setup.paths.pid_file()));
    let _client = UnixStream::connect(setup.paths.socket()).expect("the wedged owner's socket");
    wedged.accept().expect("still the wedged owner's");
    drop::<File>(owner);
}

/// A daemon that finds the key owned leaves the owner's log alone (review round 1, both
/// reviewers): its standard error is the key's log, a regular file over 1 MiB opened for
/// appending as the spawning shim opens it (spec §4.1 step 6), while another process owns the
/// key. It exits 0 and the log is byte for byte what it was. Rules out truncating the log
/// before the claim.
#[test]
fn a_daemon_that_finds_a_live_owner_leaves_the_owner_s_log_alone() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(setup.paths.owner_lock())
        .expect("the owner lock file");
    owner.try_lock().expect("the test owns the key");
    let line = b"prism-mcp daemon: a line of the daemon that owns the key\n";
    let contents = line.repeat((1 << 20) / line.len() + 2);
    assert!(contents.len() > 1 << 20);
    std::fs::write(setup.paths.log_file(), &contents).expect("the owner's log");
    let log = std::fs::OpenOptions::new()
        .append(true)
        .open(setup.paths.log_file())
        .expect("the log, for appending");

    let mut daemon = Daemon::start_logging_to(&setup, &[], log);
    let (status, _) = daemon.ended();
    assert!(status.success(), "{status:?}");
    assert_eq!(
        std::fs::read(setup.paths.log_file()).expect("the log"),
        contents
    );
    drop::<File>(owner);
}

/// Spec §9 tests 8 and 15, the daemon's halves: daemons started together for one key (as
/// racing shims, or a dead starter's child and the next starter's, would start them) leave
/// exactly one serving; the others exit 0, and every shim is admitted by the survivor.
#[test]
fn daemons_started_together_leave_exactly_one_serving() {
    let setup = setup();
    let mut daemons: Vec<Daemon> = (0..4).map(|_| Daemon::start(&setup, &[])).collect();
    wait_until("all but one daemon to exit", || {
        daemons
            .iter_mut()
            .map(|daemon| daemon.exited().is_some())
            .filter(|exited| *exited)
            .count()
            == 3
    });
    let mut survivors = Vec::new();
    for daemon in &mut daemons {
        match daemon.exited() {
            Some(_) => {
                let (status, stderr) = daemon.ended();
                assert!(status.success(), "{status:?} {stderr}");
                assert_eq!(stderr, "");
            }
            None => survivors.push(daemon.pid()),
        }
    }
    assert_eq!(survivors.len(), 1);
    for _ in 0..4 {
        let (_shim, reply) = Shim::hello(&setup.paths, setup.key);
        assert_eq!(ready_pid(&reply), survivors[0]);
    }
}

/// Spec §3.1: SIGTERM goes through Draining: an attached shim sees its connection end, the
/// socket and the pid file are removed, and the process exits 0.
#[test]
fn a_termination_signal_drains_a_daemon_with_a_shim_attached() {
    let setup = setup();
    let mut daemon = Daemon::start(&setup, &[]);
    let (mut shim, reply) = Shim::hello(&setup.paths, setup.key);
    assert_eq!(ready_pid(&reply), daemon.pid());
    shim.handshake();

    let pid = libc::pid_t::try_from(daemon.pid()).expect("a pid");
    // SAFETY: `kill` with a valid signal number has no memory-safety preconditions.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGTERM) }, 0);
    shim.expect_end_of_stream();
    let (status, stderr) = daemon.ended();
    assert!(status.success(), "{status:?} {stderr}");
    assert!(stderr.contains("termination signal"), "{stderr}");
    assert!(gone(setup.paths.socket()));
    assert!(gone(setup.paths.pid_file()));
}

/// Lane README, notes for S5a: a daemon handed a key that is not the one it computes refuses to
/// start, with a message, and binds nothing.
#[test]
fn a_daemon_handed_another_key_refuses_to_start() {
    let setup = setup();
    let other = "f".repeat(64);
    let mut daemon = Daemon::start(&setup, &["--key", &other]);
    let (status, stderr) = daemon.ended();
    assert!(!status.success());
    assert!(stderr.contains("--key"), "{stderr}");
    assert!(gone(setup.paths.socket()));
}

/// Spec §11 item 7: `--warm-at-startup` on the daemon's own command line builds the index before
/// any tool call; a later `READY` reports it ready.
#[test]
fn a_warm_daemon_builds_before_any_tool_call() {
    let setup = setup();
    let _daemon = Daemon::start(&setup, &["--warm-at-startup"]);
    wait_until("a READY that reports the index ready", || {
        let (_shim, reply) = Shim::hello(&setup.paths, setup.key);
        matches!(
            Reply::decode(reply.trim_end().as_bytes()),
            Ok(Reply::Ready(ready)) if ready.build == BuildState::Ready
        )
    });
}
