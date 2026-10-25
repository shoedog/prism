//! `prism-mcp daemon-status` (spec §4.7): what every daemon in the runtime directory is doing, or
//! the one daemon of a configuration.
//!
//! It reads and never removes. A key with a socket is asked through the control exchange. A key
//! whose socket does not answer, or that has a pid file or a build marker and no socket, is
//! looked at through its locks: its start lock first, and its owner lock only while the start
//! lock is held, each for a moment, so that the look keeps out of a shim's start; at worst a daemon
//! about to claim the key finds the owner lock taken and exits, leaving no owner, never two
//! (D102). Keys with nothing but
//! lock files and a log, which every build of prism leaves behind (S4's D70), are counted in one
//! line, not listed and not removed (owner question 1).
use super::arbitrate::BUILD_CRASH_BACKOFF;
use super::connect::{PeerUid, Refusal, Transient};
use super::control::{exchange, Exchanged};
use super::diagnostic::{one_line, pid_file};
use super::key_files::{KeyDir, Marker};
use crate::mcp::daemon::files::MarkerContents;
use crate::mcp::daemon::peer::peer_uid;
use crate::mcp::session::canonical_config;
use crate::mcp::shared::control::{ControlRequest, DaemonStatus};
use crate::mcp::shared::identity::{DaemonIdentity, IdentityKey};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::runtime_dir::{
    current_uid, runtime_dir, runtime_dir_from, validate_runtime_dir,
};
use crate::mcp::ServerConfig;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs::TryLockError;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// `--timeout` default, seconds: how long one daemon has to answer.
pub const DEFAULT_CONTROL_TIMEOUT_SECS: u64 = 10;

/// The suffixes of a key's files (spec §4.3; the build marker, S5a).
const KEY_FILE_SUFFIXES: [&str; 6] = ["sock", "start", "owner", "pid", "log", "build"];

/// What `prism-mcp daemon-status` was started with.
#[derive(Clone, Debug)]
pub struct StatusOptions {
    /// `--repo` (with `--cache-dir` and `--refresh-policy`): show this configuration's daemon
    /// only. `None` shows every key in the runtime directory.
    pub config: Option<ServerConfig>,
    /// `--runtime-dir`: the base the runtime directory is in, as `PRISM_MCP_RUNTIME_DIR` gives it
    /// (D103). `None` resolves it as a shim does.
    pub runtime_base: Option<PathBuf>,
    /// `--timeout`: how long each daemon has to answer.
    pub timeout: Duration,
}

/// Print the report on standard output and return 0; or, when a key could not be looked at,
/// print the report, then one line on standard error, and return 1 (PR #418 review round 2,
/// sol 1); or, with no report, one line on standard error and 1.
pub fn run_daemon_status(options: &StatusOptions) -> i32 {
    match status_lines(options, peer_uid) {
        Ok((report, 0)) => {
            print!("{report}");
            0
        }
        Ok((report, unlooked)) => {
            print!("{report}");
            eprintln!("prism-mcp daemon-status: {unlooked} keys could not be looked at");
            1
        }
        Err(error) => {
            eprintln!("prism-mcp daemon-status: {}", one_line(&error));
            1
        }
    }
}

/// The runtime directory for `base` (`--runtime-dir`), or as a shim resolves it.
pub(super) fn operator_runtime_dir(base: Option<&Path>) -> Result<PathBuf, String> {
    match base {
        Some(base) => runtime_dir_from(
            Some(base.as_os_str().to_owned()),
            dirs::runtime_dir(),
            std::env::temp_dir(),
        ),
        None => runtime_dir(),
    }
    .map_err(|error| error.to_string())
}

/// The identity key of `cfg` for this build, as its shim computes it.
pub(super) fn key_of(cfg: &ServerConfig) -> Result<IdentityKey, String> {
    let cfg = canonical_config(cfg).map_err(|error| error.to_string())?;
    let identity = DaemonIdentity::resolve(&cfg).map_err(|error| error.to_string())?;
    Ok(identity.key())
}

/// Seconds since the Unix epoch, as the daemon and the build marker write them.
pub(super) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The report, or why there is none: [`status_lines`] without its count, as the tests read it.
#[cfg(test)]
pub(super) fn status_report(options: &StatusOptions, peer_uid: PeerUid) -> Result<String, String> {
    status_lines(options, peer_uid).map(|(report, _)| report)
}

/// The report and how many keys in it could not be looked at, or why there is none.
fn status_lines(options: &StatusOptions, peer_uid: PeerUid) -> Result<(String, u64), String> {
    let dir = operator_runtime_dir(options.runtime_base.as_deref())?;
    let wanted = options.config.as_ref().map(key_of).transpose()?;
    let mut report = String::new();
    let _ = writeln!(
        report,
        "{}",
        one_line(&format!("runtime dir {}", dir.display()))
    );
    if !runtime_dir_exists(&dir)? {
        report.push_str("no daemon (the runtime directory does not exist)\n");
        return Ok((report, 0));
    }
    validate_runtime_dir(&dir).map_err(|error| error.to_string())?;
    let locators = match wanted {
        Some(key) => BTreeSet::from([key.locator()]),
        None => locators_in(&dir)?,
    };
    let (mut quiet, mut log_bytes, mut unlooked) = (0u64, 0u64, 0u64);
    for locator in &locators {
        let paths = DaemonPaths::from_locator(&dir, locator).map_err(|error| error.to_string())?;
        match look(&paths, locator, wanted, options.timeout, peer_uid) {
            Ok(Some(text)) => report.push_str(&text),
            Ok(None) if wanted.is_some() => {
                let _ = writeln!(report, "{locator} no daemon for this configuration");
            }
            Ok(None) => {
                quiet += 1;
                log_bytes += std::fs::symlink_metadata(paths.log_file()).map_or(0, |m| m.len());
            }
            // One key that cannot be looked at does not hide the others (review round 1, Opus S7).
            Err(error) => {
                unlooked += 1;
                let _ = writeln!(
                    report,
                    "{}",
                    one_line(&format!("{locator} cannot be looked at: {error}"))
                );
            }
        }
    }
    if locators.is_empty() {
        report.push_str("no daemon\n");
    }
    if quiet > 0 {
        let _ = writeln!(
            report,
            "{quiet} keys have nothing but lock files and logs ({log_bytes} bytes of logs)"
        );
    }
    Ok((report, unlooked))
}

/// Whether the runtime directory `dir` is there (see [`present`]).
pub(super) fn runtime_dir_exists(dir: &Path) -> Result<bool, String> {
    present(dir)
}

/// Whether anything is at `path`, not following a symlink. Only its absence (`NotFound`, or
/// `NotADirectory`: a component of the path is not a directory, so nothing can be at it; review
/// round 2, Opus S2) is "not there": any other error, such as a parent this user cannot search,
/// says nothing about what is there and is returned. Every look at the runtime directory and at a key's files goes
/// through it, so that no lookup error is taken for a missing daemon (PR #418 review round 1,
/// sol 1 and Opus S2, for the directory; round 2, sol 1 and Opus S1, for a key's files).
pub(super) fn present(path: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(format!("cannot look at {}: {error}", path.display())),
    }
}

/// The locators of every key that has a file in `dir`: names of 16 lowercase hexadecimal digits
/// and one of the key files' suffixes. Anything else in the directory is not prism's and is
/// ignored.
fn locators_in(dir: &Path) -> Result<BTreeSet<String>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|error| format!("cannot list {}: {error}", dir.display()))?;
    let mut locators = BTreeSet::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some((stem, suffix)) = name.to_str().and_then(|name| name.split_once('.')) else {
            continue;
        };
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if stem.len() == 16 && stem.chars().all(hex) && KEY_FILE_SUFFIXES.contains(&suffix) {
            locators.insert(stem.to_string());
        }
    }
    Ok(locators)
}

/// What one key's lines say, or `None` for a key with no socket, no pid file and no build marker
/// (nothing of a daemon's but lock files and a log).
fn look(
    paths: &DaemonPaths,
    locator: &str,
    wanted: Option<IdentityKey>,
    timeout: Duration,
    peer_uid: PeerUid,
) -> Result<Option<String>, String> {
    let mut no_answer = None;
    let mut incompatible = false;
    if present(paths.socket())? {
        let deadline = Instant::now() + timeout;
        match exchange(
            paths.socket(),
            &ControlRequest::Status,
            deadline,
            peer_uid,
            current_uid(),
        ) {
            Exchanged::Status(status) => return Ok(Some(describe(locator, &status, wanted))),
            Exchanged::Retiring => {
                return Ok(Some(format!("{locator} draining: it answered RETIRING\n")))
            }
            Exchanged::Busy { max_clients } => {
                return Ok(Some(format!(
                    "{locator} busy: it answered BUSY at its bound of {max_clients} clients, so \
                     no status\n"
                )))
            }
            Exchanged::NotListening(error) => no_answer = Some(format!("connect failed: {error}")),
            Exchanged::NoReply(reason) => {
                incompatible = matches!(reason, Transient::Closed);
                no_answer = Some(reason.to_string());
            }
            Exchanged::Refused(reason) => {
                incompatible = matches!(reason, Refusal::BadReply(_));
                no_answer = Some(reason.to_string());
            }
        }
    } else if !has_daemon_files(paths)? {
        return Ok(None);
    }
    let line = match under_locks(paths, |dir| dead(paths, locator, dir))? {
        Holder::Starting => format!(
            "{locator} being started or reached: a shim holds its start lock; pid file {}",
            pid_file(paths, true)
        ),
        Holder::Alive => format!(
            "{locator} alive but not answering ({}); pid file {}",
            no_answer.unwrap_or_else(|| "it has no socket: starting or draining".to_string()),
            pid_file(paths, true)
        ),
        Holder::Free(line) => line?,
    };
    let mut text = one_line(&line) + "\n";
    if incompatible {
        text.push_str("  status unavailable: incompatible or unreadable control reply; use the daemon's matching binary for status/shutdown\n");
    }
    Ok(Some(text))
}

/// Whether anything of a daemon's is at a key's paths: a socket, a pid file or a build marker.
/// A key with none of them has nothing but lock files and a log, and is not looked at. A path
/// that cannot be looked at is an error, never "none of them" ([`present`]).
pub(super) fn has_daemon_files(paths: &DaemonPaths) -> Result<bool, String> {
    Ok(present(paths.socket())? || present(paths.pid_file())? || present(paths.build_marker())?)
}

/// Who holds a key, as `daemon-status` and `daemon-shutdown` look (D102).
pub(super) enum Holder<T> {
    /// A shim holds the start lock: it is starting or reaching the daemon.
    Starting,
    /// A daemon holds the owner lock.
    Alive,
    /// Nobody holds either; what `free` found, run while this process held both.
    Free(T),
}

/// Look at a key's locks without waiting: the start lock first and, holding it, the owner lock,
/// each for a moment; `free` runs while both are held. Neither lock file is ever removed.
pub(super) fn under_locks<T>(
    paths: &DaemonPaths,
    free: impl FnOnce(&KeyDir<'_>) -> T,
) -> Result<Holder<T>, String> {
    let dir = KeyDir::open(paths).map_err(|error| error.to_string())?;
    let start = dir.start_lock().map_err(|error| error.to_string())?;
    match start.try_lock() {
        Err(TryLockError::WouldBlock) => Ok(Holder::Starting),
        Err(TryLockError::Error(error)) => Err(format!(
            "cannot lock {}: {error}",
            paths.start_lock().display()
        )),
        Ok(()) => match dir.try_own().map_err(|error| error.to_string())? {
            None => Ok(Holder::Alive),
            Some(_owner) => Ok(Holder::Free(free(&dir))),
        },
    }
}

/// A key no daemon owns: what a dead daemon left, and the restart bound if it died during its
/// build. Nothing is removed (the next shim of the key removes the socket and the pid file, D77).
fn dead(paths: &DaemonPaths, locator: &str, dir: &KeyDir<'_>) -> Result<String, String> {
    let mut left = Vec::new();
    if present(paths.socket())? {
        left.push("its socket".to_string());
    }
    if present(paths.pid_file())? {
        left.push(format!("its pid file ({})", pid_file(paths, true)));
    }
    let mut line = format!("{locator} no daemon");
    if !left.is_empty() {
        let _ = write!(line, "; a dead daemon left {}", left.join(" and "));
    }
    match dir.read_build_marker() {
        Marker::Absent => {}
        Marker::Read(marker) => {
            let _ = write!(line, "; {}", died_building(&marker, unix_now()));
        }
        Marker::Unusable(why) => {
            let _ = write!(
                line,
                "; its build marker is not usable ({why}); the next shim removes it"
            );
        }
    }
    Ok(line)
}

/// What a build marker of a dead daemon says (S5a's D77): who died during its build, and when a
/// shim may start the key's daemon again. The rule is `arbitrate`'s: 60 s from the first sighting,
/// and a sighting dated in the future does not count.
pub(super) fn died_building(marker: &MarkerContents, now: u64) -> String {
    let known = |field: Option<u64>| field.map_or_else(|| "unknown".to_string(), |v| v.to_string());
    let backoff = BUILD_CRASH_BACKOFF.as_secs();
    let when = match marker.seen {
        None => format!(
            "no shim has seen it yet: the next one records the sighting and waits {backoff} s"
        ),
        Some(seen) => match now.checked_sub(seen) {
            Some(age) if age < backoff => format!(
                "first seen by a shim at {seen}; a shim restarts it in {} s",
                backoff - age
            ),
            _ => format!("first seen by a shim at {seen}; the next shim restarts it"),
        },
    };
    format!(
        "its last daemon (pid {}) died during its index build (started {}); {when}",
        known(marker.pid.map(u64::from)),
        known(marker.started)
    )
}

/// The lines of a daemon that answered.
fn describe(locator: &str, status: &DaemonStatus, wanted: Option<IdentityKey>) -> String {
    let instance = &status.instance;
    let connection = if status.admitted == 0 && status.pending == 0 {
        "clientless"
    } else {
        "connected"
    };
    let idle = match status.idle_secs {
        Some(secs) => format!(
            "idle {secs} s; retires after {} s idle",
            status.idle_ttl_secs
        ),
        None => "in use".to_string(),
    };
    let mut lines = vec![
        format!(
            "{locator} pid {}, started {}, serving {}",
            instance.pid, instance.start_time, status.repo_root
        ),
        format!(
            "  key {}; build {}",
            instance.identity, status.build_identity
        ),
        format!(
            "  clients: {} admitted, {} pending, at most {}; {connection}; {idle}",
            status.admitted, status.pending, status.max_clients
        ),
        format!(
            "  index: {} (attempt {}, generation {})",
            status.build.word(),
            status.attempt,
            status.generation
        ),
        format!(
            "  effective index idle policy: index_idle_timeout_secs={}; index_idle_pressure_relief={}",
            status.index_idle_timeout_secs, status.index_idle_pressure_relief
        ),
    ];
    if status.shutdown {
        lines.push("  shutdown requested: it drains once no client is left".to_string());
    }
    if let Some(key) = wanted.filter(|key| *key != instance.identity) {
        lines.push(format!(
            "  this is not the daemon of this configuration ({key}): another key with the same \
             first 16 digits"
        ));
    }
    lines.iter().map(|line| one_line(line) + "\n").collect()
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
