//! Tests for finding and starting the daemon (spec §4.1 steps 4–7, §4.6, §10; the shim's halves
//! of §9 tests 3, 4, 5, 10 and 13; the restart bound of S3b's SI-10). Daemons are fakes on
//! threads of the test that take the real owner lock on a descriptor of their own, as a daemon
//! process does; a starter that the test controls stands in for spawning one.
use super::super::shim_support::{
    always, another_user, full_backlog, hello_line, key, open_lock, runtime, this_user, unix_now,
    Answer, ChildState, FakeDaemon, FakeStarter,
};
use super::*;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

const LONG: Duration = Duration::from_secs(60);
/// A deadline a test means to expire while it asserts `Owner::Held`: the shim must still reach
/// the owner lock (first reach, key directory, start lock, second reach) before it, so a ½ s
/// stall of the test thread must not cost the look (S5a-1 review, Opus S2).
const HELD_DEADLINE: Duration = Duration::from_secs(2);

fn arbiter<'a>(paths: &'a DaemonPaths, hello: &'a str, within: Duration) -> Arbiter<'a> {
    Arbiter {
        paths,
        key: key(),
        hello,
        deadline: Instant::now() + within,
        peer_uid: this_user,
        hook: None,
    }
}

type Children = Arc<Mutex<Vec<Arc<Mutex<ChildState>>>>>;

fn children() -> Children {
    Arc::new(Mutex::new(Vec::new()))
}

/// A starter whose every start would be a mistake.
fn must_not_start() -> FakeStarter {
    FakeStarter::exiting(Exit::Failure("this test starts no daemon".into()))
}

fn inode(path: &Path) -> u64 {
    std::fs::symlink_metadata(path).expect("lstat").ino()
}

fn pid_of(result: Result<Connected, Failure>) -> u32 {
    match result {
        Ok(connected) => connected.ready.pid,
        Err(failure) => panic!("no daemon reached: {failure}"),
    }
}

/// Whether some process (this test, playing one) could take the owner lock now.
fn owner_is_free(paths: &DaemonPaths) -> bool {
    open_lock(paths.owner_lock()).try_lock().is_ok()
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).expect("write a key file");
}

/// Spec §4.1 step 5: a live daemon is reached by the first attempt, before the start lock is
/// even opened, and nothing is started.
#[test]
fn a_live_daemon_is_reached_without_the_start_lock_or_a_start() {
    let (_base, paths) = runtime();
    let _daemon = FakeDaemon::claim(&paths, 7, always(Answer::Ready)).expect("the key");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert_eq!(
        pid_of(arbiter(&paths, &hello, LONG).arbitrate(&mut starter)),
        7
    );
    assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
    assert!(
        !paths.start_lock().exists(),
        "the start lock was not opened"
    );
}

/// Spec §4.1 step 6: with no daemon, the shim starts one under the start lock, is admitted by
/// it, and hands back the started daemon; the start lock is free again afterwards.
#[test]
fn with_no_daemon_one_is_started_and_reached() {
    let (_base, paths) = runtime();
    let hello = hello_line(key());
    let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
    let connected = arbiter(&paths, &hello, LONG)
        .arbitrate(&mut starter)
        .map_err(|failure| failure.to_string())
        .expect("reached");
    assert_eq!(connected.ready.pid, 1000);
    assert_eq!(
        connected.started.as_ref().map(|child| child.pid()),
        Some(1000)
    );
    assert_eq!(starter.starts.load(Ordering::SeqCst), 1);
    drop(connected);
    assert!(open_lock(paths.start_lock()).try_lock().is_ok());
}

/// Spec §9 test 3, the shim's half, and §4.1 step 6: a socket and a pid file left by a dead
/// daemon (no owner lock held) are removed while the shim holds the owner lock, and the lock is
/// released before the daemon is started. Rules out a shim that leaves them, and one that spawns
/// still holding the owner lock (its child could never own the key).
#[test]
fn a_stale_socket_and_pid_file_are_removed_under_the_owner_lock_before_the_start() {
    let (_base, paths) = runtime();
    drop(UnixListener::bind(paths.socket()).expect("a stale socket"));
    write(paths.pid_file(), "4242\n17\nbuild\n");
    let seen = Arc::new(Mutex::new(None));
    let hello = hello_line(key());
    let mut shim = arbiter(&paths, &hello, LONG);
    shim.hook = Some({
        let (paths, seen) = (paths.clone(), Arc::clone(&seen));
        Arc::new(move |point| {
            if point == Point::BeforeSpawn {
                *seen.lock().expect("seen") = Some((
                    paths.socket().exists(),
                    paths.pid_file().exists(),
                    owner_is_free(&paths),
                ));
            }
        })
    });
    let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
    assert_eq!(pid_of(shim.arbitrate(&mut starter)), 1000);
    assert_eq!(
        *seen.lock().expect("seen"),
        Some((false, false, true)),
        "(socket there, pid file there, owner lock free) just before the start"
    );
}

/// Spec §9 test 4, the shim's half, and §4.4: a live daemon that accepts and never answers is
/// reported at the deadline and never replaced: its socket and pid file are left alone and
/// nothing is started. Also when it closes every connection instead (the owner lock is then
/// looked at, found held, and still nothing is touched).
#[test]
fn a_live_but_wedged_owner_is_reported_and_its_files_are_left_alone() {
    for answer in [Answer::Silent, Answer::Close] {
        let (_base, paths) = runtime();
        let _daemon = FakeDaemon::claim(&paths, 4242, always(answer)).expect("the key");
        write(paths.pid_file(), "4242\n17\nbuild\n");
        let socket = inode(paths.socket());
        let hello = hello_line(key());
        let mut starter = must_not_start();
        // Close asserts `Owner::Held`, which the shim reports only if it reaches the owner lock
        // before the deadline; 2 s keeps a stalled test thread from failing a correct shim
        // (S5a-1 review, Opus S2). Silent asserts only `NoReply`, which any stall still gives.
        let within = match answer {
            Answer::Close => HELD_DEADLINE,
            _ => Duration::from_millis(500),
        };
        let result = arbiter(&paths, &hello, within).arbitrate(&mut starter);
        match (answer, result) {
            (
                Answer::Silent,
                Err(Failure::NoReady {
                    last: Transient::NoReply,
                    ..
                }),
            ) => {}
            (
                Answer::Close,
                Err(Failure::NoReady {
                    owner: Owner::Held, ..
                }),
            ) => {}
            (answer, other) => panic!("{answer:?}: {:?}", other.map(|c| c.ready.pid)),
        }
        assert_eq!(inode(paths.socket()), socket);
        assert_eq!(
            std::fs::read_to_string(paths.pid_file()).expect("pid file"),
            "4242\n17\nbuild\n"
        );
        assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
    }
}

/// The retirement window (S4's interface): a daemon removes its socket a moment before it
/// releases the owner lock. A shim that arrives in between finds no socket and the lock held; it
/// waits, and once the lock is free it starts a daemon. Rules out a shim that gives up on a held
/// lock, and one that never looks again.
#[test]
fn a_key_whose_owner_is_releasing_it_is_started_once_the_lock_is_free() {
    let (_base, paths) = runtime();
    let retiring = FakeDaemon::claim(&paths, 7, always(Answer::Ready)).expect("the key");
    std::fs::remove_file(paths.socket()).expect("the socket is gone first");
    let retiring = Arc::new(Mutex::new(Some(retiring)));
    let hello = hello_line(key());
    let mut shim = arbiter(&paths, &hello, LONG);
    shim.hook = Some({
        let retiring = Arc::clone(&retiring);
        Arc::new(move |point| {
            if point == Point::OwnerHeld {
                // The retiring daemon releases its lock (and ends) after the shim saw it held.
                drop(retiring.lock().expect("retiring").take());
            }
        })
    });
    let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
    assert_eq!(pid_of(shim.arbitrate(&mut starter)), 1000);
    assert!(
        retiring.lock().expect("retiring").is_none(),
        "the lock was seen held"
    );
    assert_eq!(starter.starts.load(Ordering::SeqCst), 1);
}

/// Spec §9 test 13, the shim's half: a shim paused after `connect` and before `HELLO` while the
/// daemon retires is answered `RETIRING`, or sees its connection end, re-enters arbitration and
/// is admitted by a fresh daemon within its original deadline. Rules out failing closed on
/// either.
#[test]
fn a_shim_caught_by_a_retirement_is_admitted_by_a_fresh_daemon() {
    for answer in [Answer::Retiring, Answer::Close] {
        let (_base, paths) = runtime();
        let old = FakeDaemon::claim(&paths, 7, always(answer)).expect("the key");
        let old = Arc::new(Mutex::new(Some(old)));
        let hello = hello_line(key());
        let mut shim = arbiter(&paths, &hello, LONG);
        shim.hook = Some({
            let old = Arc::clone(&old);
            Arc::new(move |point| {
                if point == Point::AfterConnect {
                    // Paused after connect: the daemon retires (socket removed, lock released);
                    // the connection it accepted is still answered by its script.
                    if let Some(old) = old.lock().expect("old").take() {
                        old.retire();
                    }
                }
            })
        });
        let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
        assert_eq!(pid_of(shim.arbitrate(&mut starter)), 1000, "{answer:?}");
        assert_eq!(starter.starts.load(Ordering::SeqCst), 1);
    }
}

/// S4: a daemon that finds the key owned exits 0 silently. A daemon the shim started that ends
/// that way, before `READY`, lost the key; the shim looks at the owner lock again and, finding it
/// free, starts another.
#[test]
fn a_started_daemon_that_lost_the_key_is_followed_by_another_start() {
    let (_base, paths) = runtime();
    let hello = hello_line(key());
    let mut claiming = FakeStarter::claiming(&paths, always(Answer::Ready), children());
    let mut starter = FakeStarter::new(move |n| {
        if n == 0 {
            FakeStarter::exiting(Exit::Success).start_fake()
        } else {
            claiming.start_fake()
        }
    });
    assert_eq!(
        pid_of(arbiter(&paths, &hello, LONG).arbitrate(&mut starter)),
        1000
    );
    assert_eq!(starter.starts.load(Ordering::SeqCst), 2);
}

/// Spec §10 "Daemon spawn fails": a daemon the shim started that ends unsuccessfully before
/// `READY` ends the start at once, naming it; it is not started again. Rules out retrying such a
/// daemon to the deadline.
#[test]
fn a_started_daemon_that_fails_before_ready_ends_the_start() {
    let (_base, paths) = runtime();
    let hello = hello_line(key());
    let mut starter = FakeStarter::exiting(Exit::Failure("exit status: 1".into()));
    match arbiter(&paths, &hello, Duration::from_secs(5)).arbitrate(&mut starter) {
        Err(Failure::DaemonExited { pid: 1000, exit }) => assert_eq!(exit, "exit status: 1"),
        other => panic!("{:?}", other.map(|c| c.ready.pid)),
    }
    assert_eq!(starter.starts.load(Ordering::SeqCst), 1);
}

/// Spec §10: a daemon that cannot be spawned fails the start closed.
#[test]
fn a_spawn_error_fails_closed() {
    let (_base, paths) = runtime();
    let hello = hello_line(key());
    let mut starter = FakeStarter::new(|_| Err(std::io::Error::other("no such program")));
    assert!(matches!(
        arbiter(&paths, &hello, LONG).arbitrate(&mut starter),
        Err(Failure::SpawnFailed(_))
    ));
}

/// Spec §10 "Start lock not acquired by the deadline": another shim holds the start lock until
/// the deadline; this one fails closed and starts nothing.
#[test]
fn a_start_lock_held_until_the_deadline_fails_closed() {
    let (_base, paths) = runtime();
    let held = open_lock(paths.start_lock());
    held.try_lock().expect("the test holds the start lock");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert!(matches!(
        arbiter(&paths, &hello, Duration::from_millis(300)).arbitrate(&mut starter),
        Err(Failure::StartLockTimeout { .. })
    ));
    assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
    drop(held);
}

/// S3b's SI-10 as review round 1 (sol 3) fixed it: a key whose last daemon died during its index
/// build is not restarted for a minute counted from when a shim first sees the dead build, however
/// long that build ran (here it began 70 s ago). The shim records that moment in the marker (it
/// holds the start and owner locks) and fails closed, leaving the dead daemon's files; a second
/// shim in that minute is refused too and does not move the moment. Rules out a bound counted
/// from the build's start, and one renewed by each refusal.
#[test]
fn a_dead_build_holds_the_key_for_a_minute_from_when_a_shim_first_sees_it() {
    let (_base, paths) = runtime();
    drop(UnixListener::bind(paths.socket()).expect("a stale socket"));
    write(paths.pid_file(), "4242\n17\nbuild\n");
    write(
        paths.build_marker(),
        &format!("4242\n{}\n", unix_now() - 70),
    );
    let hello = hello_line(key());
    let mut starter = must_not_start();
    let refused = |starter: &mut FakeStarter| {
        assert!(matches!(
            arbiter(&paths, &hello, LONG).arbitrate(starter),
            Err(Failure::RecentBuildCrash { .. })
        ));
        std::fs::read_to_string(paths.build_marker()).expect("the marker")
    };
    let first = refused(&mut starter);
    let lines: Vec<&str> = first.lines().collect();
    assert_eq!(lines.len(), 3, "{first:?}");
    let seen: u64 = lines[2].parse().expect("when it was first seen");
    assert!(unix_now().saturating_sub(seen) < 30, "{first:?}");
    // Move the sighting back five seconds; a refusal that renewed it would move it forward.
    let earlier = format!("{}\n{}\n{}\n", lines[0], lines[1], seen - 5);
    write(paths.build_marker(), &earlier);
    assert_eq!(refused(&mut starter), earlier);
    assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
    assert!(paths.socket().exists());
    assert!(paths.pid_file().exists());
    assert!(owner_is_free(&paths), "the owner lock was released");
}

/// A marker caught half-written by a death is still a dead build: it holds the key and the
/// sighting is recorded, with the pid unknown.
#[test]
fn a_partial_build_marker_holds_the_key_too() {
    let (_base, paths) = runtime();
    write(paths.build_marker(), "42");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert!(matches!(
        arbiter(&paths, &hello, LONG).arbitrate(&mut starter),
        Err(Failure::RecentBuildCrash { .. })
    ));
    let marker = std::fs::read_to_string(paths.build_marker()).expect("the marker");
    assert_eq!(marker.lines().count(), 3, "{marker:?}");
}

/// Review round 2 (sol 1, owner decision A): the minute is counted on the wall clock in UTC epoch
/// seconds from the recorded sighting. A sighting 21 s ago leaves 39 s, and the marker's pid is
/// named. (The bound allows for the clock's second turning over between the test's reading and
/// the shim's; it is exactly 39 s when it does not.) Rules out a minute restarted by each look.
#[test]
fn a_recorded_sighting_leaves_the_rest_of_the_minute() {
    let (_base, paths) = runtime();
    let before = unix_now();
    write(
        paths.build_marker(),
        &format!("4242\n{}\n{}\n", before - 300, before - 21),
    );
    let hello = hello_line(key());
    let mut starter = must_not_start();
    let result = arbiter(&paths, &hello, LONG).arbitrate(&mut starter);
    let turned = unix_now() - before;
    match result {
        Err(Failure::RecentBuildCrash {
            pid: Some(4242),
            left_secs,
        }) => assert!(
            (39_u64.saturating_sub(turned)..=39).contains(&left_secs),
            "{left_secs} s left"
        ),
        other => panic!("not refused for a recent build crash: {:?}", other.err()),
    }
}

/// Review round 2 (Opus R2-S3): a sighting recorded over a marker whose pid could not be read
/// leaves the pid unknown, so a later shim names no pid (not "pid 0").
#[test]
fn a_sighting_over_a_marker_with_no_pid_leaves_the_pid_unknown() {
    let (_base, paths) = runtime();
    write(paths.build_marker(), "42");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    for shim in ["first", "later"] {
        match arbiter(&paths, &hello, LONG).arbitrate(&mut starter) {
            Err(failure @ Failure::RecentBuildCrash { pid: None, .. }) => {
                assert!(!failure.to_string().contains("pid"), "{shim}: {failure}");
            }
            other => panic!(
                "{shim}: not refused with the pid unknown: {:?}",
                other.err()
            ),
        }
    }
}

/// The minute ends: a sighting a minute or more ago, or one dated in the future (a clock set
/// back, which must not hold a key closed for as long as the clock is off), lets the next shim
/// remove the stale files and start a daemon.
#[test]
fn an_expired_or_future_sighting_does_not_hold_the_key() {
    for marker in [
        format!("4242\n{}\n{}\n", unix_now() - 300, unix_now() - 70),
        format!("4242\n{}\n{}\n", unix_now() - 300, unix_now() + 3600),
    ] {
        let (_base, paths) = runtime();
        write(paths.build_marker(), &marker);
        let hello = hello_line(key());
        let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
        assert_eq!(
            pid_of(arbiter(&paths, &hello, LONG).arbitrate(&mut starter)),
            1000,
            "{marker:?}"
        );
        assert!(!paths.build_marker().exists(), "{marker:?}");
    }
}

/// Spec §9 test 10 (in process): a daemon that answers `MISMATCH` fails the start closed at once;
/// nothing is started in its place. And the peer check on the daemon at the socket.
#[test]
fn a_mismatch_or_a_peer_of_another_user_fails_closed_without_a_start() {
    let (_base, paths) = runtime();
    let _daemon = FakeDaemon::claim(&paths, 7, always(Answer::Mismatch)).expect("the key");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert!(matches!(
        arbiter(&paths, &hello, LONG).arbitrate(&mut starter),
        Err(Failure::Refused(Refusal::Mismatch { .. }))
    ));
    drop(_daemon);
    std::fs::remove_file(paths.socket()).expect("remove the socket");

    let _daemon = FakeDaemon::claim(&paths, 7, always(Answer::Ready)).expect("the key");
    let mut shim = arbiter(&paths, &hello, LONG);
    shim.peer_uid = another_user;
    assert!(matches!(
        shim.arbitrate(&mut starter),
        Err(Failure::Refused(Refusal::ForeignPeer { .. }))
    ));
    assert_eq!(starter.starts.load(Ordering::SeqCst), 0);
}

/// Spec §9 test 5 (in process): a runtime directory that fails validation (here a symlink to a
/// directory of this user's) fails the start closed before any file is opened or created in it.
#[test]
fn an_invalid_runtime_directory_fails_closed_before_any_file_is_made() {
    let (base, paths) = runtime();
    let elsewhere = base.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).expect("a directory");
    std::fs::remove_dir(paths.runtime_dir()).expect("remove the runtime directory");
    std::os::unix::fs::symlink(&elsewhere, paths.runtime_dir()).expect("a symlink in its place");
    let hello = hello_line(key());
    let mut starter = must_not_start();
    assert!(matches!(
        arbiter(&paths, &hello, LONG).arbitrate(&mut starter),
        Err(Failure::Files(FilesError::RuntimeDir(_)))
    ));
    assert_eq!(
        std::fs::read_dir(&elsewhere).expect("read").count(),
        0,
        "nothing was created through the symlink"
    );
}

/// Review round 1, sol 4: a live daemon (it holds the owner lock) that stops accepting until its
/// listen backlog is full must not hold the shim past its deadline. On Linux a blocking `connect`
/// waits for room in the backlog for ever; the shim's connect is non-blocking and fails closed
/// at the deadline, naming the held owner lock. Rules out a blocking `connect`.
#[test]
fn a_live_daemon_with_a_full_backlog_fails_the_shim_closed_at_the_deadline() {
    let (_base, paths) = runtime();
    let owner = open_lock(paths.owner_lock());
    owner.try_lock().expect("the test plays the live daemon");
    let (_listener, _clients) = full_backlog(paths.socket());
    let (tx, rx) = std::sync::mpsc::channel();
    let shim_paths = paths.clone();
    std::thread::spawn(move || {
        let hello = hello_line(key());
        let mut starter = must_not_start();
        // `Owner::Held` needs the owner lock reached before the deadline (see HELD_DEADLINE).
        let result = arbiter(&shim_paths, &hello, HELD_DEADLINE).arbitrate(&mut starter);
        let _ = tx.send(matches!(
            result,
            Err(Failure::NoReady {
                owner: Owner::Held,
                ..
            })
        ));
    });
    let failed_closed = rx
        .recv_timeout(super::super::shim_support::HANG)
        .expect("the shim gives up at its deadline instead of blocking in connect");
    assert!(failed_closed);
    drop(owner);
}

/// Review of S5a-1, Opus S4: after a spawn the shim stops looking at the owner lock (D82), so
/// its last look, "free", came before its own daemon started; that daemon may hold the lock
/// now. The line says only what the shim saw and when, never that no daemon held it at the
/// deadline. Without a spawn the plain wording stays.
#[test]
fn a_free_owner_lock_seen_before_a_spawn_is_reported_as_seen_before_the_spawn() {
    let after_spawn = Failure::NoReady {
        last: Transient::Closed,
        owner: Owner::Free,
        started: Some(4242),
    }
    .to_string();
    assert!(
        after_spawn.contains(
            "; the owner lock was free when this shim last looked, before it started its daemon"
        ),
        "{after_spawn}"
    );
    assert!(
        !after_spawn.contains("no daemon held the owner lock"),
        "{after_spawn}"
    );
    assert!(
        after_spawn.contains("; the daemon this shim started, pid 4242, did not answer"),
        "{after_spawn}"
    );
    let without_spawn = Failure::NoReady {
        last: Transient::Closed,
        owner: Owner::Free,
        started: None,
    }
    .to_string();
    assert!(
        without_spawn.contains("; no daemon held the owner lock)"),
        "{without_spawn}"
    );
}

/// S5b review round 1 (sol 2, Opus S6): a build marker that is not a regular file (a FIFO here)
/// is not a daemon's. Reading it does not wait for a writer, it does not hold the key, and the
/// shim removes it with the other stale files and starts a daemon. Watched by an independent
/// thread, because a blocking open of the FIFO waited for a writer for ever.
#[test]
fn a_build_marker_that_is_not_a_regular_file_neither_hangs_nor_holds_the_key() {
    let (base, paths) = runtime();
    super::super::control_support::mkfifo(paths.build_marker());
    let marker = paths.build_marker().to_path_buf();
    // The runtime directory comes back from the worker and lives through the last assertion, so
    // that it shows the shim's removal, not the fixture's cleanup (review round 2, sol).
    let (pid, _base) =
        super::super::control_support::within_hang("a shim over a FIFO marker", move || {
            let hello = hello_line(key());
            let mut starter = FakeStarter::claiming(&paths, always(Answer::Ready), children());
            let pid = pid_of(arbiter(&paths, &hello, LONG).arbitrate(&mut starter));
            (pid, base)
        });
    assert_eq!(pid, 1000);
    assert!(
        std::fs::symlink_metadata(&marker).is_err(),
        "the FIFO was removed"
    );
}
