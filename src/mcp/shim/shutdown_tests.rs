//! Tests for `daemon-shutdown` against a stand-in daemon (S5b; spec §4.7, §11 item 3): which
//! instance is asked, what is never sent, and when the command says the daemon drained.
use super::super::control_support::{
    fake_instance, fake_status, ControlFake, FakeReply, FAKE_PID, FAKE_START,
};
use super::super::shim_support::{as_root, full_backlog, open_lock, this_user, Unsearchable, HANG};
use super::*;
use crate::mcp::shared::control::DaemonStatus;
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::shared::preamble::Reply;
use crate::mcp::shared::runtime_dir::validate_runtime_dir;
use crate::mcp::CacheMode;
use std::sync::Arc;

/// A configuration of the test's own, its key for this build, and a validated runtime
/// directory under `/tmp` with that key's paths.
struct Setup {
    _repo: tempfile::TempDir,
    _cache: tempfile::TempDir,
    base: tempfile::TempDir,
    config: ServerConfig,
    key: IdentityKey,
    paths: DaemonPaths,
}

fn setup() -> Setup {
    let repo = tempfile::tempdir().expect("a repository");
    let cache = tempfile::tempdir().expect("a cache base");
    let base = tempfile::Builder::new()
        .prefix("s5b")
        .tempdir_in("/tmp")
        .expect("a runtime base under /tmp");
    let mut config = ServerConfig::new(repo.path().to_path_buf());
    config.cache = CacheMode::Dir(cache.path().to_path_buf());
    let key = key_of(&config).expect("a key");
    let dir = base.path().join("prism-mcpd");
    validate_runtime_dir(&dir).expect("a valid runtime directory");
    let paths = DaemonPaths::new(&dir, &key).expect("paths");
    Setup {
        _repo: repo,
        _cache: cache,
        base,
        config,
        key,
        paths,
    }
}

impl Setup {
    fn options(&self, instance: Option<(u32, u64)>, timeout: Duration) -> ShutdownOptions {
        ShutdownOptions {
            config: self.config.clone(),
            runtime_base: Some(self.base.path().to_path_buf()),
            instance,
            timeout,
        }
    }

    /// The fake's status, as the daemon of this configuration.
    fn status(&self, admitted: u32, shutdown: bool) -> DaemonStatus {
        let mut status = fake_status(admitted, shutdown);
        status.instance.identity = self.key;
        status
    }

    fn instance(&self) -> Instance {
        Instance {
            identity: self.key,
            ..fake_instance()
        }
    }

    fn write_pid_file(&self) {
        std::fs::write(
            self.paths.pid_file(),
            format!("{FAKE_PID}\n{FAKE_START}\n{}\n", "b".repeat(64)),
        )
        .expect("write the pid file");
    }

    /// A fake that answers `STATUS` with `status` and `SHUTDOWN` with `accepted`, running
    /// `on_shutdown` first.
    fn fake(
        &self,
        status: DaemonStatus,
        accepted: DaemonStatus,
        on_shutdown: impl Fn() + Send + Sync + 'static,
    ) -> ControlFake {
        ControlFake::start(
            &self.paths,
            true,
            Arc::new(move |line: &str| {
                if line.starts_with("SHUTDOWN ") {
                    on_shutdown();
                    FakeReply::Line(accepted.encode())
                } else {
                    FakeReply::Line(status.encode())
                }
            }),
        )
    }
}

fn run(options: &ShutdownOptions) -> Result<String, String> {
    shutdown(options, this_user)
}

/// No daemon, or no runtime directory at all: nothing to do, and nothing is created.
#[test]
fn with_no_daemon_there_is_nothing_to_shut_down() {
    let setup = setup();
    let said = run(&setup.options(None, HANG)).expect("nothing to do is not a failure");
    assert!(said.contains("nothing to shut down"), "{said}");
    let empty = tempfile::Builder::new()
        .prefix("s5b")
        .tempdir_in("/tmp")
        .expect("a runtime base");
    let options = ShutdownOptions {
        runtime_base: Some(empty.path().to_path_buf()),
        ..setup.options(None, HANG)
    };
    assert!(run(&options)
        .expect("nothing to do")
        .contains("nothing to shut down"));
    assert!(
        std::fs::symlink_metadata(empty.path().join("prism-mcpd")).is_err(),
        "the runtime directory is not created"
    );
}

/// Spec §4.7: the instance a status exchange has just shown is the one asked, and with no client
/// the command waits until it has drained (its pid file is gone). Rules out a request that names
/// no instance, and a command that reports success before the drain.
#[test]
fn an_idle_daemon_is_asked_by_its_instance_and_waited_for() {
    let setup = setup();
    setup.write_pid_file();
    let pid_path = setup.paths.pid_file().to_path_buf();
    let fake = setup.fake(setup.status(0, false), setup.status(0, true), move || {
        let _ = std::fs::remove_file(&pid_path);
    });
    let said = run(&setup.options(None, HANG)).expect("drained");
    assert_eq!(
        said,
        format!("pid {FAKE_PID} (started {FAKE_START}) drained")
    );
    assert_eq!(
        fake.lines(),
        vec![
            ControlRequest::Status.encode(),
            ControlRequest::Shutdown(setup.instance()).encode()
        ]
    );
}

/// Spec §11 item 3: a shutdown never disconnects a client. A daemon with clients accepts the
/// request and the command says it drains when they have gone, without waiting for that.
#[test]
fn a_daemon_with_clients_is_left_serving_them() {
    let setup = setup();
    setup.write_pid_file();
    let _fake = setup.fake(setup.status(2, false), setup.status(2, true), || {});
    let said = run(&setup.options(None, HANG)).expect("accepted");
    assert!(
        said.contains("drains once its 2 admitted clients have disconnected"),
        "{said}"
    );
    assert!(
        setup.paths.pid_file().exists(),
        "the daemon was not drained"
    );
}

/// `--pid` and `--start-time` bind the request to one instance: when the daemon that answers is
/// another, it is left alone and no `SHUTDOWN` is sent. Rules out a command that ignores them.
#[test]
fn another_instance_named_by_pid_and_start_time_is_left_alone() {
    let setup = setup();
    let fake = setup.fake(setup.status(0, false), setup.status(0, true), || {});
    let said = run(&setup.options(Some((FAKE_PID, FAKE_START + 1)), HANG)).expect("left alone");
    assert!(said.contains("which was left alone"), "{said}");
    assert_eq!(fake.lines(), vec![ControlRequest::Status.encode()]);
}

/// The status exchange verifies the instance before it is asked: a daemon at the key's socket
/// that serves another identity (another key with the same first 16 digits) is not asked.
#[test]
fn a_daemon_of_another_identity_is_not_asked() {
    let setup = setup();
    let other = fake_status(0, false);
    assert_ne!(other.instance.identity, setup.key);
    let fake = setup.fake(other.clone(), other, || {});
    let said = run(&setup.options(None, HANG)).expect_err("not this configuration's");
    assert!(said.contains("it was not asked"), "{said}");
    assert_eq!(fake.lines(), vec![ControlRequest::Status.encode()]);
}

/// A daemon that was replaced between the two exchanges answers the `SHUTDOWN` with its own,
/// other, instance and records nothing; the command says so and does not wait.
#[test]
fn a_daemon_replaced_between_the_exchanges_is_left_alone() {
    let setup = setup();
    let mut replaced = setup.status(0, false);
    replaced.instance.pid = FAKE_PID + 1;
    let _fake = setup.fake(setup.status(0, false), replaced, || {});
    let said = run(&setup.options(None, HANG)).expect("left alone");
    assert!(said.contains("was replaced by pid"), "{said}");
}

/// A daemon that accepts the request and does not drain by the timeout (a client arrived) is
/// reported, with exit status 1. Rules out a wait without a bound.
#[test]
fn a_daemon_that_does_not_drain_by_the_timeout_is_reported() {
    let setup = setup();
    setup.write_pid_file();
    let _fake = setup.fake(setup.status(0, false), setup.status(0, true), || {});
    // Both exchanges must finish inside the deadline for the wait to be reached, so it is not
    // short (PR #418 review round 1, Opus S1); only its outcome is asserted.
    let said = run(&setup.options(None, Duration::from_secs(2))).expect_err("not drained");
    assert!(said.contains("still held key"), "{said}");
}

/// A daemon that is draining is not asked; one at its bound, one that does not answer and one
/// that refuses the line could not be asked (exit status 1).
#[test]
fn draining_busy_silent_and_refusing_daemons_are_told_apart() {
    let setup = setup();
    let retiring = ControlFake::start(
        &setup.paths,
        true,
        Arc::new(|_| FakeReply::Line(Reply::Retiring.encode())),
    );
    assert!(run(&setup.options(None, HANG))
        .expect("draining")
        .contains("already draining"));
    retiring.retire();
    // Only the silent daemon's deadline is meant to expire, so only it is short; the others must
    // finish their exchange inside theirs (PR #418 review round 1, Opus S1).
    let replies: [(FakeReply, &str, Duration); 3] = [
        (
            FakeReply::Line(Reply::Busy { max_clients: 3 }.encode()),
            "bound of 3",
            HANG,
        ),
        (
            FakeReply::Silent,
            "does not answer",
            Duration::from_millis(300),
        ),
        (FakeReply::Close, "does not answer", HANG),
    ];
    for (reply, says, timeout) in replies {
        let reply = Arc::new(std::sync::Mutex::new(Some(reply)));
        let fake = ControlFake::start(
            &setup.paths,
            true,
            Arc::new(move |_| {
                reply
                    .lock()
                    .expect("reply")
                    .take()
                    .unwrap_or(FakeReply::Close)
            }),
        );
        let said = run(&setup.options(None, timeout)).expect_err(says);
        assert!(said.contains(says), "{said}");
        fake.retire();
    }
}

/// Review round 1, Opus W1: a live daemon that does not accept connections (its listen backlog is
/// full; or, when a dead daemon's files are still there, one that has claimed the key and not
/// bound yet) is not "nothing to shut down": its owner lock is held, and the command fails (exit
/// 1). A key whose socket refuses and whose owner lock
/// is free has no daemon. Rules out taking every connect failure for a missing daemon.
#[test]
fn a_live_daemon_that_does_not_accept_connections_is_not_taken_for_none() {
    let setup = setup();
    setup.write_pid_file();
    let owner = open_lock(setup.paths.owner_lock());
    owner.try_lock().expect("the owner lock");
    let (_listener, _clients) = full_backlog(setup.paths.socket());
    let said = run(&setup.options(None, Duration::from_millis(500))).expect_err("alive");
    assert!(
        said.contains("alive but not accepting connections"),
        "{said}"
    );
    drop(owner);
    let said = run(&setup.options(None, Duration::from_millis(500))).expect("no daemon");
    assert!(said.contains("nothing to shut down"), "{said}");
}

/// Review round 1, Opus W4: when the reply to the `SHUTDOWN` is lost, the request may already be
/// recorded (it cannot be withdrawn), and the command says so instead of "not shut down".
#[test]
fn a_lost_reply_to_the_shutdown_says_it_may_have_been_recorded() {
    let setup = setup();
    let status = setup.status(0, false);
    let _fake = ControlFake::start(
        &setup.paths,
        true,
        Arc::new(move |line: &str| {
            if line.starts_with("SHUTDOWN ") {
                FakeReply::Close
            } else {
                FakeReply::Line(status.encode())
            }
        }),
    );
    let said = run(&setup.options(None, HANG)).expect_err("no reply");
    assert!(said.contains("it may have been recorded"), "{said}");
}

/// Review round 2, Opus S1: the same judgment at the second exchange. The daemon answers the
/// status, then its socket is gone before the `SHUTDOWN` connects: with its owner lock still held
/// it is alive (exit 1), not "gone"; with the lock free it is gone (exit 0).
#[test]
fn a_refused_second_exchange_is_judged_by_the_owner_lock_too() {
    let setup = setup();
    setup.write_pid_file();
    for (held, says) in [
        (true, "alive but not accepting connections"),
        (false, "is gone"),
    ] {
        let owner = open_lock(setup.paths.owner_lock());
        if held {
            owner.try_lock().expect("the owner lock");
        }
        let status = setup.status(0, false);
        let socket = setup.paths.socket().to_path_buf();
        let _fake = ControlFake::start(
            &setup.paths,
            false,
            Arc::new(move |_| {
                let _ = std::fs::remove_file(&socket);
                FakeReply::Line(status.encode())
            }),
        );
        let said = run(&setup.options(None, HANG));
        let line = match &said {
            Ok(line) | Err(line) => line.clone(),
        };
        assert!(line.contains(says), "{said:?}");
        assert_eq!(said.is_err(), held, "{said:?}");
        drop(owner);
    }
}

/// PR #418 review round 1 (sol 1, Opus S2): a runtime directory whose parent this user cannot
/// search (`EACCES` from `lstat`) is not "nothing to shut down": a daemon may be serving there
/// (here, its owner lock held and its pid file written). The command fails (exit 1), saying why.
/// Rules out taking every lookup error for absence. Needs a user other than root, who searches
/// any directory; skipped under root.
#[test]
fn a_runtime_directory_that_cannot_be_looked_at_is_not_taken_for_none() {
    if as_root() {
        eprintln!("skipped: root searches a directory of mode 0");
        return;
    }
    let setup = setup();
    setup.write_pid_file();
    let owner = open_lock(setup.paths.owner_lock());
    owner.try_lock().expect("the owner lock");
    let unsearchable = Unsearchable::new(setup.base.path());
    let said = run(&setup.options(None, HANG)).expect_err("not \"nothing to shut down\"");
    assert!(said.contains("Permission denied"), "{said}");
    assert!(!said.contains("nothing to shut down"), "{said}");
    drop(unsearchable);
    let said = run(&setup.options(None, HANG)).expect_err("alive");
    assert!(
        said.contains("alive but not accepting connections"),
        "{said}"
    );
}

/// PR #418 review round 1 (Opus S2): during the wait for the drain, a pid file that cannot be
/// read for any reason but its absence (here a symlink in its place, which is never followed:
/// `ELOOP`) is not "drained". The command keeps looking and, at the timeout, fails (exit 1)
/// with the error. Rules out taking every open or read error for a removed pid file. Works as
/// root too.
#[test]
fn a_pid_file_that_cannot_be_read_during_the_wait_is_not_taken_for_a_drain() {
    let setup = setup();
    setup.write_pid_file();
    let pid_path = setup.paths.pid_file().to_path_buf();
    let elsewhere = setup.base.path().join("pid-elsewhere");
    let _fake = setup.fake(setup.status(0, false), setup.status(0, true), move || {
        let text = std::fs::read(&pid_path).expect("the pid file");
        std::fs::write(&elsewhere, text).expect("a copy");
        std::fs::remove_file(&pid_path).expect("remove the pid file");
        std::os::unix::fs::symlink(&elsewhere, &pid_path).expect("a symlink in its place");
    });
    let said = run(&setup.options(None, Duration::from_secs(2))).expect_err("not drained");
    assert!(said.contains("its pid file could not be read"), "{said}");
    assert!(said.contains("symbolic links"), "{said}");
}
