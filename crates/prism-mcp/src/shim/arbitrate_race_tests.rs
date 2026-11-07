//! Tests for shims racing to start one daemon (spec §4.1 step 6, §4.6; the shim's halves of §9
//! tests 8 and 15). Each shim runs on a thread of the test with a starter of its own, as each
//! shim process spawns its own daemon; one is paused at a boundary of the start protocol while
//! the others go on.
use super::super::shim_support::{
    always, hello_line, key, runtime, this_user, wait_until, Answer, ChildState, FakeChild,
    FakeDaemon, FakeStarter, Gate,
};
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const LONG: Duration = Duration::from_secs(60);

/// Run a shim for `paths` on a thread with `starter`, its hook `hook`; the thread returns the pid
/// of the daemon it reached and how many daemons it started.
fn shim(
    paths: &DaemonPaths,
    mut starter: FakeStarter,
    hook: Option<Hook>,
) -> std::thread::JoinHandle<(u32, u32)> {
    let paths = paths.clone();
    std::thread::spawn(move || {
        let hello = hello_line(key());
        let shim = Arbiter {
            paths: &paths,
            key: key(),
            hello: &hello,
            deadline: Instant::now() + LONG,
            peer_uid: this_user,
            hook,
        };
        let pid = match shim.arbitrate(&mut starter) {
            Ok(connected) => connected.ready.pid,
            Err(failure) => panic!("no daemon reached: {failure}"),
        };
        (pid, starter.starts.load(Ordering::SeqCst))
    })
}

fn children() -> Arc<Mutex<Vec<Arc<Mutex<ChildState>>>>> {
    Arc::new(Mutex::new(Vec::new()))
}

/// How many of `children` own the key and serve it.
fn serving(children: &Mutex<Vec<Arc<Mutex<ChildState>>>>) -> usize {
    children
        .lock()
        .expect("children")
        .iter()
        .filter(|child| child.lock().expect("child").daemon.is_some())
        .count()
}

/// Spec §9 test 8, the shim's half: one shim is paused holding the start lock, before it spawns,
/// and after it spawns; a second shim started meanwhile waits for the lock (or, once a daemon
/// answers, is admitted by it). When the first goes on, both are admitted by the same daemon
/// and exactly one daemon was started between them. Rules out a second shim that starts its own
/// daemon while the first holds the start lock.
#[test]
fn shims_racing_to_start_a_daemon_start_exactly_one() {
    for boundary in [Point::StartLockTaken, Point::BeforeSpawn, Point::AfterSpawn] {
        let (_base, paths) = runtime();
        let started = children();
        let gate = Gate::new();
        let paused = Arc::new(AtomicBool::new(false));
        let first = shim(
            &paths,
            FakeStarter::claiming(&paths, always(Answer::Ready), Arc::clone(&started)),
            Some({
                let (gate, paused) = (Arc::clone(&gate), Arc::clone(&paused));
                Arc::new(move |point| {
                    if point == boundary && !paused.swap(true, Ordering::SeqCst) {
                        gate.pass();
                    }
                })
            }),
        );
        wait_until("the first shim to reach the boundary", || {
            paused.load(Ordering::SeqCst)
        });
        let waiting = Arc::new(AtomicBool::new(false));
        let second = shim(
            &paths,
            FakeStarter::claiming(&paths, always(Answer::Ready), Arc::clone(&started)),
            Some({
                let waiting = Arc::clone(&waiting);
                Arc::new(move |point| {
                    if point == Point::StartLockBusy {
                        waiting.store(true, Ordering::SeqCst);
                    }
                })
            }),
        );
        if boundary != Point::AfterSpawn {
            wait_until("the second shim to wait for the start lock", || {
                waiting.load(Ordering::SeqCst)
            });
        }
        gate.open();
        let (first_pid, first_starts) = first.join().expect("the first shim");
        let (second_pid, second_starts) = second.join().expect("the second shim");
        assert_eq!(first_pid, second_pid, "{boundary:?}");
        assert_eq!(first_starts + second_starts, 1, "{boundary:?}");
        assert_eq!(serving(&started), 1, "{boundary:?}");
    }
}

/// Spec §9 test 15, the shim's half, and §4.6 "Starter crashes mid-start": a shim dies right
/// after spawning a daemon, before that daemon takes the owner lock, while another shim starts.
/// Whether the orphan takes the key first or the second shim's own daemon does, exactly one
/// daemon serves and two later shims are both admitted by it.
#[test]
fn a_starter_that_dies_after_spawning_leaves_exactly_one_daemon() {
    for orphan_wins in [false, true] {
        let (_base, paths) = runtime();
        let release = Gate::new();
        let orphan = Arc::new(Mutex::new(ChildState::default()));
        // The dying shim's daemon takes the key only when `release` opens.
        let mut dying_starter = FakeStarter::new({
            let (paths, release, orphan) =
                (paths.clone(), Arc::clone(&release), Arc::clone(&orphan));
            move |_| {
                let (paths, release, state) =
                    (paths.clone(), Arc::clone(&release), Arc::clone(&orphan));
                std::thread::spawn(move || {
                    release.pass();
                    let mut child = state.lock().expect("orphan");
                    match FakeDaemon::claim(&paths, 2000, always(Answer::Ready)) {
                        Some(daemon) => child.daemon = Some(daemon),
                        None => child.exit = Some(Exit::Success),
                    }
                });
                Ok(Box::new(FakeChild {
                    pid: 2000,
                    state: Arc::clone(&orphan),
                }) as Box<dyn Started>)
            }
        });
        let hello = hello_line(key());
        let mut dying = Arbiter {
            paths: &paths,
            key: key(),
            hello: &hello,
            deadline: Instant::now() + LONG,
            peer_uid: this_user,
            hook: None,
        };
        dying.hook = Some(Arc::new(|point| {
            if point == Point::AfterSpawn {
                panic!("the starter dies here, on purpose");
            }
        }));
        let died = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dying
                .arbitrate(&mut dying_starter)
                .map(|connected| connected.ready.pid)
        }));
        assert!(died.is_err(), "the starter died");
        if orphan_wins {
            release.open();
            wait_until("the orphan to own the key", || {
                orphan.lock().expect("orphan").daemon.is_some()
            });
        }

        let started = children();
        let (second, _) = shim(
            &paths,
            FakeStarter::claiming(&paths, always(Answer::Ready), Arc::clone(&started)),
            None,
        )
        .join()
        .expect("the second shim");
        if !orphan_wins {
            release.open();
            wait_until("the orphan to find the key taken and end", || {
                orphan.lock().expect("orphan").exit == Some(Exit::Success)
            });
        }
        let (third, third_starts) = shim(
            &paths,
            FakeStarter::claiming(&paths, always(Answer::Ready), Arc::clone(&started)),
            None,
        )
        .join()
        .expect("the third shim");

        let survivor = if orphan_wins { 2000 } else { 1000 };
        assert_eq!(
            (second, third),
            (survivor, survivor),
            "orphan wins: {orphan_wins}"
        );
        assert_eq!(third_starts, 0);
        let orphan_serves = usize::from(orphan.lock().expect("orphan").daemon.is_some());
        assert_eq!(
            orphan_serves + serving(&started),
            1,
            "orphan wins: {orphan_wins}"
        );
    }
}

/// Spec §4.1 step 6 and the module doc: while a daemon this shim started has neither answered nor
/// ended, the shim waits for it and does not look at the owner lock (looking takes the lock for a
/// moment, and a daemon claiming the key in that moment leaves believing another owns it). Here
/// the started daemon takes the key only when the test lets it; until then exactly one daemon has
/// been started. The pause before the gate opens orders nothing a correct shim depends on; it
/// gives a shim that looks again the time to start a second daemon.
#[test]
fn a_started_daemon_that_has_not_taken_the_key_yet_is_waited_for() {
    let (_base, paths) = runtime();
    let release = Gate::new();
    let starter = FakeStarter::new({
        let (paths, release) = (paths.clone(), Arc::clone(&release));
        move |n| {
            let state = Arc::new(Mutex::new(ChildState::default()));
            let (paths, release, child) = (paths.clone(), Arc::clone(&release), Arc::clone(&state));
            std::thread::spawn(move || {
                release.pass();
                let mut child = child.lock().expect("child");
                match FakeDaemon::claim(&paths, 1000 + n, always(Answer::Ready)) {
                    Some(daemon) => child.daemon = Some(daemon),
                    None => child.exit = Some(Exit::Success),
                }
            });
            Ok(Box::new(FakeChild {
                pid: 1000 + n,
                state,
            }) as Box<dyn Started>)
        }
    });
    let starts = Arc::clone(&starter.starts);
    let running = shim(&paths, starter, None);
    wait_until("the shim to start a daemon", || {
        starts.load(Ordering::SeqCst) == 1
    });
    std::thread::sleep(Duration::from_millis(200));
    release.open();
    let (pid, total) = running.join().expect("the shim");
    assert_eq!((pid, total), (1000, 1));
}
