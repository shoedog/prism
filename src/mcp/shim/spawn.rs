//! Starting the daemon process (spec §4.1 step 6): `prism-mcp daemon …` from this same
//! executable, in a session of its own (`setsid`), with standard input null and standard output
//! and error appended to the key's log, from the root directory, with the shim's environment
//! minus what would change its answers (spec §7). The lock descriptors are close-on-exec, so the
//! child inherits neither lock; it takes the owner lock itself.
use super::arbitrate::{Exit, Started, Starter};
use super::key_files::KeyDir;
use crate::mcp::shared::identity::{refresh_policy_name, IdentityKey};
use crate::mcp::shared::runtime_dir::RUNTIME_DIR_ENV;
use crate::mcp::RefreshPolicy;
use crate::navigation::call_edge_cache::LOAD_DIRTY_OVERRIDE;
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// What the daemon is started with: the shim's resolved configuration and key (S4's interface
/// for S5a). The daemon resolves its own key from these and refuses to start on a difference.
#[derive(Clone, Debug)]
pub(super) struct DaemonArgs {
    /// The canonical repository root.
    pub(super) repo_root: PathBuf,
    /// The canonical cache base (`DaemonIdentity::cache_base`).
    pub(super) cache_base: PathBuf,
    pub(super) refresh_policy: RefreshPolicy,
    pub(super) key: IdentityKey,
    /// `--warm-at-startup` on the daemon's command line (spec §11 item 7).
    pub(super) warm_at_startup: bool,
    /// `--idle-ttl` and `--max-clients` for a daemon this shim starts; a running daemon keeps
    /// its own.
    pub(super) idle_ttl: Option<u64>,
    pub(super) max_clients: Option<u32>,
    pub(super) idle_policy: crate::mcp::idle::IdlePolicy,
}

impl DaemonArgs {
    /// The daemon's arguments, `daemon` first.
    pub(super) fn to_args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec![
            "daemon".into(),
            "--repo".into(),
            self.repo_root.clone().into(),
            "--cache-dir".into(),
            self.cache_base.clone().into(),
            "--refresh-policy".into(),
            refresh_policy_name(self.refresh_policy).into(),
            "--key".into(),
            self.key.to_hex().into(),
        ];
        if self.warm_at_startup {
            args.push("--warm-at-startup".into());
        }
        if let Some(secs) = self.idle_ttl {
            args.extend(["--idle-ttl".into(), secs.to_string().into()]);
        }
        if let Some(max) = self.max_clients {
            args.extend(["--max-clients".into(), max.to_string().into()]);
        }
        // Always explicit, including off: the child must not decide policy from inherited env.
        args.extend([
            "--index-idle-secs".into(),
            self.idle_policy
                .timeout
                .unwrap_or_default()
                .as_secs()
                .to_string()
                .into(),
        ]);
        if self.idle_policy.pressure_relief {
            args.push("--index-idle-pressure-relief".into());
        }
        args
    }
}

/// The command that starts the daemon. `runtime_dir` is the directory the shim validated; the
/// daemon is pointed at the same one through `PRISM_MCP_RUNTIME_DIR` (its parent, the base).
pub(super) fn daemon_command(
    exe: &Path,
    args: &DaemonArgs,
    runtime_dir: &Path,
    log: File,
) -> io::Result<Command> {
    let mut command = Command::new(exe);
    command
        .args(args.to_args())
        // A daemon serves every client of its key; what changes its answers is in the key or
        // the preamble, never inherited from the shim that happened to start it (spec §7;
        // S4's SI-17).
        .env_remove(LOAD_DIRTY_OVERRIDE)
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    if let Some(base) = runtime_dir.parent() {
        command.env(RUNTIME_DIR_ENV, base);
    }
    // SAFETY: the closure runs in the child between `fork` and `exec` and calls only `setsid`,
    // which is async-signal-safe, and reads `errno`.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(command)
}

/// Starts daemon processes from this executable.
pub(super) struct ProcessStarter {
    pub(super) exe: PathBuf,
    pub(super) args: DaemonArgs,
    pub(super) runtime_dir: PathBuf,
}

impl Starter for ProcessStarter {
    fn start(&mut self, dir: &KeyDir<'_>) -> io::Result<Box<dyn Started>> {
        let log = dir
            .open_log()
            .map_err(|error| io::Error::other(error.to_string()))?;
        let child = daemon_command(&self.exe, &self.args, &self.runtime_dir, log)?.spawn()?;
        Ok(Box::new(DaemonChild(child)))
    }
}

/// A daemon process this shim spawned.
struct DaemonChild(Child);

impl Started for DaemonChild {
    fn pid(&self) -> u32 {
        self.0.id()
    }

    fn exit(&mut self) -> Option<Exit> {
        match self.0.try_wait() {
            Ok(None) => None,
            Ok(Some(status)) if status.success() => Some(Exit::Success),
            Ok(Some(status)) => Some(Exit::Failure(status.to_string())),
            Err(error) => Some(Exit::Failure(format!("cannot wait for it: {error}"))),
        }
    }

    fn reap(mut self: Box<Self>) {
        let _ = self.0.wait();
    }
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
