//! `prism-mcp --shared` as a process: the shim (spec §4.1, §4.6, §10; §9 tests 5, 10 and 11,
//! and the shim's halves of tests 3, 4 and 12). Each test runs the real binary with a runtime
//! directory of its own; the daemons a shim starts are detached from it, so every test ends them
//! itself (`Daemons`), however it ends.
#![cfg(feature = "mcp")]

use crate::daemon_process_test::{gone, setup, wait_until, Setup, CALLEES, HANG, INIT, INITED};
use crate::transcript::spawn_line_reader;
use prism::mcp::shared::paths::DaemonPaths;
use prism::mcp::shared::preamble::Reply;
use prism::mcp::shared::runtime_dir::{validate_runtime_dir, RUNTIME_DIR_ENV};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A shim process with piped stdio; its standard error goes to a file the test reads.
pub(crate) struct ShimProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: mpsc::Receiver<std::io::Result<Vec<u8>>>,
    stderr_path: std::path::PathBuf,
    _stderr_dir: tempfile::TempDir,
}

impl Drop for ShimProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl ShimProcess {
    /// `prism-mcp --repo R --cache-dir C --shared <extra>`, with the setup's runtime base and
    /// `env`.
    pub(crate) fn start(setup: &Setup, extra: &[&str], env: &[(&str, &str)]) -> Self {
        Self::start_with_cache(setup, setup.cache.path(), extra, env)
    }

    pub(crate) fn start_with_cache(
        setup: &Setup,
        cache: &std::path::Path,
        extra: &[&str],
        env: &[(&str, &str)],
    ) -> Self {
        Self::start_with(setup, setup.repo.path(), cache, extra, env)
    }

    /// The shim for any repository and cache. Its deadline is 60 s unless `extra` sets one: a
    /// test that means to reach a daemon must not depend on how fast this host starts one
    /// (review round 1, Opus S2).
    pub(crate) fn start_with(
        setup: &Setup,
        repo: &std::path::Path,
        cache: &std::path::Path,
        extra: &[&str],
        env: &[(&str, &str)],
    ) -> Self {
        let stderr_dir = tempfile::tempdir().expect("a directory for standard error");
        let stderr_path = stderr_dir.path().join("stderr.txt");
        let mut command = Command::new(env!("CARGO_BIN_EXE_prism-mcp"));
        command
            .arg("--repo")
            .arg(repo)
            .arg("--cache-dir")
            .arg(cache)
            .arg("--shared")
            .args(extra);
        if !extra.contains(&"--daemon-start-timeout") {
            command.args(["--daemon-start-timeout", "60"]);
        }
        command
            .env(RUNTIME_DIR_ENV, setup.base.path())
            .env_remove("PRISM_MCP_STRUCTURED_CONTENT")
            .env_remove("PRISM_MCP_CONCISE_SHAPE")
            .env_remove("PRISM_MCP_MAX_RESULT_CHARS");
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(
                File::create(&stderr_path).expect("stderr file"),
            ))
            .spawn()
            .expect("start the shim");
        let stdin = child.stdin.take();
        let (lines, _reader) = spawn_line_reader(child.stdout.take().expect("stdout"));
        Self {
            child,
            stdin,
            lines,
            stderr_path,
            _stderr_dir: stderr_dir,
        }
    }

    pub(crate) fn send(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{line}").expect("write to the shim");
        stdin.flush().expect("flush");
    }

    pub(crate) fn request(&mut self, line: &str) -> serde_json::Value {
        self.send(line);
        match self.lines.recv_timeout(HANG) {
            Ok(Ok(response)) => serde_json::from_slice(&response).expect("a JSON response"),
            other => panic!(
                "no response from the shim: {other:?}; stderr: {}",
                self.stderr()
            ),
        }
    }

    /// `initialize`, then `notifications/initialized`.
    pub(crate) fn handshake(&mut self) {
        let initialized = self.request(INIT);
        assert!(initialized["result"].is_object(), "{initialized}");
        self.send(INITED);
    }

    pub(crate) fn close_input(&mut self) {
        self.stdin = None;
    }

    pub(crate) fn stderr(&self) -> String {
        std::fs::read_to_string(&self.stderr_path).unwrap_or_default()
    }

    /// Wait for the shim to end; return how it ended and its standard error.
    pub(crate) fn ended(&mut self) -> (ExitStatus, String) {
        let mut status = None;
        wait_until("the shim to end", || {
            status = self.child.try_wait().expect("the shim's status");
            status.is_some()
        });
        (status.expect("ended"), self.stderr())
    }

    /// The pid of the daemon the shim reports it reached.
    pub(crate) fn daemon_pid(&self) -> u32 {
        let stderr = self.stderr();
        let rest = stderr
            .split("shared daemon pid ")
            .nth(1)
            .unwrap_or_else(|| panic!("no daemon reached: {stderr}"));
        rest.split(' ')
            .next()
            .and_then(|pid| pid.parse().ok())
            .expect("a pid")
    }
}

/// Ends, on drop, whatever daemon owns the setup's key: the daemons a shim starts are not the
/// test's children, so nothing else would.
pub(crate) struct Daemons<'a>(pub(crate) &'a DaemonPaths);

impl Drop for Daemons<'_> {
    fn drop(&mut self) {
        end_daemons(self.0);
    }
}

/// Whether a process of this test could take the owner lock now.
pub(crate) fn owner_is_free(paths: &DaemonPaths) -> bool {
    match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(paths.owner_lock())
    {
        Ok(lock) => lock.try_lock().is_ok(),
        Err(_) => false,
    }
}

/// The pid the pid file names, when it is whole.
pub(crate) fn pid_file_pid(paths: &DaemonPaths) -> Option<u32> {
    let text = std::fs::read_to_string(paths.pid_file()).ok()?;
    text.lines().next()?.parse().ok()
}

pub(crate) fn kill(pid: u32, signal: libc::c_int) {
    let pid = libc::pid_t::try_from(pid).expect("a pid");
    // SAFETY: `kill` with a valid signal number has no memory-safety preconditions.
    unsafe { libc::kill(pid, signal) };
}

/// Kill the daemon that owns the key, if one does, and wait until the key is free.
pub(crate) fn end_daemons(paths: &DaemonPaths) {
    if !paths.runtime_dir().exists() {
        return;
    }
    let give_up = std::time::Instant::now() + HANG;
    while !owner_is_free(paths) && std::time::Instant::now() < give_up {
        if let Some(pid) = pid_file_pid(paths) {
            kill(pid, libc::SIGKILL);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs()
}

/// Spec §4.1 and §3: a shim with no daemon running starts one, detached in a session of its own,
/// with its standard error appended to the key's log; serves a session through it; and exits 0
/// when its client closes its input. The pid it reports is the one in the pid file.
#[test]
fn a_shim_starts_a_detached_daemon_and_serves_a_session_through_it() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    std::fs::write(setup.paths.log_file(), "an earlier daemon's line\n").expect("a log");
    let mut shim = ShimProcess::start(&setup, &["--idle-ttl", "30"], &[]);
    shim.handshake();
    let answer = shim.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    let pid = shim.daemon_pid();
    assert_eq!(pid_file_pid(&setup.paths), Some(pid));
    let sid = {
        let pid = libc::pid_t::try_from(pid).expect("a pid");
        // SAFETY: `getsid` takes a pid and touches no memory.
        unsafe { libc::getsid(pid) }
    };
    assert_eq!(
        sid,
        libc::pid_t::try_from(pid).expect("a pid"),
        "its own session"
    );
    let log = std::fs::read_to_string(setup.paths.log_file()).expect("the log");
    assert!(log.starts_with("an earlier daemon's line\n"), "{log}");
    assert!(log.contains(&format!("pid {pid} serves")), "{log}");

    shim.close_input();
    let (status, stderr) = shim.ended();
    assert!(status.success(), "{status:?} {stderr}");
}

/// Spec §9 test 5: a runtime directory that fails validation (a symlink, or a directory other
/// users can read) fails the shim closed: a non-zero exit, one line on standard error naming the
/// cause and the `--standalone` invocation, no daemon, and no cache directory created anywhere.
#[test]
fn a_runtime_directory_that_fails_validation_fails_closed_and_creates_nothing() {
    for bad in ["symlink", "mode"] {
        let setup = setup();
        let dir = setup.paths.runtime_dir();
        match bad {
            "symlink" => {
                let elsewhere = setup.base.path().join("elsewhere");
                std::fs::create_dir(&elsewhere).expect("a directory");
                std::os::unix::fs::symlink(&elsewhere, dir).expect("a symlink");
            }
            _ => {
                std::fs::create_dir(dir).expect("the runtime directory");
                std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))
                    .expect("mode 0755");
            }
        }
        let cache = setup.cache.path().join("never-created");
        let mut shim = ShimProcess::start_with_cache(&setup, &cache, &[], &[]);
        let (status, stderr) = shim.ended();
        assert!(!status.success(), "{bad}: {status:?}");
        assert_eq!(stderr.lines().count(), 1, "{bad}: {stderr}");
        assert!(stderr.contains("failed closed"), "{bad}: {stderr}");
        assert!(stderr.contains("--standalone"), "{bad}: {stderr}");
        let cause = if bad == "symlink" {
            "symlink"
        } else {
            "mode 0755"
        };
        assert!(stderr.contains(cause), "{bad}: {stderr}");
        assert!(!cache.exists(), "{bad}: a cache directory was created");
        assert!(gone(
            &dir.join(setup.paths.socket().file_name().expect("name"))
        ));
    }
}

/// Spec §9 test 10: a daemon at the key's socket that answers `MISMATCH` (here a fixture that
/// owns the key, standing in for a daemon of another build) fails the shim closed, naming the
/// identity it serves, and nothing replaces it.
#[test]
fn a_daemon_that_answers_mismatch_fails_the_shim_closed() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    let owner = owner_lock_held(&setup.paths);
    let listener = UnixListener::bind(setup.paths.socket()).expect("the fixture's socket");
    let socket = std::fs::symlink_metadata(setup.paths.socket())
        .expect("socket")
        .ino();
    let expected = "f".repeat(64);
    let fixture = {
        let expected = expected.clone();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let _ = read_line(&stream);
            let reply = Reply::Mismatch {
                expected: prism::mcp::shared::identity::IdentityKey::from_hex(&expected)
                    .expect("a key"),
            };
            let _ = stream.write_all(reply.encode().as_bytes());
        })
    };
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    let (status, stderr) = shim.ended();
    assert!(!status.success());
    assert!(stderr.contains("MISMATCH"), "{stderr}");
    assert!(stderr.contains(&expected), "{stderr}");
    fixture.join().expect("the fixture");
    assert_eq!(
        std::fs::symlink_metadata(setup.paths.socket())
            .expect("socket")
            .ino(),
        socket
    );
    drop::<File>(owner);
}

/// Spec §9 test 11 and §4.1 step 9: a daemon killed after MCP bytes have been forwarded ends the
/// shim with a non-zero exit, and the shim does not reconnect: no daemon owns the key afterwards,
/// and the pid file still names the killed one.
#[test]
fn a_daemon_killed_mid_session_ends_the_shim_without_a_reconnect() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    shim.handshake();
    let pid = shim.daemon_pid();
    kill(pid, libc::SIGKILL);
    let (status, stderr) = shim.ended();
    assert!(!status.success(), "{status:?}");
    assert!(stderr.contains("does not reconnect"), "{stderr}");
    assert!(owner_is_free(&setup.paths), "a daemon was started again");
    assert_eq!(pid_file_pid(&setup.paths), Some(pid));
}

/// Spec §9 test 3, the shim's half: a socket and a pid file left by a dead daemon are replaced:
/// the shim starts a daemon that serves on the same path.
#[test]
fn a_stale_socket_left_by_a_dead_daemon_is_replaced() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    drop(UnixListener::bind(setup.paths.socket()).expect("a stale socket"));
    std::fs::write(setup.paths.pid_file(), "999999\n1\nold\n").expect("a stale pid file");
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    shim.handshake();
    let answer = shim.request(CALLEES);
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    assert_eq!(pid_file_pid(&setup.paths), Some(shim.daemon_pid()));
}

/// Spec §9 test 4, the shim's half: a live daemon (it holds the owner lock) that accepts and
/// never answers fails the shim closed at its deadline, naming the pid and start time from the
/// pid file; its socket is not unlinked and no daemon is started in its place.
#[test]
fn a_live_but_wedged_owner_fails_the_shim_closed_naming_its_pid() {
    let setup = setup();
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    let owner = owner_lock_held(&setup.paths);
    let listener = UnixListener::bind(setup.paths.socket()).expect("the wedged socket");
    let socket = std::fs::symlink_metadata(setup.paths.socket())
        .expect("socket")
        .ino();
    std::fs::write(setup.paths.pid_file(), "4242\n1700000000\nwedged\n").expect("pid file");
    // The fixture accepts until it is told to stop, or its sender is dropped by a failing test;
    // it never blocks in `accept`, so joining it cannot hang.
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    listener.set_nonblocking(true).expect("nonblocking");
    let fixture = std::thread::spawn(move || {
        let mut held = Vec::new();
        while let Err(mpsc::TryRecvError::Empty) = stop_rx.try_recv() {
            if let Ok((stream, _)) = listener.accept() {
                held.push(stream);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
    let mut shim = ShimProcess::start(&setup, &["--daemon-start-timeout", "2"], &[]);
    let (status, stderr) = shim.ended();
    let _ = stop_tx.send(());
    fixture.join().expect("the fixture");
    assert!(!status.success());
    assert!(
        stderr.contains("pid 4242, start time 1700000000"),
        "{stderr}"
    );
    assert_eq!(
        std::fs::symlink_metadata(setup.paths.socket())
            .expect("socket")
            .ino(),
        socket
    );
    assert_eq!(
        std::fs::read_to_string(setup.paths.pid_file()).expect("pid file"),
        "4242\n1700000000\nwedged\n"
    );
    drop::<File>(owner);
}

/// Spec §9 test 12, the shim's half ("the next shim restarts it"): after the daemon dies (here
/// killed; S3b's process test aborts one by an injected panic), the next shim finds the key free,
/// replaces the dead daemon's files and starts a daemon that serves it.
#[test]
fn the_next_shim_restarts_a_daemon_that_died() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    let mut first = ShimProcess::start(&setup, &[], &[]);
    first.handshake();
    assert_eq!(first.request(CALLEES)["result"]["isError"], false);
    let dead = first.daemon_pid();
    kill(dead, libc::SIGKILL);
    let _ = first.ended();

    let mut next = ShimProcess::start(&setup, &[], &[]);
    next.handshake();
    assert_eq!(next.request(CALLEES)["result"]["isError"], false);
    assert_ne!(next.daemon_pid(), dead);
}

/// S3b's SI-10: a key whose last daemon died during its index build moments ago (its build
/// marker left) is not restarted: the shim fails closed naming the log and leaves the files. A
/// minute after that build began, the next shim starts a daemon again and the marker is gone.
#[test]
fn a_daemon_that_died_during_its_build_is_not_restarted_for_a_minute() {
    let setup = setup();
    let _daemons = Daemons(&setup.paths);
    validate_runtime_dir(setup.paths.runtime_dir()).expect("the runtime directory");
    drop(UnixListener::bind(setup.paths.socket()).expect("the dead daemon's socket"));
    std::fs::write(setup.paths.pid_file(), "4242\n1700000000\nb\n").expect("pid file");
    std::fs::write(
        setup.paths.build_marker(),
        format!("4242\n{}\n", unix_now() - 5),
    )
    .expect("a build marker");
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    let (status, stderr) = shim.ended();
    assert!(!status.success());
    assert!(stderr.contains("died during its index build"), "{stderr}");
    assert!(
        stderr.contains(&setup.paths.log_file().display().to_string()),
        "{stderr}"
    );
    assert!(setup.paths.socket().exists() && setup.paths.pid_file().exists());
    assert!(owner_is_free(&setup.paths));
    // The shim recorded when it first saw the dead build (review round 1, sol 3): the minute
    // runs from there.
    let marker = std::fs::read_to_string(setup.paths.build_marker()).expect("the marker");
    assert_eq!(marker.lines().count(), 3, "{marker:?}");

    std::fs::write(
        setup.paths.build_marker(),
        format!("4242\n{}\n{}\n", unix_now() - 300, unix_now() - 120),
    )
    .expect("a build marker first seen two minutes ago");
    let mut shim = ShimProcess::start(&setup, &[], &[]);
    shim.handshake();
    assert_eq!(shim.request(CALLEES)["result"]["isError"], false);
    assert!(!setup.paths.build_marker().exists());
}

/// Spec §4.1 step 7, for failures before the key is known (review round 1, sol 5): a repository
/// that does not exist, and a cache directory that is a regular file, still fail closed in one
/// line that names the `--standalone` invocation.
#[test]
fn a_failure_before_the_key_is_known_still_names_the_standalone_invocation() {
    let setup = setup();
    let missing = setup.repo.path().join("no-such-repository");
    let not_a_dir = setup.cache.path().join("a-file");
    std::fs::write(&not_a_dir, "not a directory").expect("a regular file");
    for (repo, cache) in [
        (missing.as_path(), setup.cache.path()),
        (setup.repo.path(), not_a_dir.as_path()),
    ] {
        let mut shim = ShimProcess::start_with(&setup, repo, cache, &[], &[]);
        let (status, stderr) = shim.ended();
        assert!(!status.success(), "{stderr}");
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
        assert!(stderr.contains("failed closed"), "{stderr}");
        assert!(stderr.contains("--standalone"), "{stderr}");
    }
}

/// Hold the key's owner lock, as a live daemon does.
pub(crate) fn owner_lock_held(paths: &DaemonPaths) -> File {
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(paths.owner_lock())
        .expect("the owner lock file");
    owner.try_lock().expect("the test owns the key");
    owner
}

/// One line from a stream, newline included, read one byte at a time.
pub(crate) fn read_line(mut stream: &UnixStream) -> String {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    while let Ok(1) = stream.read(&mut byte) {
        line.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8_lossy(&line).into_owned()
}
