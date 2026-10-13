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

fn main() -> anyhow::Result<()> {
    let c = <Cli as clap::Parser>::parse();
    let cache = if c.no_cache {
        prism::mcp::CacheMode::NoCache
    } else if let Some(d) = c.cache_dir {
        prism::mcp::CacheMode::Dir(d)
    } else {
        prism::mcp::CacheMode::Default
    };
    let refresh_policy = match c.refresh_policy.as_str() {
        "warn-only" => prism::mcp::RefreshPolicy::WarnOnly,
        "auto-full" => prism::mcp::RefreshPolicy::AutoFull,
        "auto-incremental" => prism::mcp::RefreshPolicy::AutoIncremental,
        _ => unreachable!("clap value_parser restricts refresh-policy"),
    };
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
        return prism::mcp::mem_report::run(&cfg, &options, &mut std::io::stderr().lock());
    }

    match owner {
        Some(owner) => prism::mcp::run_with_owner(cfg, owner),
        None => prism::mcp::run(cfg),
    }
}
