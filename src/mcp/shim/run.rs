//! `prism-mcp --repo X --shared`: the shim's process (spec §4.1). Resolve the configuration, the
//! connection settings and the key exactly as a standalone server would; find or start the
//! daemon before one deadline; then pump. Every failure is one line on standard error and a
//! non-zero exit; nothing builds a private index.
use super::arbitrate::Arbiter;
use super::diagnostic::{fail_closed_line, one_line, standalone_invocation, Where};
use super::pump::{pump, PumpEnd};
use super::spawn::{DaemonArgs, ProcessStarter};
use crate::mcp::daemon::peer::peer_uid;
use crate::mcp::session::canonical_config;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::shared::identity::DaemonIdentity;
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::shared::preamble::Hello;
use crate::mcp::shared::runtime_dir::runtime_dir;
use crate::mcp::{CacheMode, ServerConfig, StartupMode};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// `--daemon-start-timeout` default, seconds (spec §4.1 step 4, §8 step 4).
pub const DEFAULT_DAEMON_START_TIMEOUT_SECS: u64 = 10;

/// What `prism-mcp --shared` was started with.
#[derive(Clone, Debug)]
pub struct ShimOptions {
    /// The repository, the cache (`Default` or a directory; never `NoCache`), the refresh
    /// policy, `startup` (`Lazy`, or `Background` for `--warm-at-startup`; never `Eager`) and
    /// the first-call wait, as for a standalone server.
    pub config: ServerConfig,
    /// `--daemon-start-timeout`: the one deadline from here to `READY`.
    pub start_timeout: Duration,
    /// `--idle-ttl` and `--max-clients` for a daemon this shim starts.
    pub idle_ttl: Option<u64>,
    pub max_clients: Option<u32>,
}

/// Run a shim until its client or its daemon ends the session. Returns the exit status: 0 when
/// the client closed the session, 1 otherwise, with one line on standard error. Call it from the
/// `prism-mcp` executable only: it starts the daemon as `prism-mcp daemon …` from
/// `current_exe()` (review round 1, Opus S5).
pub fn run_shim(options: ShimOptions) -> i32 {
    match shim(&options) {
        Ok(()) => 0,
        Err(line) => {
            eprintln!("{line}");
            1
        }
    }
}

fn shim(options: &ShimOptions) -> Result<(), String> {
    // The `--standalone` invocation comes first, from the configuration as given, so that every
    // failure, those before the key is known included, names it (review round 1, sol 5).
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("prism-mcp"));
    let given_root = std::path::absolute(&options.config.repo_root)
        .unwrap_or_else(|_| options.config.repo_root.clone());
    let standalone = standalone_invocation(
        &exe,
        &ServerConfig {
            repo_root: given_root,
            ..options.config.clone()
        },
    );
    let fail = |reason: &dyn std::fmt::Display, at: Where<'_>| {
        fail_closed_line(&reason.to_string(), &at, &standalone)
    };
    let early = |reason: &dyn std::fmt::Display| {
        fail(
            reason,
            Where {
                key: None,
                runtime_dir: None,
                paths: None,
                dir_validated: false,
            },
        )
    };
    // Spec §4.1 step 2: never a silent downgrade. The command line rejects these first.
    if matches!(options.config.cache, CacheMode::NoCache) {
        return Err(early(
            &"--shared needs a cache; --no-cache is served in process",
        ));
    }
    if options.config.startup == StartupMode::Eager {
        return Err(early(
            &"--shared cannot build eagerly; --eager is served in process",
        ));
    }
    let cfg = canonical_config(&options.config).map_err(|e| early(&e))?;
    let identity = DaemonIdentity::resolve(&cfg).map_err(|e| early(&e))?;
    let key = identity.key();
    // Spec §4.1 step 3: this shim's own settings, from its own environment and flags; an
    // invalid value warns here, as it does for a standalone server.
    let settings = ClientSettings::new(
        ConnectionSettings::from_env(),
        cfg.first_call_wait.as_secs(),
        cfg.startup == StartupMode::Background,
    )
    .map_err(|e| early(&e))?;
    let hello = Hello {
        identity: key,
        settings,
    }
    .encode();
    // Spec §4.1 step 4: the one deadline, from here to `READY`.
    let deadline = Instant::now() + options.start_timeout;
    let dir = runtime_dir().map_err(|e| {
        fail(
            &e,
            Where {
                key: Some(&key),
                runtime_dir: None,
                paths: None,
                dir_validated: false,
            },
        )
    })?;
    let paths = DaemonPaths::new(&dir, &key).map_err(|e| {
        fail(
            &e,
            Where {
                key: Some(&key),
                runtime_dir: Some(&dir),
                paths: None,
                dir_validated: false,
            },
        )
    })?;
    let at = |dir_validated| Where {
        key: Some(&key),
        runtime_dir: Some(&dir),
        paths: Some(&paths),
        dir_validated,
    };
    let mut starter = ProcessStarter {
        exe,
        args: DaemonArgs {
            repo_root: identity.repo_root().to_path_buf(),
            cache_base: identity.cache_base().to_path_buf(),
            refresh_policy: identity.refresh_policy(),
            key,
            warm_at_startup: cfg.startup == StartupMode::Background,
            idle_ttl: options.idle_ttl,
            max_clients: options.max_clients,
        },
        runtime_dir: dir.clone(),
    };
    let arbiter = Arbiter {
        paths: &paths,
        key,
        hello: &hello,
        deadline,
        peer_uid,
        hook: None,
    };
    let connected = arbiter
        .arbitrate(&mut starter)
        .map_err(|failure| fail(&failure, at(failure.runtime_dir_validated())))?;
    let pid = connected.ready.pid;
    eprintln!(
        "{}",
        one_line(&format!(
            "prism-mcp: shared daemon pid {pid} (started {}, generation {}) at {}",
            connected.ready.start_time,
            connected.ready.generation,
            paths.socket().display()
        ))
    );
    if let Some(started) = connected.started {
        std::thread::spawn(move || started.reap());
    }
    match pump(
        std::io::stdin(),
        &mut std::io::stdout().lock(),
        connected.stream,
    ) {
        PumpEnd::ClientClosed => Ok(()),
        end => Err(one_line(&format!(
            "prism-mcp: {end} (shared daemon pid {pid}); this session cannot continue, and the \
             shim does not reconnect once MCP bytes have been forwarded"
        ))),
    }
}
