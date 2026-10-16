//! Tests for the build marker around the daemon's builder (S5a; S3b's SI-10): set while the
//! builder runs, removed when it returns either way, left when it panics.
use super::*;
use crate::mcp::daemon::test_support::fixture;
use crate::mcp::shared::identity::IdentityKey;
use crate::mcp::RefreshPolicy;
use std::path::{Path, PathBuf};

/// A key's files, owned, in a runtime directory of the test's own under `/tmp`.
fn owned_files() -> (tempfile::TempDir, DaemonFiles) {
    let base = tempfile::Builder::new()
        .prefix("s5a")
        .tempdir_in("/tmp")
        .expect("a temporary directory under /tmp");
    let key = IdentityKey::from_hex(&"ab".repeat(32)).expect("a key");
    match DaemonFiles::claim(&base.path().join("prism-mcpd"), &key).expect("claim") {
        Claim::Owned(files) => (base, *files),
        Claim::Taken => panic!("the key is taken"),
    }
}

fn marker_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// While the builder runs the marker holds this daemon's pid and a start time; once the builder
/// has returned an index the marker is gone. Rules out a marker that is never set, and one that
/// is set but not removed after a good build.
#[test]
fn the_marker_is_there_while_the_build_runs_and_gone_after_it_succeeds() {
    let (_base, files) = owned_files();
    let path: PathBuf = files.paths().build_marker().to_path_buf();
    let (_repo, cfg) = fixture(RefreshPolicy::WarnOnly);
    let seen = Arc::new(std::sync::Mutex::new(None));
    let builder: SessionBuilder = {
        let (path, seen) = (path.clone(), Arc::clone(&seen));
        Arc::new(move || {
            *seen.lock().expect("seen") = marker_text(&path);
            SessionProvider::bootstrap(&cfg)
        })
    };
    let built = marked(builder, files.build_marker().expect("a marker"))();
    assert!(built.is_ok());
    let during = seen
        .lock()
        .expect("seen")
        .clone()
        .expect("a marker during the build");
    let mut lines = during.lines();
    assert_eq!(lines.next(), Some(std::process::id().to_string().as_str()));
    let started: u64 = lines
        .next()
        .expect("a start time")
        .parse()
        .expect("seconds");
    assert!(started > 1_700_000_000, "{during}");
    assert!(marker_text(&path).is_none(), "removed after the build");
}

/// A build that fails is not a build that died: the marker is removed. Rules out removing it
/// only on success.
#[test]
fn the_marker_is_removed_when_the_build_fails() {
    let (_base, files) = owned_files();
    let path = files.paths().build_marker().to_path_buf();
    let builder: SessionBuilder = Arc::new(|| Err(anyhow::anyhow!("the build failed")));
    let built = marked(builder, files.build_marker().expect("a marker"))();
    assert!(built.is_err());
    assert!(marker_text(&path).is_none());
}

/// A builder that panics never returns, so the marker stays for the next shim (in production
/// the panic aborts the daemon, spec §3.1). Rules out removing the marker from a destructor,
/// which would run during the unwinding.
#[test]
fn a_builder_that_panics_leaves_the_marker() {
    let (_base, files) = owned_files();
    let path = files.paths().build_marker().to_path_buf();
    let builder: SessionBuilder = Arc::new(|| panic!("a builder panic, on purpose"));
    let wrapped = marked(builder, files.build_marker().expect("a marker"));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| wrapped()));
    assert!(outcome.is_err());
    let left = marker_text(&path).expect("the marker is left");
    assert!(
        left.starts_with(&format!("{}\n", std::process::id())),
        "{left}"
    );
}

/// A drain can end a daemon while its build runs; that is not a build that died, so releasing
/// the files removes the marker too. Rules out a release that leaves it, which would make the
/// next shim refuse to start a daemon after an ordinary SIGTERM.
#[test]
fn releasing_the_files_removes_the_marker() {
    let (_base, files) = owned_files();
    let path = files.paths().build_marker().to_path_buf();
    files.build_marker().expect("a marker").set();
    assert!(marker_text(&path).is_some());
    files.release();
    assert!(marker_text(&path).is_none());
}

/// Review round 1 (sol 2, Opus S3): a daemon that released the key (a drain during its build) can
/// neither clear nor set the marker afterwards, so it cannot remove the marker of the daemon that
/// took the key next, nor leave one behind for a build that did not die. Rules out a marker
/// handle that outlives its daemon's ownership.
#[test]
fn a_released_daemon_s_marker_cannot_touch_its_successor_s() {
    let (base, first) = owned_files();
    let dir = base.path().join("prism-mcpd");
    let key = IdentityKey::from_hex(&"ab".repeat(32)).expect("a key");
    let path = first.paths().build_marker().to_path_buf();
    let late = first.build_marker().expect("a marker");
    late.set();
    first.release();

    let second = match DaemonFiles::claim(&dir, &key).expect("claim") {
        Claim::Owned(files) => *files,
        Claim::Taken => panic!("the key is taken"),
    };
    // A late set from the released daemon writes nothing.
    late.set();
    assert!(
        marker_text(&path).is_none(),
        "a released daemon set the marker"
    );
    // The successor's own marker survives the released daemon's late clear.
    second.build_marker().expect("a marker").set();
    late.clear();
    assert!(
        marker_text(&path).is_some(),
        "a released daemon cleared its successor's marker"
    );
}

/// Review round 1 (sol 2): a marker change already in progress when the daemon releases the key
/// finishes first, and the release then removes what it wrote. The release cannot finish while
/// the change holds the fence; the 200 ms wait orders nothing a right implementation depends on,
/// it gives a wrong one (the flag read without holding its lock through the change) time to
/// finish, after which the change would leave a marker behind.
#[test]
fn a_release_waits_for_a_marker_change_in_progress() {
    use crate::mcp::daemon::test_support::HANG;
    use std::sync::mpsc;
    let (_base, files) = owned_files();
    let path = files.paths().build_marker().to_path_buf();
    let (paused_tx, paused_rx) = mpsc::channel::<()>();
    let (resume_tx, resume_rx) = mpsc::channel::<()>();
    let hooks = std::sync::Mutex::new((paused_tx, resume_rx));
    let marker = files
        .build_marker()
        .expect("a marker")
        .pausing(Arc::new(move || {
            let hooks = hooks.lock().expect("hooks");
            let _ = hooks.0.send(());
            let _ = hooks.1.recv_timeout(HANG);
        }));
    let setter = std::thread::spawn(move || marker.set());
    paused_rx
        .recv_timeout(HANG)
        .expect("the set is in progress");
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let releaser = std::thread::spawn(move || {
        files.release();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx
            .recv_timeout(std::time::Duration::from_millis(200))
            .is_err(),
        "the release finished while a marker change held the fence"
    );
    resume_tx.send(()).expect("resume the set");
    setter.join().expect("the set");
    releaser.join().expect("the release");
    assert!(
        marker_text(&path).is_none(),
        "a marker outlived the release"
    );
}
