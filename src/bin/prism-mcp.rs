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
    /// Measurement only (needs PRISM_MCP_MEM_REPORT=1): run this tools/call workload after the build.
    #[arg(long, hide = true)]
    mem_workload: Option<std::path::PathBuf>,
    /// Measurement only: run one refresh at the end, for the refresh peak.
    #[arg(long, hide = true)]
    mem_refresh: bool,
    /// Measurement only: seconds to wait before the steady-state sample.
    #[arg(long, hide = true, default_value_t = 5, value_parser = clap::value_parser!(u64).range(0..=600))]
    mem_settle_secs: u64,
    /// Measurement only: print a workload for this many functions and exit.
    #[arg(long, hide = true, value_parser = clap::value_parser!(u64).range(1..=10000), conflicts_with_all = ["mem_workload", "mem_refresh"])]
    mem_emit_workload: Option<u64>,
}

/// `prism-mcp daemon ...`: the shared per-repository daemon (spec §3.1, §4.2). Internal: the
/// shim of a later slice starts it; it serves only through its Unix socket.
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
    /// Pending and admitted connections together.
    #[arg(long, default_value_t = prism::mcp::DEFAULT_MAX_CLIENTS, value_parser = clap::value_parser!(u32).range(1..=4096))]
    max_clients: u32,
    /// The full identity key the starting shim computed; the daemon refuses to start on a
    /// different one.
    #[arg(long)]
    key: Option<String>,
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
    let mut config = prism::mcp::ServerConfig::new(c.repo);
    config.cache = cache_mode(false, c.cache_dir);
    config.refresh_policy = refresh_policy_from(&c.refresh_policy);
    config.startup = if c.warm_at_startup {
        prism::mcp::StartupMode::Background
    } else {
        prism::mcp::StartupMode::Lazy
    };
    prism::mcp::run_daemon(prism::mcp::DaemonOptions {
        config,
        idle_ttl: std::time::Duration::from_secs(c.idle_ttl),
        max_clients: c.max_clients,
        expected_key: c.key,
    })
}

fn main() -> anyhow::Result<()> {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|first| first == "daemon")
    {
        return daemon_main();
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
    let mem_flag = c.mem_workload.is_some() || c.mem_refresh || c.mem_emit_workload.is_some();
    if mem_flag && !report_mode {
        anyhow::bail!(
            "--mem-workload, --mem-refresh and --mem-emit-workload are measurement-only and need PRISM_MCP_MEM_REPORT=1"
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

    match owner {
        Some(owner) => prism::mcp::run_with_owner(cfg, owner),
        None => prism::mcp::run(cfg),
    }
}
