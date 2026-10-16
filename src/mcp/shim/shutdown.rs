//! `prism-mcp daemon-shutdown --repo X` (spec §4.7, §11 item 3): ask the daemon of one
//! configuration to drain once it has no client.
//!
//! Graceful: it never disconnects a client, and until it drains the daemon goes on admitting new
//! ones (D100). Instance-bound: the request names the instance (identity, pid, start time) that a
//! status exchange on the same socket has just shown, or the one `--pid` and `--start-time` name,
//! and a daemon that is another instance records nothing. When the instance had no client, the
//! command waits, up to its timeout, until the daemon has removed its pid file, which it does
//! just before it releases the key.
//!
//! Exit 0 when the instance asked about no longer serves the key when the command ends (it
//! drained, or was already gone or draining), or has accepted the request and drains once its
//! clients have left; exit 1 when the command could not ask, or could not see the drain.
use super::arbitrate::POLL;
use super::connect::PeerUid;
use super::control::{exchange, Exchanged};
use super::diagnostic::{one_line, pid_file};
use super::key_files::KeyDir;
use super::status::{
    has_daemon_files, key_of, operator_runtime_dir, runtime_dir_exists, under_locks, Holder,
};
use crate::mcp::daemon::peer::peer_uid;
use crate::mcp::shared::control::{ControlRequest, Instance};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use crate::mcp::ServerConfig;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// What `prism-mcp daemon-shutdown` was started with.
#[derive(Clone, Debug)]
pub struct ShutdownOptions {
    /// `--repo`, `--cache-dir`, `--refresh-policy`: the configuration whose daemon (of this
    /// build) is asked.
    pub config: ServerConfig,
    /// `--runtime-dir`, as for `daemon-status`.
    pub runtime_base: Option<PathBuf>,
    /// `--pid` and `--start-time`: ask this instance only. `None` asks the one that answers.
    pub instance: Option<(u32, u64)>,
    /// `--timeout`: the whole command, the wait for the drain included.
    pub timeout: Duration,
}

/// Print one line, on standard output with exit status 0 or on standard error with 1.
pub fn run_daemon_shutdown(options: &ShutdownOptions) -> i32 {
    match shutdown(options, peer_uid) {
        Ok(message) => {
            println!("prism-mcp daemon-shutdown: {}", one_line(&message));
            0
        }
        Err(message) => {
            eprintln!("prism-mcp daemon-shutdown: {}", one_line(&message));
            1
        }
    }
}

fn named(instance: &Instance) -> String {
    format!("pid {} (started {})", instance.pid, instance.start_time)
}

/// The command; `Ok` is the exit-0 line and `Err` the exit-1 line.
pub(super) fn shutdown(options: &ShutdownOptions, peer_uid: PeerUid) -> Result<String, String> {
    let deadline = Instant::now() + options.timeout;
    let key = key_of(&options.config)?;
    let locator = key.locator();
    let nothing = || {
        format!(
            "no daemon serves this configuration of this build (key {locator}); nothing to shut down"
        )
    };
    let dir = operator_runtime_dir(options.runtime_base.as_deref())?;
    if !runtime_dir_exists(&dir)? {
        return Ok(nothing());
    }
    validate_runtime_dir(&dir).map_err(|error| error.to_string())?;
    let paths = DaemonPaths::new(&dir, &key).map_err(|error| error.to_string())?;
    let ask = |request: &ControlRequest| {
        exchange(paths.socket(), request, deadline, peer_uid, current_uid())
    };
    let found = match ask(&ControlRequest::Status) {
        Exchanged::Status(status) => status.instance,
        Exchanged::Retiring => {
            return Ok(format!("the daemon of key {locator} is already draining"))
        }
        Exchanged::NotListening(error) => {
            return not_listening(&paths, &locator, &error, nothing())
        }
        other => return Err(not_asked(&paths, &locator, other)),
    };
    if found.identity != key {
        return Err(format!(
            "the daemon at {} serves identity {}, not this configuration's {key}; it was not asked",
            paths.socket().display(),
            found.identity
        ));
    }
    if let Some((pid, start_time)) = options.instance {
        if (found.pid, found.start_time) != (pid, start_time) {
            return Ok(format!(
                "pid {pid} (started {start_time}) does not serve key {locator}; its daemon is {}, \
                 which was left alone",
                named(&found)
            ));
        }
    }
    let accepted = match ask(&ControlRequest::Shutdown(found)) {
        Exchanged::Status(status) => status,
        Exchanged::Retiring => return Ok(format!("{} is already draining", named(&found))),
        Exchanged::NotListening(error) => {
            return not_listening(
                &paths,
                &locator,
                &error,
                format!("{} is gone", named(&found)),
            )
        }
        // `BUSY` is answered before the line is read: nothing was recorded.
        busy @ Exchanged::Busy { .. } => return Err(not_asked(&paths, &locator, busy)),
        // The line was sent and no reply came back: the daemon may have recorded the request
        // before the reply was lost, and a request cannot be withdrawn (review round 1, Opus W4).
        Exchanged::NoReply(reason) => return Err(maybe_recorded(&found, &reason.to_string())),
        Exchanged::Refused(reason) => return Err(maybe_recorded(&found, &reason.to_string())),
    };
    if accepted.instance != found {
        return Ok(format!(
            "{} was replaced by {} before it was asked; the new daemon was left alone",
            named(&found),
            named(&accepted.instance)
        ));
    }
    if !accepted.shutdown {
        return Err(format!("{} did not record the request", named(&found)));
    }
    if accepted.admitted > 0 {
        return Ok(format!(
            "{} drains once its {} admitted clients have disconnected; it serves them, and any \
             new client, until then",
            named(&found),
            accepted.admitted
        ));
    }
    // Nothing is admitted: the daemon drains at its accept loop's next turn, and removes its pid
    // file just before it releases the key.
    let key_dir = KeyDir::open(&paths).map_err(|error| error.to_string())?;
    let written = format!("{}\n{}\n", found.pid, found.start_time);
    // Only a pid file that is gone, or that names another instance, is a drain: one that cannot
    // be read (any error but its absence) is looked at again, and reported at the timeout
    // (PR #418 review round 1, Opus S2).
    loop {
        let unreadable = match key_dir.pid_text() {
            Ok(Some(text)) if text.starts_with(&written) => None,
            Ok(_) => return Ok(format!("{} drained", named(&found))),
            Err(error) => Some(error),
        };
        if Instant::now() >= deadline {
            return Err(match unreadable {
                None => format!(
                    "{} accepted the request and still held key {locator} at the timeout; a \
                     client that arrived meanwhile keeps it up until it leaves (see daemon-status)",
                    named(&found)
                ),
                Some(error) => format!(
                    "{} accepted the request, and at the timeout its pid file could not be read \
                     ({error}), so its drain was not seen (see daemon-status)",
                    named(&found)
                ),
            });
        }
        std::thread::sleep(POLL);
    }
}

/// The reply to a sent `SHUTDOWN` was lost.
fn maybe_recorded(found: &Instance, why: &str) -> String {
    format!(
        "{} was sent the request and no reply came back ({why}); it may have been recorded, and a \
         request cannot be withdrawn: see daemon-status",
        named(found)
    )
}

/// Nothing accepted a connection at the key's socket. That is no daemon only when nobody holds
/// the key's owner lock: a live daemon whose listen backlog is full, or one that has claimed the
/// key and not bound yet, refuses connections too (review round 1, Opus W1). The locks are
/// looked at as `daemon-status` looks at them (D102), and only when a daemon's files are there.
fn not_listening(
    paths: &DaemonPaths,
    locator: &str,
    error: &std::io::Error,
    none: String,
) -> Result<String, String> {
    if !has_daemon_files(paths) {
        return Ok(none);
    }
    match under_locks(paths, |_| ())? {
        Holder::Free(()) => Ok(none),
        Holder::Alive => Err(format!(
            "the daemon of key {locator} is alive but not accepting connections (connect failed: \
             {error}); it was not asked; pid file {}",
            pid_file(paths, true)
        )),
        Holder::Starting => Err(format!(
            "the daemon of key {locator} is being started or reached (a shim holds its start \
             lock); it was not asked; try again"
        )),
    }
}

/// Why the daemon at the key's socket could not be asked.
fn not_asked(paths: &DaemonPaths, locator: &str, ended: Exchanged) -> String {
    let why = match ended {
        Exchanged::Busy { max_clients } => {
            format!("it is at its bound of {max_clients} clients; try again")
        }
        Exchanged::NoReply(reason) => format!("it does not answer ({reason})"),
        Exchanged::Refused(reason) => reason.to_string(),
        Exchanged::Status(_) | Exchanged::Retiring | Exchanged::NotListening(_) => {
            "it answered out of turn".to_string()
        }
    };
    format!(
        "the daemon of key {locator} was not shut down: {why}; pid file {}",
        pid_file(paths, true)
    )
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
