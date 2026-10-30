//! Tests for how the daemon is started (spec §4.1 step 6, §7, §11 item 7; S4's SI-16 and SI-17).
use super::super::shim_support::{key, runtime};
use super::*;
use std::ffi::OsStr;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;

fn args(warm: bool, idle_ttl: Option<u64>, max_clients: Option<u32>) -> DaemonArgs {
    DaemonArgs {
        repo_root: PathBuf::from("/repo"),
        cache_base: PathBuf::from("/cache"),
        refresh_policy: RefreshPolicy::AutoIncremental,
        key: key(),
        warm_at_startup: warm,
        idle_ttl,
        max_clients,
        idle_policy: crate::mcp::idle::IdlePolicy::default(),
    }
}

fn command_for(args: &DaemonArgs) -> (tempfile::TempDir, Command) {
    let (base, paths) = runtime();
    let log = File::create(base.path().join("log")).expect("a log");
    let command = daemon_command(Path::new("/bin/prism-mcp"), args, paths.runtime_dir(), log)
        .expect("a command");
    (base, command)
}

/// S4's interface for S5a: the daemon gets the shim's canonical root, canonical cache base,
/// refresh policy and full key, and `--warm-at-startup` only when the shim has it (spec §11 item
/// 7); `--idle-ttl` and `--max-clients` only when given.
#[test]
fn the_daemon_is_started_with_the_shim_s_configuration_and_key() {
    let hex = key().to_hex();
    let (_base, plain) = command_for(&args(false, None, None));
    assert_eq!(
        plain.get_args().collect::<Vec<_>>(),
        [
            "daemon",
            "--repo",
            "/repo",
            "--cache-dir",
            "/cache",
            "--refresh-policy",
            "auto-incremental",
            "--key",
            hex.as_str(),
            "--index-idle-secs",
            "0"
        ]
        .map(OsStr::new)
    );
    let (_base, full) = command_for(&args(true, Some(5), Some(8)));
    let tail: Vec<_> = full.get_args().skip(9).collect();
    assert_eq!(
        tail,
        [
            "--warm-at-startup",
            "--idle-ttl",
            "5",
            "--max-clients",
            "8",
            "--index-idle-secs",
            "0"
        ]
        .map(OsStr::new)
    );
}

/// Spec §7 and S4's SI-17: `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY` changes which call-edge sidecar a
/// daemon loads and is in neither the key nor the preamble, so it is removed from the daemon's
/// environment; the runtime directory is passed explicitly, and the daemon runs from `/`, so it
/// holds no directory of the shim's open.
#[test]
fn the_daemon_s_environment_drops_the_dirty_sidecar_override_and_names_the_runtime_dir() {
    let (base, paths) = runtime();
    let log = File::create(base.path().join("log")).expect("a log");
    let command = daemon_command(
        Path::new("/bin/prism-mcp"),
        &args(false, None, None),
        paths.runtime_dir(),
        log,
    )
    .expect("a command");
    let envs: Vec<_> = command.get_envs().collect();
    assert!(
        envs.contains(&(OsStr::new(LOAD_DIRTY_OVERRIDE), None)),
        "{envs:?}"
    );
    assert!(
        envs.contains(&(OsStr::new(RUNTIME_DIR_ENV), Some(base.path().as_os_str()))),
        "{envs:?}"
    );
    assert_eq!(command.get_current_dir(), Some(Path::new("/")));
}

/// S4's SI-16: the log the shim gives the daemon is opened for appending (the daemon that wins
/// the key truncates it when it is over 1 MiB; a descriptor without `O_APPEND` would overwrite
/// or leave a hole), created 0600, and an existing log is added to, not replaced.
#[test]
fn the_log_is_opened_for_appending() {
    let (_base, paths) = runtime();
    std::fs::write(paths.log_file(), "an earlier daemon's line\n").expect("an earlier log");
    let dir = KeyDir::open(&paths).expect("the runtime directory");
    let mut log = dir.open_log().expect("the log");
    // SAFETY: `F_GETFL` on an open descriptor reads its flags and touches no memory.
    let flags = unsafe { libc::fcntl(log.as_raw_fd(), libc::F_GETFL) };
    assert_ne!(flags & libc::O_APPEND, 0, "O_APPEND");
    log.write_all(b"this daemon's line\n").expect("write");
    assert_eq!(
        std::fs::read_to_string(paths.log_file()).expect("the log"),
        "an earlier daemon's line\nthis daemon's line\n"
    );
    std::fs::remove_file(paths.log_file()).expect("remove");
    drop(dir.open_log().expect("a new log"));
    let mode = std::fs::metadata(paths.log_file())
        .expect("log")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}
