#[cfg(any(target_os = "macos", test))]
#[path = "prism-mcp/malloc.rs"]
mod malloc;

#[derive(clap::Parser)]
struct Cli {
    #[command(flatten)]
    owner: prism::cli::OwnerArgs,
    #[arg(long)]
    repo: std::path::PathBuf,
    #[arg(long, conflicts_with = "cache_dir")]
    no_cache: bool,
    #[arg(long)]
    cache_dir: Option<std::path::PathBuf>,
    #[arg(long, default_value = "warn-only", value_parser = ["warn-only", "auto-full", "auto-incremental"])]
    refresh_policy: String,
    #[arg(long)]
    eager: bool,
    #[arg(long, conflicts_with = "eager")]
    warm_at_startup: bool,
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u64).range(0..=600))]
    first_call_wait: u64,
    /// Unload a stdio or shared index after this many idle seconds (0 disables). Off unless this flag or
    /// PRISM_MCP_INDEX_IDLE_SECS is set; measurement runs pass 600 explicitly.
    #[arg(long)]
    index_idle_secs: Option<String>,
    /// Prototype A+C allocator release, only after whole-session idle disposal.
    #[arg(long, hide = true)]
    index_idle_pressure_relief: bool,
    /// Measurement only (needs PRISM_MCP_MEM_REPORT=1): run this tools/call workload after the build.
    #[arg(long, hide = true)]
    mem_workload: Option<std::path::PathBuf>,
    /// Measurement only: run one refresh at the end, for the refresh peak.
    #[arg(long, hide = true)]
    mem_refresh: bool,
    /// Measurement only: edit manifest for a dedicated private mirror (requires feature).
    #[cfg(feature = "memreport-changed-refresh")]
    #[arg(long, hide = true, conflicts_with_all = ["mem_refresh", "mem_emit_workload"])]
    mem_changed_refresh: Option<std::path::PathBuf>,
    /// Measurement only: seconds to wait before the steady-state sample.
    #[arg(long, hide = true, default_value_t = 5, value_parser = clap::value_parser!(u64).range(0..=600))]
    mem_settle_secs: u64,
    /// Measurement only: print a workload for this many functions and exit.
    #[arg(long, hide = true, value_parser = clap::value_parser!(u64).range(1..=10000), conflicts_with_all = ["mem_workload", "mem_refresh"])]
    mem_emit_workload: Option<u64>,
    /// Use the shared per-repository daemon instead of an index of this process's own; starts it
    /// when none is running, and fails closed when it cannot be reached.
    #[arg(long, conflicts_with_all = ["standalone", "no_cache", "eager", "owner_compiler", "owner_config"])]
    shared: bool,
    /// Serve from an index of this process's own (the default).
    #[arg(long)]
    standalone: bool,
    /// With --shared: seconds to reach a ready daemon, starting one if needed (default 10).
    #[arg(long, requires = "shared", value_parser = clap::value_parser!(u64).range(1..=600))]
    daemon_start_timeout: Option<u64>,
    /// With --shared: idle seconds before a daemon this shim starts retires (default 900).
    #[arg(long, requires = "shared", value_parser = clap::value_parser!(u64).range(1..=86_400))]
    idle_ttl: Option<u64>,
    /// With --shared: connections a daemon this shim starts accepts at once (default 64).
    #[arg(long, requires = "shared", value_parser = clap::value_parser!(u32).range(1..=4096))]
    max_clients: Option<u32>,
}

/// `prism-mcp daemon ...`: the shared per-repository daemon (spec §3.1, §4.2). Internal:
/// `prism-mcp --shared` (the shim) starts it; it serves only through its Unix socket.
#[derive(clap::Parser)]
#[command(name = "prism-mcp daemon")]
struct DaemonCli {
    #[arg(long)]
    repo: std::path::PathBuf,
    /// The cache base directory; the user cache directory when absent.
    #[arg(long)]
    cache_dir: Option<std::path::PathBuf>,
    #[arg(long, default_value = "warn-only", value_parser = ["warn-only", "auto-full", "auto-incremental"])]
    refresh_policy: String,
    /// Build the index at start instead of on the first tool call (spec §11 item 7).
    #[arg(long)]
    warm_at_startup: bool,
    /// Seconds with no client and no build before the daemon retires.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_IDLE_TTL_SECS, value_parser = clap::value_parser!(u64).range(1..=86_400))]
    idle_ttl: u64,
    /// Unload the daemon index after idle seconds (0 or absent disables); ignores environment.
    #[arg(long)]
    index_idle_secs: Option<String>,
    #[arg(long, hide = true)]
    index_idle_pressure_relief: bool,
    /// Pending and admitted connections together.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_MAX_CLIENTS, value_parser = clap::value_parser!(u32).range(1..=4096))]
    max_clients: u32,
    /// The full identity key the starting shim computed; the daemon refuses to start on a
    /// different one.
    #[arg(long)]
    key: Option<String>,
}

/// `prism-mcp daemon-status`: what the shared daemons in the runtime directory are doing (spec
/// §4.7). It reads and never removes anything.
#[derive(clap::Parser)]
#[command(name = "prism-mcp daemon-status")]
struct StatusCli {
    /// Show only the daemon of this repository (with the cache directory and refresh policy
    /// given, for this build of prism).
    #[arg(long)]
    repo: Option<std::path::PathBuf>,
    #[arg(long, requires = "repo")]
    cache_dir: Option<std::path::PathBuf>,
    #[arg(long, requires = "repo", value_parser = ["warn-only", "auto-full", "auto-incremental"])]
    refresh_policy: Option<String>,
    /// The runtime base, as PRISM_MCP_RUNTIME_DIR gives it: the daemons' files are in its
    /// prism-mcpd directory.
    #[arg(long)]
    runtime_dir: Option<std::path::PathBuf>,
    /// Seconds each daemon has to answer.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_CONTROL_TIMEOUT_SECS, value_parser = clap::value_parser!(u64).range(1..=600))]
    timeout: u64,
}

/// `prism-mcp daemon-shutdown`: ask the shared daemon of a repository to drain once it has no
/// client (spec §4.7). It never disconnects a client.
#[derive(clap::Parser)]
#[command(name = "prism-mcp daemon-shutdown")]
struct ShutdownCli {
    #[arg(long)]
    repo: std::path::PathBuf,
    #[arg(long)]
    cache_dir: Option<std::path::PathBuf>,
    #[arg(long, default_value = "warn-only", value_parser = ["warn-only", "auto-full", "auto-incremental"])]
    refresh_policy: String,
    /// The runtime base, as PRISM_MCP_RUNTIME_DIR gives it.
    #[arg(long)]
    runtime_dir: Option<std::path::PathBuf>,
    /// Ask only the daemon with this pid and start time (as `prism-mcp --shared` reports them).
    #[arg(long, requires = "start_time")]
    pid: Option<u32>,
    #[arg(long, requires = "pid")]
    start_time: Option<u64>,
    /// Seconds for the whole command, the wait for the drain included.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_CONTROL_TIMEOUT_SECS, value_parser = clap::value_parser!(u64).range(1..=600))]
    timeout: u64,
}

fn refresh_policy_from(name: &str) -> prism::mcp::RefreshPolicy {
    match name {
        "warn-only" => prism::mcp::RefreshPolicy::WarnOnly,
        "auto-full" => prism::mcp::RefreshPolicy::AutoFull,
        "auto-incremental" => prism::mcp::RefreshPolicy::AutoIncremental,
        _ => unreachable!("clap value_parser restricts refresh-policy"),
    }
}

fn cache_mode(no_cache: bool, cache_dir: Option<std::path::PathBuf>) -> prism::mcp::CacheMode {
    if no_cache {
        prism::mcp::CacheMode::NoCache
    } else if let Some(d) = cache_dir {
        prism::mcp::CacheMode::Dir(d)
    } else {
        prism::mcp::CacheMode::Default
    }
}

fn daemon_main() -> anyhow::Result<()> {
    // The first argument is the word `daemon`; clap reads it as the program name.
    let c = <DaemonCli as clap::Parser>::parse_from(std::env::args_os().skip(1));
    prism::mcp::run_daemon(daemon_options(c)?)
}

fn daemon_options(c: DaemonCli) -> anyhow::Result<prism::mcp::DaemonOptions> {
    let mut config = prism::mcp::ServerConfig::new(c.repo);
    config.cache = cache_mode(false, c.cache_dir);
    config.refresh_policy = refresh_policy_from(&c.refresh_policy);
    config.startup = if c.warm_at_startup {
        prism::mcp::StartupMode::Background
    } else {
        prism::mcp::StartupMode::Lazy
    };
    let idle_policy =
        prism::mcp::idle::IdlePolicy::resolve(c.index_idle_secs.as_deref(), None, &config, false)?
            .policy
            .with_pressure_relief(c.index_idle_pressure_relief);
    Ok(prism::mcp::DaemonOptions {
        idle_policy,
        config,
        idle_ttl: std::time::Duration::from_secs(c.idle_ttl),
        max_clients: c.max_clients,
        expected_key: c.key,
    })
}

/// The configuration whose daemon an operator command is about.
fn operator_config(
    repo: std::path::PathBuf,
    cache_dir: Option<std::path::PathBuf>,
    refresh_policy: &str,
) -> prism::mcp::ServerConfig {
    let mut config = prism::mcp::ServerConfig::new(repo);
    config.cache = cache_mode(false, cache_dir);
    config.refresh_policy = refresh_policy_from(refresh_policy);
    config
}

fn status_main() -> i32 {
    // The first argument is the word `daemon-status`; clap reads it as the program name.
    let c = <StatusCli as clap::Parser>::parse_from(std::env::args_os().skip(1));
    prism::mcp::run_daemon_status(&prism::mcp::StatusOptions {
        config: c.repo.map(|repo| {
            operator_config(
                repo,
                c.cache_dir,
                c.refresh_policy.as_deref().unwrap_or("warn-only"),
            )
        }),
        runtime_base: c.runtime_dir,
        timeout: std::time::Duration::from_secs(c.timeout),
    })
}

fn shutdown_main() -> i32 {
    let c = <ShutdownCli as clap::Parser>::parse_from(std::env::args_os().skip(1));
    prism::mcp::run_daemon_shutdown(&prism::mcp::ShutdownOptions {
        config: operator_config(c.repo, c.cache_dir, &c.refresh_policy),
        runtime_base: c.runtime_dir,
        instance: c.pid.zip(c.start_time),
        timeout: std::time::Duration::from_secs(c.timeout),
    })
}

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    malloc::restart_if_needed();

    let first = std::env::args_os().nth(1);
    if first.as_ref().is_some_and(|first| first == "daemon") {
        return daemon_main();
    }
    if first.as_ref().is_some_and(|first| first == "daemon-status") {
        std::process::exit(status_main());
    }
    if first
        .as_ref()
        .is_some_and(|first| first == "daemon-shutdown")
    {
        std::process::exit(shutdown_main());
    }
    let c = <Cli as clap::Parser>::parse();
    let cache = cache_mode(c.no_cache, c.cache_dir);
    let refresh_policy = refresh_policy_from(&c.refresh_policy);
    let mut cfg = prism::mcp::ServerConfig::new(c.repo);
    cfg.cache = cache;
    cfg.refresh_policy = refresh_policy;
    cfg.startup = if c.eager {
        prism::mcp::StartupMode::Eager
    } else if c.warm_at_startup {
        prism::mcp::StartupMode::Background
    } else {
        prism::mcp::StartupMode::Lazy
    };
    cfg.first_call_wait = std::time::Duration::from_secs(c.first_call_wait);
    let owner = c.owner.options()?;

    let report_mode = std::env::var("PRISM_MCP_MEM_REPORT").is_ok_and(|value| value == "1");
    #[cfg(feature = "memreport-changed-refresh")]
    let changed_refresh = c.mem_changed_refresh;
    #[cfg(not(feature = "memreport-changed-refresh"))]
    let changed_refresh = None;
    let mem_flag = c.mem_workload.is_some()
        || c.mem_refresh
        || c.mem_emit_workload.is_some()
        || changed_refresh.is_some();
    if mem_flag && !report_mode {
        anyhow::bail!(
            "--mem-workload, --mem-refresh, --mem-emit-workload and --mem-changed-refresh (when compiled) are measurement-only and need PRISM_MCP_MEM_REPORT=1"
        );
    }
    if report_mode {
        eprintln!("prism-mcp: PRISM_MCP_MEM_REPORT=1 puts this process in memory-report mode; it is not serving MCP");
        anyhow::ensure!(
            c.eager,
            "PRISM_MCP_MEM_REPORT=1 selects memory-report mode, which needs --eager and never serves MCP; unset PRISM_MCP_MEM_REPORT to run the server"
        );
        // Count tree-sitter's allocations from here on. Nothing has parsed yet, and this process
        // is single-purpose; library code and tests must never install the hooks.
        // SAFETY: this is the main thread, before the index build or any other parse starts; no
        // other thread is inside tree-sitter, and this is the only call.
        unsafe { prism::mcp::mem_tree_alloc::install() };
        anyhow::ensure!(
            owner.is_none(),
            "memory-report mode does not support --owner-compiler/--owner-config"
        );
        if let Some(functions) = c.mem_emit_workload {
            return prism::mcp::mem_report::emit_workload(
                &cfg,
                functions as usize,
                &mut std::io::stdout().lock(),
            );
        }
        let options = prism::mcp::mem_report::MemReportOptions {
            workload: c.mem_workload,
            refresh: c.mem_refresh,
            changed_refresh,
            settle: std::time::Duration::from_secs(c.mem_settle_secs),
        };
        // Do not hold the stderr lock while the report runs. The index is built on pool worker
        // threads while this thread waits, and any `eprintln!` on a worker (the product has several,
        // and the out-of-memory message in `mem_tree_alloc`) takes that same lock: holding it here
        // would hang the build forever. The buffer is filled after the last sample, so writing it
        // out afterwards changes nothing that was measured.
        let mut report = Vec::new();
        prism::mcp::mem_report::run(&cfg, &options, &mut report)?;
        use std::io::Write as _;
        let mut stderr = std::io::stderr().lock();
        stderr.write_all(&report)?;
        return Ok(stderr.flush()?);
    }

    if c.shared {
        std::process::exit(prism::mcp::run_shim(prism::mcp::ShimOptions {
            index_idle_secs: c.index_idle_secs,
            index_idle_pressure_relief: c.index_idle_pressure_relief,
            config: cfg,
            start_timeout: std::time::Duration::from_secs(
                c.daemon_start_timeout
                    .unwrap_or(prism::mcp::DEFAULT_DAEMON_START_TIMEOUT_SECS),
            ),
            idle_ttl: c.idle_ttl,
            max_clients: c.max_clients,
        }));
    }

    let ambient = std::env::var("PRISM_MCP_INDEX_IDLE_SECS").ok();
    let idle = prism::mcp::idle::IdlePolicy::resolve(
        c.index_idle_secs.as_deref(),
        ambient.as_deref(),
        &cfg,
        owner.is_some(),
    )?;
    if let Some(note) = idle.note {
        eprintln!("{note}");
    }
    if let Some(note) = idle
        .policy
        .pressure_relief_note(c.index_idle_pressure_relief, false)
    {
        eprintln!("{note}");
    }
    let policy = idle
        .policy
        .with_pressure_relief(c.index_idle_pressure_relief);
    match owner {
        Some(owner) => prism::mcp::run_with_owner(cfg, owner),
        None => prism::mcp::run_with_idle_policy(cfg, policy),
    }
}

#[cfg(test)]
mod mem_cli_tests {
    use super::Cli;
    use clap::Parser;

    #[test]
    fn mem_changed_refresh_cli_surface_matches_feature() {
        let parsed = Cli::try_parse_from([
            "prism-mcp",
            "--repo",
            ".",
            "--mem-changed-refresh",
            "edit.json",
        ]);
        #[cfg(feature = "memreport-changed-refresh")]
        assert_eq!(
            parsed.unwrap().mem_changed_refresh.unwrap(),
            std::path::PathBuf::from("edit.json")
        );
        #[cfg(not(feature = "memreport-changed-refresh"))]
        assert_eq!(
            parsed.err().unwrap().kind(),
            clap::error::ErrorKind::UnknownArgument
        );
    }
}

#[cfg(test)]
#[path = "prism-mcp/daemon_idle_tests.rs"]
mod daemon_idle_tests;

#[cfg(test)]
#[path = "prism-mcp/idle_tests.rs"]
mod idle_tests;
