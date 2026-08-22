#[derive(clap::Parser)]
struct Cli {
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
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u64).range(0..=600))]
    first_call_wait: u64,
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
    } else {
        prism::mcp::StartupMode::Lazy
    };
    cfg.first_call_wait = std::time::Duration::from_secs(c.first_call_wait);
    prism::mcp::run(cfg)
}
