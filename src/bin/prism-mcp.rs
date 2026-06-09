#[derive(clap::Parser)]
struct Cli {
    #[arg(long)]
    repo: std::path::PathBuf,
    #[arg(long, conflicts_with = "cache_dir")]
    no_cache: bool,
    #[arg(long)]
    cache_dir: Option<std::path::PathBuf>,
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
    prism::mcp::run(prism::mcp::ServerConfig {
        repo_root: c.repo,
        cache,
    })
}
