//! Tests for `daemon-status` against stand-in daemons and planted files (S5b; spec §4.7): what
//! each kind of key is reported as, that the report reads and never removes, and that a key a
//! shim is starting is not looked at under its owner lock.
use super::super::control_support::{
    always_line, fake_status, mkfifo, within_hang, ControlFake, FakeReply,
};
use super::super::shim_support::{
    full_backlog, key, open_lock, runtime, this_user, unix_now, HANG,
};
use super::*;
use crate::mcp::shared::preamble::Reply;
use crate::mcp::CacheMode;
use std::sync::Arc;

const LOCATOR: &str = "cdcdcdcdcdcdcdcd";

fn options(base: &Path, timeout: Duration) -> StatusOptions {
    StatusOptions {
        config: None,
        runtime_base: Some(base.to_path_buf()),
        timeout,
    }
}

fn report(base: &Path) -> String {
    status_report(&options(base, HANG), this_user).expect("a report")
}

/// What the report says about the test's key: its lines after the first.
fn about_the_key(base: &Path, paths: &DaemonPaths) -> String {
    let report = report(base);
    let head = format!("runtime dir {}\n", paths.runtime_dir().display());
    report
        .strip_prefix(&head)
        .unwrap_or_else(|| panic!("starts with the runtime dir: {report}"))
        .to_string()
}

fn write_pid_file(paths: &DaemonPaths) {
    std::fs::write(paths.pid_file(), "4242\n1760600000\nbuild\n").expect("write the pid file");
}

/// A socket file nobody listens on, as a daemon that died leaves it.
fn stale_socket(paths: &DaemonPaths) {
    drop(std::os::unix::net::UnixListener::bind(paths.socket()).expect("bind"));
}

/// Spec §4.7: a daemon that answers is listed with its instance, root, build, clients, build state
/// and idle time.
#[test]
fn a_daemon_that_answers_is_listed_with_its_instance_and_state() {
    let (base, paths) = runtime();
    let _fake = ControlFake::start(&paths, true, always_line(fake_status(1, false).encode()));
    assert_eq!(
        about_the_key(base.path(), &paths),
        format!(
            "{LOCATOR} pid 4242, started 1760600000, serving /work/repo\n  key {}; build {}\n  \
             clients: 1 admitted, 0 pending, at most 64; in use\n  index: ready (attempt 1, \
             generation 2)\n",
            key(),
            "b".repeat(64)
        )
    );
}

/// An idle daemon shows how long it has been idle and when it retires, and a requested
/// shutdown is shown.
#[test]
fn an_idle_daemon_shows_its_idle_time_and_a_requested_shutdown() {
    let (base, paths) = runtime();
    let _fake = ControlFake::start(&paths, true, always_line(fake_status(0, true).encode()));
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.contains("; idle 7 s; retires after 900 s idle\n"),
        "{said}"
    );
    assert!(
        said.ends_with("  shutdown requested: it drains once no client is left\n"),
        "{said}"
    );
}

/// `RETIRING` and `BUSY` are reported as what they are.
#[test]
fn a_draining_or_busy_daemon_is_named_as_such() {
    let (base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, always_line(Reply::Retiring.encode()));
    assert_eq!(
        about_the_key(base.path(), &paths),
        format!("{LOCATOR} draining: it answered RETIRING\n")
    );
    fake.retire();
    let _fake = ControlFake::start(
        &paths,
        true,
        always_line(Reply::Busy { max_clients: 2 }.encode()),
    );
    assert_eq!(
        about_the_key(base.path(), &paths),
        format!("{LOCATOR} busy: it answered BUSY at its bound of 2 clients, so no status\n")
    );
}

/// A daemon that holds its owner lock and does not answer (a wedged daemon, or one older than
/// the control exchange) is named alive, with what its pid file says.
#[test]
fn a_daemon_that_does_not_answer_is_named_alive_with_its_pid_file() {
    let (base, paths) = runtime();
    write_pid_file(&paths);
    let _fake = ControlFake::start(&paths, true, Arc::new(|_| FakeReply::Silent));
    let said = status_report(&options(base.path(), Duration::from_millis(300)), this_user)
        .expect("a report");
    assert!(
        said.contains(&format!(
            "{LOCATOR} alive but not answering (the daemon accepted but sent no reply); pid file \
             {}: pid 4242, start time 1760600000, build build\n",
            paths.pid_file().display()
        )),
        "{said}"
    );
}

/// A key no daemon owns is reported with what a dead daemon left, and nothing is removed: the
/// next shim of the key removes it under the owner lock (owner question 1). Rules out a status
/// that cleans up.
#[test]
fn a_dead_daemon_s_leftovers_are_named_and_left_in_place() {
    let (base, paths) = runtime();
    stale_socket(&paths);
    write_pid_file(&paths);
    let said = about_the_key(base.path(), &paths);
    assert_eq!(
        said,
        format!(
            "{LOCATOR} no daemon; a dead daemon left its socket and its pid file ({}: \
             pid 4242, start time 1760600000, build build)\n",
            paths.pid_file().display()
        )
    );
    assert!(std::fs::symlink_metadata(paths.socket()).is_ok());
    assert!(paths.pid_file().exists());
    let owner = open_lock(paths.owner_lock());
    assert!(owner.try_lock().is_ok(), "the owner lock was released");
}

/// S5a's interface for S5b: a key whose daemon died during its build says when a shim may start
/// it again (60 s from the first sighting), and the report records no sighting: only a shim
/// does. Rules out a status that starts the minute itself.
#[test]
fn a_daemon_that_died_during_its_build_says_when_it_may_restart() {
    let (base, paths) = runtime();
    let unseen = "4242\n1760000000\n";
    std::fs::write(paths.build_marker(), unseen).expect("plant a marker");
    let said = about_the_key(base.path(), &paths);
    assert_eq!(
        said,
        format!(
            "{LOCATOR} no daemon; its last daemon (pid 4242) died during its index build (started \
             1760000000); no shim has seen it yet: the next one records the sighting and waits \
             60 s\n"
        )
    );
    assert_eq!(
        std::fs::read_to_string(paths.build_marker()).expect("the marker"),
        unseen,
        "the report recorded nothing"
    );
    let seen = unix_now() - 21;
    std::fs::write(paths.build_marker(), format!("4242\n1760000000\n{seen}\n")).expect("marker");
    assert!(about_the_key(base.path(), &paths).contains(&format!(
        "first seen by a shim at {seen}; a shim restarts it in 39 s\n"
    )));
}

/// D102: while a shim holds a key's start lock (it may have spawned a daemon that has not taken
/// the owner lock yet), the report does not look at the owner lock, which would make that daemon
/// believe another owns the key. Rules out a look at the owner lock without the start lock.
#[test]
fn a_key_a_shim_is_starting_is_not_looked_at_under_its_owner_lock() {
    let (base, paths) = runtime();
    write_pid_file(&paths);
    let start = open_lock(paths.start_lock());
    start.try_lock().expect("the start lock");
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.starts_with(&format!(
            "{LOCATOR} being started or reached: a shim holds its start lock; "
        )),
        "{said}"
    );
}

/// Keys that have nothing of a daemon's left (every rebuild of prism leaves lock files and a log
/// per repository, S4's D70) are counted in one line with the size of their logs; files that are
/// not prism's are ignored; nothing is removed.
#[test]
fn keys_with_only_lock_files_and_logs_are_counted_not_listed() {
    let (base, paths) = runtime();
    let dir = paths.runtime_dir();
    for name in [
        "1111111111111111.start",
        "1111111111111111.owner",
        "2222222222222222.owner",
    ] {
        std::fs::write(dir.join(name), "").expect("a lock file");
    }
    std::fs::write(dir.join("1111111111111111.log"), "0123456789").expect("a log");
    for name in [
        "README",
        "123.sock",
        "ABCDEFABCDEFABCD.sock",
        "3333333333333333.tmp",
    ] {
        std::fs::write(dir.join(name), "x").expect("a stray file");
    }
    assert_eq!(
        about_the_key(base.path(), &paths),
        "2 keys have nothing but lock files and logs (10 bytes of logs)\n"
    );
    assert_eq!(
        std::fs::read_dir(dir).expect("list").count(),
        8,
        "nothing removed"
    );
}

/// `--repo` shows that configuration's daemon only, and for a configuration with no daemon says
/// so without creating any of its files.
#[test]
fn with_a_repository_only_its_daemon_is_shown() {
    let (base, paths) = runtime();
    let _other = ControlFake::start(&paths, true, always_line(fake_status(1, false).encode()));
    let repo = tempfile::tempdir().expect("a repository");
    let cache = tempfile::tempdir().expect("a cache base");
    let mut config = ServerConfig::new(repo.path().to_path_buf());
    config.cache = CacheMode::Dir(cache.path().to_path_buf());
    let wanted = key_of(&config).expect("a key");
    let mut options = options(base.path(), HANG);
    options.config = Some(config);
    let said = status_report(&options, this_user).expect("a report");
    assert_eq!(
        said,
        format!(
            "runtime dir {}\n{} no daemon for this configuration\n",
            paths.runtime_dir().display(),
            wanted.locator()
        )
    );
    let mine = DaemonPaths::new(paths.runtime_dir(), &wanted).expect("paths");
    for path in [mine.start_lock(), mine.owner_lock()] {
        assert!(
            std::fs::symlink_metadata(path).is_err(),
            "{}",
            path.display()
        );
    }
}

/// A runtime directory that does not exist is reported and not created; one that fails
/// validation is an error, and nothing in it is read.
#[test]
fn a_missing_or_invalid_runtime_directory() {
    let base = tempfile::Builder::new()
        .prefix("s5b")
        .tempdir_in("/tmp")
        .expect("a runtime base");
    assert!(report(base.path()).ends_with("no daemon (the runtime directory does not exist)\n"));
    assert!(std::fs::symlink_metadata(base.path().join("prism-mcpd")).is_err());

    use std::os::unix::fs::PermissionsExt;
    let dir = base.path().join("prism-mcpd");
    std::fs::create_dir(&dir).expect("create");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let error = status_report(&options(base.path(), HANG), this_user).expect_err("invalid");
    assert!(error.contains("prism-mcpd"), "{error}");
}

/// An empty runtime directory has no daemon.
#[test]
fn an_empty_runtime_directory_has_no_daemon() {
    let (base, paths) = runtime();
    assert_eq!(about_the_key(base.path(), &paths), "no daemon\n");
}

/// Nothing the report prints carries a control character, whatever a daemon reports: a root may
/// hold any byte. Rules out printing what a daemon sent as it came.
#[test]
fn a_root_with_control_characters_is_printed_on_one_line() {
    let (base, paths) = runtime();
    let mut status = fake_status(0, false);
    status.repo_root = "/w/a\nb\u{1b}[31m".to_string();
    let _fake = ControlFake::start(&paths, true, always_line(status.encode()));
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.starts_with(&format!(
            "{LOCATOR} pid 4242, started 1760600000, serving /w/a\\nb\\u{{1b}}[31m\n"
        )),
        "{said:?}"
    );
    assert!(!said.contains('\u{1b}'), "{said:?}");
}

/// Review round 1, sol 2 and Opus S6: a build marker is read without waiting and only so far. A
/// FIFO in its place (a blocking open would wait for a writer for ever, holding the key's start
/// and owner locks) is reported as unusable at once; a marker of 4 MiB is read only as far as a
/// daemon's marker reaches. Watched by an independent thread, because the failure is a hang.
#[test]
fn a_build_marker_that_is_a_fifo_or_oversized_does_not_hold_the_report() {
    let (base, paths) = runtime();
    mkfifo(paths.build_marker());
    let root = base.path().to_path_buf();
    let said = within_hang("daemon-status over a FIFO marker", move || report(&root));
    assert!(
        said.contains(&format!(
            "{LOCATOR} no daemon; its build marker is not usable ("
        )),
        "{said}"
    );
    let owner = open_lock(paths.owner_lock());
    assert!(owner.try_lock().is_ok(), "the owner lock was released");
    drop(owner);

    std::fs::remove_file(paths.build_marker()).expect("remove the FIFO");
    // Two whole lines, then a third line that ends 4 MiB in: read whole, it would be a sighting
    // at time 0; only its first 256 bytes are read, so it is no sighting.
    let mut big = b"4242\n1760000000\n".to_vec();
    big.resize(4 << 20, b'0');
    big.push(b'\n');
    std::fs::write(paths.build_marker(), big).expect("an oversized marker");
    let root = base.path().to_path_buf();
    let said = within_hang("daemon-status over a 4 MiB marker", move || report(&root));
    assert!(
        said.contains(
            "its last daemon (pid 4242) died during its index build (started 1760000000); no \
             shim has seen it yet"
        ),
        "{said}"
    );
}

/// Review round 1, sol 4: the runtime directory's own path is printed on one line too.
#[test]
fn a_runtime_directory_with_control_characters_is_printed_on_one_line() {
    let base = tempfile::Builder::new()
        .prefix("s5b")
        .tempdir_in("/tmp")
        .expect("a runtime base");
    let odd = base.path().join("a\nb\u{1b}c");
    std::fs::create_dir(&odd).expect("an odd base");
    let said = report(&odd);
    assert_eq!(
        said,
        format!(
            "runtime dir {}/a\\nb\\u{{1b}}c/prism-mcpd\nno daemon (the runtime directory does not \
             exist)\n",
            base.path().display()
        )
    );
}

/// Review round 1, Opus S7: a key whose files cannot be opened (a symlink in place of its start
/// lock) is reported on its own line, and the other keys are still listed. Rules out one bad key
/// failing the whole report.
#[test]
fn a_key_that_cannot_be_looked_at_does_not_hide_the_others() {
    let (base, paths) = runtime();
    let _fake = ControlFake::start(&paths, true, always_line(fake_status(1, false).encode()));
    let dir = paths.runtime_dir();
    std::fs::write(dir.join("1111111111111111.pid"), "1\n2\nb\n").expect("a pid file");
    std::os::unix::fs::symlink("/dev/null", dir.join("1111111111111111.start")).expect("symlink");
    let said = report(base.path());
    assert!(
        said.contains("\n1111111111111111 cannot be looked at: "),
        "{said}"
    );
    assert!(
        said.contains(&format!("\n{LOCATOR} pid 4242, started 1760600000")),
        "{said}"
    );
}

/// Review round 1, Opus W1: a live daemon whose socket refuses connections (its backlog is full)
/// is named alive, with the connect error, not "no socket"; one without a socket says so.
#[test]
fn a_live_daemon_that_does_not_accept_is_named_with_why() {
    let (base, paths) = runtime();
    write_pid_file(&paths);
    let owner = open_lock(paths.owner_lock());
    owner.try_lock().expect("the owner lock");
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.starts_with(&format!(
            "{LOCATOR} alive but not answering (it has no socket: starting or draining); "
        )),
        "{said}"
    );
    let (_listener, _clients) = full_backlog(paths.socket());
    let said = about_the_key(base.path(), &paths);
    assert!(
        said.starts_with(&format!(
            "{LOCATOR} alive but not answering (connect failed: "
        )),
        "{said}"
    );
}
