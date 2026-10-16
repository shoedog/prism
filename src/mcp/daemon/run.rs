//! The daemon process, `prism-mcp daemon ...` (spec §3.1 "Starting", §4.2, §8 step 4): resolve
//! the identity, own the key or leave silently, bind, write the pid file, serve, drain.
use super::coordinator::DaemonState;
use super::files::{BuildMarker, Claim, DaemonFiles};
use super::log;
use super::server::{ServeOptions, Served, Server, Waker, HANDSHAKE_TIMEOUT};
use crate::mcp::lazy::SessionBuilder;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::session::canonical_config;
use crate::mcp::shared::identity::{DaemonIdentity, IdentityKey};
use crate::mcp::shared::runtime_dir::runtime_dir;
use crate::mcp::{ServerConfig, SessionProvider};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// `--idle-ttl` default, seconds (spec §8 step 4, §11 item 1).
pub const DEFAULT_IDLE_TTL_SECS: u64 = 900;
/// `--max-clients` default (spec §4.2, §8 step 4).
pub const DEFAULT_MAX_CLIENTS: u32 = 64;

/// What `prism-mcp daemon` was started with.
#[derive(Clone, Debug)]
pub struct DaemonOptions {
    /// The repository, the cache base, the refresh policy, and `startup`: `Lazy`, or
    /// `Background` for `--warm-at-startup` on the daemon's own command line (spec §11 item 7).
    /// `first_call_wait` is not read: each client sends its own.
    pub config: ServerConfig,
    pub idle_ttl: Duration,
    pub max_clients: u32,
    /// The full key the starting shim computed (`--key`). The daemon refuses to start when the
    /// key it computes differs, rather than adopt it (lane README, notes for S5a).
    pub expected_key: Option<String>,
}

/// Run a daemon for `options` until it drains. Returns `Ok` when it drained, and also at once,
/// silently, when another live daemon owns the key (spec §3.1: "exit 0 silently").
pub fn run_daemon(options: DaemonOptions) -> anyhow::Result<()> {
    let cfg = canonical_config(&options.config)?;
    let identity = DaemonIdentity::resolve(&cfg)?;
    let key = identity.key();
    if let Some(expected) = &options.expected_key {
        let expected = IdentityKey::from_hex(expected)
            .map_err(|error| anyhow::anyhow!("--key is not an identity key: {error}"))?;
        anyhow::ensure!(
            expected == key,
            "--key {expected} is not the identity this daemon computes for its configuration \
             ({key}); the repository root or the cache base changed between the two resolutions"
        );
    }
    let dir = runtime_dir()?;
    let mut files = match DaemonFiles::claim(&dir, &key)? {
        Claim::Owned(files) => *files,
        Claim::Taken => return Ok(()),
    };
    // Only the owner may truncate the key's log: a daemon that lost the claim shares it with
    // the owner (review round 1).
    log::start();
    let listener = files.bind()?;
    let pid = std::process::id();
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    files.write_pid(pid, start_time, &identity.build().cache_build_identity)?;

    let builder: SessionBuilder = {
        let cfg = cfg.clone();
        marked(
            Arc::new(move || SessionProvider::bootstrap(&cfg)),
            files.build_marker()?,
        )
    };
    let state = DaemonState::new(builder, cfg.startup)?;
    let served = Served {
        key,
        repo_root: identity.repo_root().to_path_buf(),
        root_fs: identity.root_fs(),
        pid,
        start_time,
        build_identity: identity.build().cache_build_identity.clone(),
    };
    let options = ServeOptions {
        max_clients: options.max_clients,
        idle_ttl: options.idle_ttl,
        handshake_timeout: HANDSHAKE_TIMEOUT,
    };
    let socket = files.paths().socket().to_path_buf();
    let mut server = Server::new(
        state,
        Arc::new(ToolRegistry::all_v1()),
        files,
        listener,
        served,
        options,
    )?;
    install_signal_handlers(&server.waker())?;
    server.set_signalled(signalled);
    log::line(&format!(
        "pid {pid} serves {} for {} at {}",
        key.locator(),
        identity.repo_root().display(),
        socket.display()
    ));
    let reason = server.run()?;
    log::line(&format!("pid {pid} stopped: {}", reason.describe()));
    Ok(())
}

/// `builder` with the build marker set while it runs and removed when it returns, with an index
/// or an error. A builder that panics does not return: the abort that follows (spec §3.1) leaves
/// the marker for the next shim to find (S5a, S3b's SI-10).
fn marked(builder: SessionBuilder, marker: BuildMarker) -> SessionBuilder {
    Arc::new(move || {
        marker.set();
        let built = builder();
        marker.clear();
        built
    })
}

/// The descriptor the signal handler writes to; -1 until one is installed.
static SIGNAL_WAKE_FD: AtomicI32 = AtomicI32::new(-1);
static SIGNALLED: AtomicBool = AtomicBool::new(false);
static WAKE_BYTE: u8 = 1;

/// SIGTERM and SIGINT: only async-signal-safe work, a flag and one `write`.
extern "C" fn on_termination_signal(_signal: libc::c_int) {
    SIGNALLED.store(true, Ordering::SeqCst);
    let fd = SIGNAL_WAKE_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        // SAFETY: `write` is async-signal-safe; `fd` is a descriptor that is never closed and
        // `WAKE_BYTE` a static byte.
        unsafe {
            libc::write(fd, std::ptr::addr_of!(WAKE_BYTE).cast(), 1);
        }
    }
}

fn signalled() -> bool {
    SIGNALLED.load(Ordering::SeqCst)
}

/// Make SIGTERM and SIGINT drain the daemon (spec §3.1) instead of ending it where it stands.
fn install_signal_handlers(waker: &Waker) -> anyhow::Result<()> {
    SIGNAL_WAKE_FD.store(waker.leak_raw_fd()?, Ordering::SeqCst);
    let handler = on_termination_signal as extern "C" fn(libc::c_int);
    for signal in [libc::SIGTERM, libc::SIGINT] {
        // SAFETY: the handler does only async-signal-safe work (an atomic store and `write`).
        let previous = unsafe { libc::signal(signal, handler as libc::sighandler_t) };
        anyhow::ensure!(
            previous != libc::SIG_ERR,
            "could not install the handler for signal {signal}: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
