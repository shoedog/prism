//! Tests for the daemon's files (spec §3.1 "Starting", §4.3 "Files", §4.6; the daemon's half of
//! spec §9 tests 3 and 4). Every test uses a runtime directory of its own under `/tmp`, short
//! enough for a socket path (S2's note for S4).
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;

/// A runtime directory of this test's own, and a key for it.
fn runtime() -> (tempfile::TempDir, PathBuf, IdentityKey) {
    let base = tempfile::Builder::new()
        .prefix("s4")
        .tempdir_in("/tmp")
        .expect("a temporary directory under /tmp");
    let dir = base.path().join("prism-mcpd");
    let key = IdentityKey::from_hex(&"ab".repeat(32)).expect("a key");
    (base, dir, key)
}

fn owned(dir: &Path, key: &IdentityKey) -> DaemonFiles {
    match DaemonFiles::claim(dir, key).expect("the files can be claimed") {
        Claim::Owned(files) => *files,
        Claim::Taken => panic!("the key was taken"),
    }
}

fn taken(dir: &Path, key: &IdentityKey) -> bool {
    matches!(
        DaemonFiles::claim(dir, key).expect("the files can be claimed"),
        Claim::Taken
    )
}

fn inode(path: &Path) -> u64 {
    std::fs::symlink_metadata(path).expect("lstat").ino()
}

/// Spec §3.1: the owner lock decides who owns a key. A second claim, from another descriptor
/// (as a second daemon process would make), is told the key is taken, and once the owner is
/// gone the key can be claimed again. Rules out a claim that does not lock, and a lock that
/// outlives its holder.
#[test]
fn the_first_claim_owns_the_key_and_a_second_is_told_it_is_taken() {
    let (_base, dir, key) = runtime();
    let first = owned(&dir, &key);
    assert!(taken(&dir, &key));
    assert!(taken(&dir, &key), "and again");
    first.release();
    let again = owned(&dir, &key);
    drop(again);
    let _third = owned(&dir, &key);
}

/// Spec §4.1 step 6 and §9 test 4, the daemon's half: a daemon that finds the owner lock held
/// leaves the live owner's socket and pid file alone, and the owner goes on accepting. Rules out
/// a starting daemon that clears the socket before it knows it owns the key.
#[test]
fn a_daemon_that_finds_the_key_taken_touches_no_file() {
    let (_base, dir, key) = runtime();
    let mut live = owned(&dir, &key);
    let listener = live.bind().expect("bind");
    live.write_pid(4242, 17, "build").expect("pid");
    let socket = live.paths().socket().to_path_buf();
    let pid_file = live.paths().pid_file().to_path_buf();
    let (socket_inode, pid_before) = (
        inode(&socket),
        std::fs::read_to_string(&pid_file).expect("pid"),
    );

    assert!(taken(&dir, &key));

    assert_eq!(inode(&socket), socket_inode);
    assert_eq!(std::fs::read_to_string(&pid_file).expect("pid"), pid_before);
    let _client = UnixStream::connect(&socket).expect("the live socket still answers");
    listener.accept().expect("the live owner accepts");
}

/// Spec §4.6 "Daemon crashes" and §9 test 3, the daemon's half: a socket left by a daemon that
/// died is replaced by the next owner, which then accepts on it. Rules out a bind that fails on
/// the leftover file.
#[test]
fn binding_replaces_a_stale_socket_left_by_a_dead_daemon() {
    let (_base, dir, key) = runtime();
    let socket = {
        let files = owned(&dir, &key);
        let socket = files.paths().socket().to_path_buf();
        drop(UnixListener::bind(&socket).expect("a socket nobody listens on"));
        drop(files);
        socket
    };
    // Only the file is checked: on macOS a child another test spawns while the listener is being
    // made close-on-exec can inherit it and keep it accepting.
    assert!(is_socket(&socket), "the stale socket is there");

    let mut files = owned(&dir, &key);
    let listener = files.bind().expect("bind over the stale socket");
    let _client = UnixStream::connect(&socket).expect("the new socket answers");
    listener.accept().expect("the new owner accepts");
}

/// Spec §4.3: lock files are created 0600 and never removed, not even by the owner when it
/// stops.
#[test]
fn the_lock_file_is_0600_and_stays_after_release() {
    let (_base, dir, key) = runtime();
    let files = owned(&dir, &key);
    let owner_lock = files.paths().owner_lock().to_path_buf();
    let mode = std::fs::metadata(&owner_lock)
        .expect("the lock file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let before = inode(&owner_lock);
    files.release();
    assert_eq!(inode(&owner_lock), before, "the same file, still there");
}

/// Spec §3.1: the pid file holds `pid\nstart_time\nbuild_identity\n` while the daemon lives and
/// is removed by the owner when it stops; the owner lock is free afterwards.
#[test]
fn the_pid_file_says_who_owns_the_key_and_is_removed_at_release() {
    let (_base, dir, key) = runtime();
    let mut files = owned(&dir, &key);
    files
        .write_pid(4242, 1_700_000_000, "0123abcd")
        .expect("pid");
    let pid_file = files.paths().pid_file().to_path_buf();
    assert_eq!(
        std::fs::read_to_string(&pid_file).expect("pid"),
        "4242\n1700000000\n0123abcd\n"
    );
    let mode = std::fs::metadata(&pid_file)
        .expect("pid")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);

    files.release();
    assert!(!pid_file.exists());
    let _again = owned(&dir, &key);
}

/// A symlink where a lock file should be is refused, whether it dangles or names a file this
/// user owns, and what it points at is neither created nor locked. Rules out an `open` that
/// follows it, which would let whoever placed it choose the file the daemon locks and writes.
#[test]
fn a_symlink_in_place_of_the_owner_lock_is_refused() {
    let (base, dir, key) = runtime();
    validate_runtime_dir(&dir).expect("create the runtime directory");
    let paths = DaemonPaths::new(&dir, &key).expect("paths");

    let dangling = base.path().join("dangling");
    std::os::unix::fs::symlink(&dangling, paths.owner_lock()).expect("symlink");
    let refused = DaemonFiles::claim(&dir, &key);
    assert!(
        matches!(refused, Err(FilesError::Io { op: "open", .. })),
        "{:?}",
        refused.err()
    );
    assert!(!dangling.exists());

    let existing = base.path().join("existing");
    std::fs::write(&existing, "untouched").expect("a file of this user");
    std::fs::remove_file(paths.owner_lock()).expect("remove the symlink");
    std::os::unix::fs::symlink(&existing, paths.owner_lock()).expect("symlink");
    let refused = DaemonFiles::claim(&dir, &key);
    assert!(
        matches!(refused, Err(FilesError::Io { op: "open", .. })),
        "{:?}",
        refused.err()
    );
    let target = File::open(&existing).expect("the target");
    target.try_lock().expect("the target is not locked");
    assert_eq!(
        std::fs::read_to_string(&existing).expect("read"),
        "untouched"
    );
}

/// A runtime directory that fails validation is refused before any file is created in it.
#[test]
fn an_invalid_runtime_directory_is_refused_before_anything_is_created() {
    let (_base, dir, key) = runtime();
    std::fs::create_dir(&dir).expect("mkdir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let refused = DaemonFiles::claim(&dir, &key);
    assert!(
        matches!(refused, Err(FilesError::RuntimeDir(_))),
        "{:?}",
        refused.err()
    );
    assert_eq!(std::fs::read_dir(&dir).expect("list").count(), 0);
}

/// The socket is removed only while the entry of its name is the one this daemon bound. Rules
/// out removing a socket that someone else put there since.
#[test]
fn the_socket_is_removed_only_if_it_is_still_the_one_this_daemon_bound() {
    let (_base, dir, key) = runtime();
    let mut files = owned(&dir, &key);
    let _listener = files.bind().expect("bind");
    let socket = files.paths().socket().to_path_buf();
    files.unlink_socket();
    assert!(!socket.exists(), "its own socket is removed");

    let _listener = files.bind().expect("bind again");
    std::fs::remove_file(&socket).expect("remove");
    let _other = UnixListener::bind(&socket).expect("another socket at the path");
    let other_inode = inode(&socket);
    files.unlink_socket();
    assert_eq!(inode(&socket), other_inode, "another socket is left alone");
}

/// Lane README, notes for S4: when the runtime directory is replaced under a live daemon, a
/// second daemon could take the key in the new directory. The daemon notices that its files are
/// no longer the ones at their paths, and what it removes as it stops is removed from its own
/// directory, not from the new one.
#[test]
fn a_replaced_runtime_directory_is_noticed_and_left_alone() {
    let (base, dir, key) = runtime();
    let mut files = owned(&dir, &key);
    let _listener = files.bind().expect("bind");
    files.write_pid(1, 2, "build").expect("pid");
    assert!(files.still_ours());

    let moved = base.path().join("moved");
    std::fs::rename(&dir, &moved).expect("move the runtime directory away");
    assert!(!files.still_ours(), "the directory is gone");
    let mut newcomer = owned(&dir, &key);
    let _new_listener = newcomer.bind().expect("bind");
    newcomer.write_pid(3, 4, "build").expect("pid");
    assert!(
        !files.still_ours(),
        "another daemon's files are at the paths"
    );

    files.unlink_socket();
    files.release();
    let paths = newcomer.paths().clone();
    assert!(is_socket(paths.socket()), "the newcomer's socket stays");
    assert_eq!(
        std::fs::read_to_string(paths.pid_file()).expect("pid"),
        "3\n4\nbuild\n",
        "the newcomer's pid file stays"
    );
    let old = DaemonPaths::new(&moved, &key).expect("paths");
    assert!(!old.socket().exists(), "its own socket is removed");
    assert!(!old.pid_file().exists(), "its own pid file is removed");
}

/// The socket half of `still_ours` (review of S4-1, Opus F1): with the owner lock and the pid
/// file untouched, a socket that was removed or replaced since the bind means the daemon's name
/// is no longer reachable, so its files are no longer its own. Rules out a check that looks
/// only at the owner lock.
#[test]
fn a_replaced_socket_alone_is_noticed_and_left_alone() {
    let (_base, dir, key) = runtime();
    let mut files = owned(&dir, &key);
    let _listener = files.bind().expect("bind");
    files.write_pid(1, 2, "build").expect("pid");
    let paths = files.paths().clone();
    assert!(files.still_ours());

    std::fs::remove_file(paths.socket()).expect("remove the socket");
    assert!(!files.still_ours(), "the socket is gone");
    let _other = UnixListener::bind(paths.socket()).expect("another socket at the path");
    let other_inode = inode(paths.socket());
    assert!(
        !files.still_ours(),
        "another socket is at the path, though the owner lock and pid file are unchanged"
    );

    files.unlink_socket();
    assert_eq!(
        inode(paths.socket()),
        other_inode,
        "the other socket is not removed"
    );
    files.release();
    assert!(!paths.pid_file().exists(), "its own pid file is removed");
}
