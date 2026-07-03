use anyhow::{Context, Result};
use clap::Parser;
use prism::algorithms;
use prism::ast::ParsedFile;
use prism::cpg::{CodePropertyGraph, CpgContext};
use prism::cpg_cache::{self, CacheResult};
use prism::diff::DiffInput;
use prism::languages::Language;
use prism::output;
use prism::slice::{AlgorithmError, MultiSliceResult, SliceConfig, SliceFinding, SlicingAlgorithm};
use prism::type_db::TypeDatabase;
use prism::type_provider::LanguageVersion;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Validate `--diagram-node-cap`: must be >= 4.
///
/// 4 is the smallest cap that meaningfully fits head + ghost + tail with at
/// least one node on each side of the elision point.  Values below 4 cause
/// `truncate_to_cap`'s arithmetic to produce more nodes than the cap allows
/// (the internal clamp handles it defensively, but we surface a clear error
/// at the CLI boundary so users understand why their cap was rejected).
fn parse_diagram_cap(s: &str) -> Result<usize, String> {
    let n: usize = s.parse().map_err(|e| format!("invalid integer: {}", e))?;
    if n < 4 {
        return Err(format!(
            "--diagram-node-cap must be >= 4 (got {}); \
             values below 4 cannot fit head + ghost + tail",
            n
        ));
    }
    Ok(n)
}

#[derive(Parser, Debug)]
#[command(
    name = "slicing",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_SHA"), ")"),
    about = "Code slicing for defect-focused automated code review (arXiv:2505.17928)",
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    review: ReviewArgs,
}

#[derive(clap::Args, Debug)]
struct ReviewArgs {
    /// Path to the repository root
    #[arg(short, long, required_unless_present = "list_algorithms")]
    repo: Option<PathBuf>,

    /// Slicing algorithm (see --list-algorithms for all options)
    #[arg(short, long, default_value = "leftflow")]
    algorithm: String,

    /// Diff input: path to a unified diff file, or a JSON diff spec
    #[arg(short, long, required_unless_present = "list_algorithms")]
    diff: Option<PathBuf>,

    /// Output format: text, json, paper, review, callers, mermaid
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Maximum number of nodes a single Mermaid diagram may render before truncation.
    /// Must be >= 4 (the minimum that fits head + ghost + tail).
    #[arg(long, default_value_t = 40, value_parser = parse_diagram_cap)]
    diagram_node_cap: usize,

    /// Exit non-zero if any bug-class diagram warning is produced.
    #[arg(long, default_value_t = false)]
    strict_diagrams: bool,

    /// Maximum branch lines to include fully (default: 5)
    #[arg(long, default_value = "5")]
    max_branch_lines: usize,

    /// Don't include return statements in LeftFlow/FullFlow
    #[arg(long)]
    no_returns: bool,

    /// Don't trace into called functions (FullFlow only)
    #[arg(long)]
    no_trace_callees: bool,

    /// List all available algorithms and exit
    #[arg(long)]
    list_algorithms: bool,

    // --- Algorithm-specific flags ---
    /// Barrier slice: max call depth (default: 2)
    #[arg(long, default_value = "2")]
    barrier_depth: usize,

    /// Barrier slice: comma-separated function names to not trace into
    #[arg(long, default_value = "")]
    barrier_symbols: String,

    /// Chop: source location (file:line)
    #[arg(long)]
    chop_source: Option<String>,

    /// Chop: sink location (file:line)
    #[arg(long)]
    chop_sink: Option<String>,

    /// Taint: explicit source location (file:line), can be repeated
    #[arg(long)]
    taint_source: Vec<String>,

    /// Conditioned slice: condition predicate (e.g., "x==5", "x!=null")
    #[arg(long)]
    condition: Option<String>,

    /// Delta slice: path to old version of the repository
    #[arg(long)]
    old_repo: Option<PathBuf>,

    /// Spiral slice: maximum ring level (1-6)
    #[arg(long, default_value = "4")]
    spiral_max_ring: usize,

    /// Quantum slice: target variable name
    #[arg(long)]
    quantum_var: Option<String>,

    /// Horizontal slice: peer pattern (e.g., "decorator:@app.route", "name:test_*")
    #[arg(long)]
    peer_pattern: Option<String>,

    /// Vertical slice: comma-separated layer names (highest to lowest)
    #[arg(long)]
    layers: Option<String>,

    /// Angle slice: concern to trace (error_handling, logging, auth, caching, or custom keywords)
    #[arg(long)]
    concern: Option<String>,

    /// 3D slice: how many days back to look in git history
    #[arg(long, default_value = "90")]
    temporal_days: usize,

    /// Max caller-graph traversal depth for --format callers (default: 5)
    #[arg(long, default_value = "5")]
    caller_depth: usize,

    /// --format review only: minimum finding severity to include (findings
    /// below this floor are dropped, along with any block that then has no
    /// remaining finding on one of its lines). Does not affect --format
    /// json/text/paper.
    #[arg(long, default_value = "warning", value_parser = ["info", "suggestion", "warning", "concern"])]
    review_min_severity: String,

    /// --format review only: keep every block regardless of whether it has
    /// a retained finding. This restores block retention ONLY — it does not
    /// lower the severity floor (pair with --review-min-severity info to
    /// also see low-severity findings) and it does not restore slice_lines/
    /// diff_lines in review output; for the full pre-compaction shape use
    /// --format json. Does not affect --format json/text/paper.
    #[arg(long, default_value_t = false)]
    review_full_slices: bool,

    /// Only process these files from the diff (comma-separated paths).
    /// If omitted, process all files in the diff.
    #[arg(long)]
    files: Option<String>,

    /// Build CPG from only diff-changed files + direct callers/callees.
    /// Reduces construction time for large repos with small diffs.
    #[arg(long)]
    scoped_cpg: bool,

    /// Path to compile_commands.json for C/C++ type enrichment.
    /// Enables precise whole-struct detection, typedef resolution,
    /// and virtual dispatch via class hierarchy analysis.
    #[arg(long)]
    compile_commands: Option<PathBuf>,

    /// Directory to cache the CPG for faster subsequent runs.
    /// On the first run, the CPG is serialized to this directory.
    /// On subsequent runs, the cache is loaded if all file hashes match.
    ///
    /// Note: the cache covers only the files referenced in the current diff,
    /// not the entire repository. This means it is per-MR: re-running the
    /// same diff is a cache hit, but a different diff touching different
    /// files will miss and trigger a full rebuild.
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// Ignore any existing cache and force a full CPG rebuild.
    #[arg(long)]
    no_cache: bool,

    // --- Target language version flags (stored, informational in Phase 1) ---
    /// Target Python version (e.g., "3.8", "3.11"). Stored for future use.
    #[arg(long)]
    python_version: Option<String>,

    /// Target Go version (e.g., "1.21"). Stored for future use.
    #[arg(long)]
    go_version: Option<String>,

    /// Target Node.js version (e.g., "18", "20"). Stored for future use.
    #[arg(long)]
    node_version: Option<String>,

    /// Target TypeScript version (e.g., "5.0"). Stored for future use.
    #[arg(long)]
    typescript_version: Option<String>,

    /// Target Java version (e.g., "17", "21"). Stored for future use.
    #[arg(long)]
    java_version: Option<String>,

    /// Target Rust edition/version (e.g., "2021"). Stored for future use.
    #[arg(long)]
    rust_version: Option<String>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Whole-repo navigation/architecture queries.
    Nav(NavArgs),
}

#[derive(clap::Args, Debug)]
struct NavArgs {
    /// Ignore the whole-repo navigation cache and force a full CPG rebuild.
    #[arg(long, conflicts_with = "cache_dir")]
    no_cache: bool,

    /// Directory to use for the whole-repo navigation cache.
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    #[command(subcommand)]
    query: NavQuery,
}

#[derive(clap::Subcommand, Debug)]
enum NavQuery {
    /// CPG nodes at a file:line (plus the enclosing function).
    NodesAt {
        #[arg(long)]
        repo: std::path::PathBuf,
        /// `file:line`
        #[arg(long)]
        location: String,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
    Callers {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        location: Option<String>,
        #[arg(long, default_value_t = 1)]
        depth: usize,
        #[arg(long, default_value = "all", value_parser = ["exact", "all"])]
        confidence: String,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
    Callees {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        location: Option<String>,
        #[arg(long, default_value_t = 1)]
        depth: usize,
        #[arg(long, default_value = "all", value_parser = ["exact", "all"])]
        confidence: String,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
    Ego {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        location: Option<String>,
        #[arg(long, default_value_t = 1)]
        hops: usize,
        #[arg(long, default_value = "Call,Return,DataFlow,Contains")]
        edges: String,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
    /// Outbound module dependencies of a file (call-derived + labeled imports).
    ModuleDeps {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long)]
        file: String,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
    /// Whole-repo file->file module dependency graph.
    RepoMap {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
    /// Whole-repo call-resolution telemetry.
    CallStats {
        #[arg(long)]
        repo: std::path::PathBuf,
    },
    /// Whole-repo interface-dispatch in-scope manifest (Phase-IP PR-2 §8a).
    InterfaceManifest {
        #[arg(long)]
        repo: std::path::PathBuf,
    },
    /// Whole-repo function inventory from the FunctionTable (Tier-A spec §2.3).
    Functions {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long, default_value = "json", value_parser = ["text", "json"])]
        format: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // Run the whole command on the large-stack pool. prism walks tree-sitter
    // ASTs recursively in parsing, CPG build, and the live-type scan; on deeply
    // nested files those overflow a default ~2 MiB thread/worker stack. install()
    // runs the command and every nested par_iter on big-stack workers, covering
    // all of run_nav/run_review (incl. direct changed-file / delta / contract
    // parses) in one place. See prism::build_pool.
    prism::build_pool::install(|| match &cli.command {
        Some(Command::Nav(nav)) => run_nav(nav),
        None => run_review(&cli.review),
    })
}

fn run_nav(nav: &NavArgs) -> anyhow::Result<()> {
    match &nav.query {
        NavQuery::NodesAt {
            repo,
            location,
            format,
        } => {
            let (file, line) = location
                .rsplit_once(':')
                .and_then(|(f, l)| l.parse::<usize>().ok().map(|n| (f.to_string(), n)))
                .ok_or_else(|| anyhow::anyhow!("--location must be file:line"))?;
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let ev = prism::navigation::queries::nodes_at(&session, &file, line);
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::Callers {
            repo,
            symbol,
            file,
            location,
            depth,
            confidence,
            format,
        } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let exact = confidence == "exact";
            match prism::navigation::queries::callers_with_confidence(
                &session,
                symbol.as_deref(),
                file.as_deref(),
                location.as_deref(),
                *depth,
                exact,
            ) {
                Ok(ev) => {
                    println!("{}", prism::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism::output::navigation::render_err(&e, format);
                    println!("{s}");
                    std::process::exit(code);
                }
            }
        }
        NavQuery::Callees {
            repo,
            symbol,
            file,
            location,
            depth,
            confidence,
            format,
        } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let exact = confidence == "exact";
            match prism::navigation::queries::callees_with_confidence(
                &session,
                symbol.as_deref(),
                file.as_deref(),
                location.as_deref(),
                *depth,
                exact,
            ) {
                Ok(ev) => {
                    println!("{}", prism::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism::output::navigation::render_err(&e, format);
                    println!("{s}");
                    std::process::exit(code);
                }
            }
        }
        NavQuery::Ego {
            repo,
            symbol,
            file,
            location,
            hops,
            edges,
            format,
        } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let edge_kinds: Vec<&str> = edges.split(',').collect();
            match prism::navigation::queries::ego_graph(
                &session,
                symbol.as_deref(),
                file.as_deref(),
                location.as_deref(),
                *hops,
                &edge_kinds,
            ) {
                Ok(ev) => {
                    println!("{}", prism::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism::output::navigation::render_err(&e, format);
                    println!("{s}");
                    std::process::exit(code);
                }
            }
        }
        NavQuery::ModuleDeps { repo, file, format } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let ev = prism::navigation::module_graph::module_deps(&session, file);
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::RepoMap { repo, format } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let ev = prism::navigation::module_graph::repo_map(&session);
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::CallStats { repo } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let stats = prism::navigation::queries::call_stats(session.index.call_graph());
            println!("{}", serde_json::to_string_pretty(&stats)?);
            Ok(())
        }
        NavQuery::InterfaceManifest { repo } => {
            let session = build_session(repo, nav.no_cache, nav.cache_dir.as_deref())?;
            let manifest =
                prism::navigation::queries::interface_dispatch_manifest(session.index.call_graph());
            println!("{}", serde_json::to_string_pretty(&manifest)?);
            Ok(())
        }
        NavQuery::Functions { repo, format } => {
            let recs = prism::navigation::inventory::functions_inventory(repo)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&recs)?);
            } else {
                for r in &recs {
                    println!(
                        "{}:{}-{} {} [{}]",
                        r.file,
                        r.start_line,
                        r.end_line,
                        r.name.as_deref().unwrap_or("<anon>"),
                        r.kind
                    );
                }
            }
            Ok(())
        }
    }
}

fn build_session(
    repo: &Path,
    no_cache: bool,
    cache_dir: Option<&Path>,
) -> anyhow::Result<prism::navigation::NavigationSession> {
    let repo = std::sync::Arc::new(prism::repo_loader::load_repo(repo)?);
    let index = if no_cache {
        prism::navigation::NavigationIndex::build(&repo)
    } else {
        match cache_dir {
            Some(base) => prism::navigation::NavigationIndex::build_cached_under(&repo, base),
            None => prism::navigation::NavigationIndex::build_cached(&repo),
        }
    };
    let index = std::sync::Arc::new(index);
    Ok(prism::navigation::NavigationSession { repo, index })
}

fn run_review(cli: &ReviewArgs) -> Result<()> {
    if cli.list_algorithms {
        println!("Available algorithms:\n");
        println!("  Paper (arXiv:2505.17928):");
        println!("    originaldiff     Raw diff lines only");
        println!("    parentfunction   Entire enclosing function");
        println!("    leftflow         Backward data-flow from L-values (default)");
        println!("    fullflow         LeftFlow + R-value forward tracing");
        println!();
        println!("  Established taxonomy:");
        println!("    thin             Data deps only, no control flow context");
        println!("    barrier          Interprocedural with depth limits (--barrier-depth, --barrier-symbols)");
        println!("    chop             Source-to-sink paths (--chop-source, --chop-sink)");
        println!("    taint            Forward taint propagation (--taint-source)");
        println!("    relevant         Backward + alternate branch paths");
        println!("    conditioned      Slice under assumption (--condition)");
        println!("    delta            Behavioral diff between versions (--old-repo)");
        println!();
        println!("  Theoretical extensions:");
        println!("    spiral           Adaptive-depth concentric rings (--spiral-max-ring)");
        println!("    circular         Data flow cycle detection");
        println!("    quantum          Concurrent state enumeration (--quantum-var)");
        println!("    horizontal       Peer pattern consistency (--peer-pattern)");
        println!("    vertical         End-to-end feature path (--layers)");
        println!("    angle            Cross-cutting concern trace (--concern)");
        println!("    3d               Temporal-structural risk (--temporal-days)");
        println!();
        println!("  Novel extensions:");
        println!(
            "    absence          Missing counterparts (open without close, lock without unlock)"
        );
        println!("    resonance        Files that usually co-change but aren't in this diff");
        println!("    symmetry         Broken symmetry (serialize changed, deserialize not)");
        println!("    gradient         Continuous relevance scoring with distance decay");
        println!("    provenance       Trace data origin (user input, config, database, constant)");
        println!("    phantom          Recently deleted code the diff may depend on");
        println!("    membrane         Module boundary: who calls this API and will they break");
        println!(
            "    echo             Ripple effect: downstream callers missing new error handling"
        );
        println!(
            "    contract         Implicit behavioral contract extraction and violation detection"
        );
        println!(
            "    peer             Peer-signature guard divergence (C/C++ sibling NULL-guard clusters)"
        );
        println!(
            "    callback         Callback-dispatcher resolution (function-pointer registrations → invocation sites)"
        );
        println!(
            "    primitive        Security-primitive fingerprints (hash truncation, weak-hash-for-identity, shell-injection, TLS disabled, hardcoded secrets)"
        );
        return Ok(());
    }

    // Parse the algorithm list: "review", "all", comma-separated names, or single name
    let algorithms_to_run: Vec<SlicingAlgorithm> = match cli.algorithm.to_lowercase().as_str() {
        "review" => SlicingAlgorithm::review_suite(),
        "all" => SlicingAlgorithm::all(),
        multi if multi.contains(',') => {
            let mut algos = Vec::new();
            for part in multi.split(',') {
                let part = part.trim();
                let algo = SlicingAlgorithm::from_str(part).context(format!(
                    "Unknown algorithm: {}. Use --list-algorithms to see options.",
                    part
                ))?;
                algos.push(algo);
            }
            algos
        }
        single => {
            let algo = SlicingAlgorithm::from_str(single).context(format!(
                "Unknown algorithm: {}. Use --list-algorithms to see options.",
                cli.algorithm
            ))?;
            vec![algo]
        }
    };

    let multi_run = algorithms_to_run.len() > 1;

    let config = SliceConfig {
        algorithm: algorithms_to_run[0],
        max_branch_lines: cli.max_branch_lines,
        include_returns: !cli.no_returns,
        trace_callees: !cli.no_trace_callees,
        scoped_cpg: cli.scoped_cpg,
        diagram_node_cap: cli.diagram_node_cap,
        strict_diagrams: cli.strict_diagrams,
    };

    let repo = cli.repo.as_ref().context("--repo is required")?;
    let diff_path = cli.diff.as_ref().context("--diff is required")?;

    // Read diff input
    let diff_text =
        fs::read_to_string(diff_path).context(format!("Failed to read diff: {:?}", diff_path))?;

    let mut diff_input = if diff_text.trim_start().starts_with('{') {
        DiffInput::from_json(&diff_text)?
    } else {
        DiffInput::parse_unified_diff(&diff_text)
    };

    // Apply --files filter early so algorithms only see the selected files
    let file_filter: Option<HashSet<String>> = cli
        .files
        .as_ref()
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect());
    diff_input.filter_files(file_filter.as_ref());

    // Parse all referenced source files
    let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    let mut sources: BTreeMap<String, String> = BTreeMap::new();

    for diff_info in &diff_input.files {
        let file_path = repo.join(&diff_info.file_path);
        let language = match Language::from_path(&diff_info.file_path) {
            Some(l) => l,
            None => {
                eprintln!(
                    "Warning: unsupported language for {}, skipping",
                    diff_info.file_path
                );
                continue;
            }
        };

        let source = fs::read_to_string(&file_path)
            .context(format!("Failed to read source: {:?}", file_path))?;

        let parsed = ParsedFile::parse(&diff_info.file_path, &source, language)?;
        sources.insert(diff_info.file_path.clone(), source);
        files.insert(diff_info.file_path.clone(), parsed);
    }

    // Load type database if compile_commands.json is provided
    let type_db: Option<TypeDatabase> = if let Some(cc_path) = &cli.compile_commands {
        let diff_files: Vec<&str> = diff_input
            .files
            .iter()
            .map(|f| f.file_path.as_str())
            .collect();
        match TypeDatabase::from_compile_commands(cc_path, Some(&diff_files)) {
            Ok(db) => {
                eprintln!(
                    "Type enrichment: {} records, {} typedefs from {}",
                    db.records.len(),
                    db.typedefs.len(),
                    cc_path.display()
                );
                Some(db)
            }
            Err(e) => {
                eprintln!("Warning: failed to load type database: {}", e);
                None
            }
        }
    } else {
        // Auto-enable tree-sitter fallback for C/C++ files
        let has_c_cpp = files.values().any(|pf| {
            matches!(
                pf.language,
                prism::languages::Language::C | prism::languages::Language::Cpp
            )
        });
        if has_c_cpp {
            let db = TypeDatabase::from_parsed_files(&files);
            if !db.records.is_empty() || !db.typedefs.is_empty() {
                eprintln!(
                    "Type enrichment (tree-sitter fallback): {} records, {} typedefs",
                    db.records.len(),
                    db.typedefs.len()
                );
                Some(db)
            } else {
                None
            }
        } else {
            None
        }
    };

    // Check parse quality for all files and collect warnings + structured data.
    let (parse_warnings, parse_quality) = algorithms::check_parse_quality(&files);
    let scope_graph_inputs = prism::repo_loader::scope_graph_build_inputs(repo, &files);

    // Build CPG once — shared across all algorithm runs.
    // With --cache-dir, attempt to load from cache first.
    // With --scoped-cpg, only process diff-changed files + direct callers/callees.
    let mut ctx = {
        let use_cache = cli.cache_dir.is_some() && !cli.no_cache && !cli.scoped_cpg;
        let file_hashes = if use_cache {
            Some(cpg_cache::compute_file_hashes(&sources))
        } else {
            None
        };
        let topology_key = file_hashes.as_ref().map(|hashes| {
            let mut key =
                cpg_cache::compute_topology_key(hashes, &scope_graph_inputs.manifest_hashes);
            if let Some(type_db) = type_db.as_ref() {
                key.insert(
                    "type_db:fingerprint".to_string(),
                    type_db.cache_fingerprint(),
                );
            }
            key
        });

        // Try loading from cache.
        // Pass type_db availability so cache can detect virtual dispatch edge mismatches.
        let has_type_db = type_db.is_some();
        let cache_result = if use_cache {
            let cache_dir = cli.cache_dir.as_ref().unwrap();
            let hashes = file_hashes.as_ref().unwrap();
            cpg_cache::load_cache_with_topology(
                hashes,
                topology_key.as_ref().unwrap(),
                has_type_db,
                cache_dir,
            )
        } else {
            CacheResult::Miss
        };

        match cache_result {
            CacheResult::Hit(cpg) => {
                let hashes = file_hashes.as_ref().unwrap();
                eprintln!("CPG loaded from cache ({} files)", hashes.len());
                CpgContext::build_with_cached_cpg(&files, cpg, type_db.as_ref())
            }
            CacheResult::PartialHit {
                cached_call_graph,
                cached_dfg,
                changed_files,
            } => {
                eprintln!(
                    "CPG cache partial hit: {} of {} files changed, rebuilding incrementally",
                    changed_files.len(),
                    file_hashes.as_ref().map_or(0, |h| h.len())
                );
                let cpg = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
                    cached_call_graph,
                    cached_dfg,
                    &changed_files,
                    &files,
                    type_db.clone(),
                    Some(&scope_graph_inputs),
                );
                let ctx = CpgContext::build_with_cached_cpg(&files, cpg, type_db.as_ref());

                // Save updated cache.
                if let (Some(cache_dir), Some(hashes)) = (&cli.cache_dir, &file_hashes) {
                    if let Err(e) = cpg_cache::save_cache_with_topology(
                        &ctx.cpg,
                        hashes,
                        topology_key.as_ref().unwrap(),
                        has_type_db,
                        cache_dir,
                    ) {
                        eprintln!("Warning: failed to write CPG cache: {}", e);
                    } else {
                        eprintln!("CPG cache updated to {}", cache_dir.display());
                    }
                }
                ctx
            }
            CacheResult::Miss => {
                let ctx = if cli.scoped_cpg {
                    CpgContext::build_scoped(&files, &diff_input, type_db.as_ref())
                } else {
                    CpgContext::build_with_scope_graph_inputs(
                        &files,
                        type_db.as_ref(),
                        Some(&scope_graph_inputs),
                    )
                };

                // Save cache after a full build (not for scoped builds).
                if let (Some(cache_dir), Some(hashes)) = (&cli.cache_dir, &file_hashes) {
                    if let Err(e) = cpg_cache::save_cache_with_topology(
                        &ctx.cpg,
                        hashes,
                        topology_key.as_ref().unwrap(),
                        has_type_db,
                        cache_dir,
                    ) {
                        eprintln!("Warning: failed to write CPG cache: {}", e);
                    } else {
                        eprintln!("CPG cache written to {}", cache_dir.display());
                    }
                }
                ctx
            }
        }
    };

    // Store target language versions in the registry (informational in Phase 1).
    if let Some(ref v) = cli.python_version {
        if let Some(lv) = LanguageVersion::parse(v) {
            ctx.types.set_target_version(Language::Python, lv);
        }
    }
    if let Some(ref v) = cli.go_version {
        if let Some(lv) = LanguageVersion::parse(v) {
            ctx.types.set_target_version(Language::Go, lv);
        }
    }
    if let Some(ref v) = cli.node_version {
        if let Some(lv) = LanguageVersion::parse(v) {
            ctx.types.set_target_version(Language::JavaScript, lv);
        }
    }
    if let Some(ref v) = cli.typescript_version {
        if let Some(lv) = LanguageVersion::parse(v) {
            ctx.types.set_target_version(Language::TypeScript, lv);
        }
    }
    if let Some(ref v) = cli.java_version {
        if let Some(lv) = LanguageVersion::parse(v) {
            ctx.types.set_target_version(Language::Java, lv);
        }
    }
    if let Some(ref v) = cli.rust_version {
        if let Some(lv) = LanguageVersion::parse(v) {
            ctx.types.set_target_version(Language::Rust, lv);
        }
    }

    // --format callers: emit raw call graph without running any algorithm.
    if cli.format == "callers" {
        let callers_output = output::to_callers_output(&ctx, &diff_input, cli.caller_depth);
        println!("{}", serde_json::to_string_pretty(&callers_output)?);
        return Ok(());
    }

    if multi_run {
        // --- Multi-algorithm run ---
        let mut results = Vec::new();
        let mut all_errors: Vec<AlgorithmError> = Vec::new();

        for &algo in &algorithms_to_run {
            let algo_config = SliceConfig {
                algorithm: algo,
                max_branch_lines: cli.max_branch_lines,
                include_returns: !cli.no_returns,
                trace_callees: !cli.no_trace_callees,
                scoped_cpg: cli.scoped_cpg,
                diagram_node_cap: cli.diagram_node_cap,
                strict_diagrams: cli.strict_diagrams,
            };
            match run_algorithm(algo, &ctx, &diff_input, &algo_config, &cli, repo) {
                Ok(r) => results.push(r),
                Err(e) => all_errors.push(AlgorithmError {
                    algorithm: algo.name().to_string(),
                    error: e.to_string(),
                }),
            }
        }

        let algorithms_run: Vec<String> = algorithms_to_run
            .iter()
            .map(|a| a.name().to_string())
            .collect();
        let mut all_findings: Vec<_> = results.iter().flat_map(|r| r.findings.clone()).collect();
        annotate_finding_parse_quality(&mut all_findings, &files);

        match cli.format.as_str() {
            "review" => {
                // Compact review-only path (P1 Change 3): severity floor +
                // block retention + dropped slice_lines/diff_lines. Distinct
                // from "json" below, which keeps the old byte-pinned shape.
                let min_rank = output::severity_rank(&cli.review_min_severity);
                let all_diagram_warnings: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.iter().cloned())
                    .collect();
                let review_results: Vec<_> = results
                    .iter()
                    .map(|r| {
                        output::to_compact_review_output(
                            r,
                            &sources,
                            min_rank,
                            cli.review_full_slices,
                        )
                    })
                    .collect();
                let filtered_all_findings: Vec<_> = all_findings
                    .iter()
                    .filter(|f| output::severity_rank(&f.severity) >= min_rank)
                    .cloned()
                    .collect();
                let out = output::CompactMultiReviewOutput {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results: review_results,
                    all_findings: filtered_all_findings,
                    errors: all_errors,
                    warnings: parse_warnings,
                    parse_quality: parse_quality.clone(),
                    diagram_warnings: all_diagram_warnings.clone(),
                };
                println!("{}", serde_json::to_string_pretty(&out)?);
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "json" => {
                // json and review produce the same ReviewOutput structure so that
                // slice_text (rendered source code) is always present in structured output.
                let all_diagram_warnings: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.iter().cloned())
                    .collect();
                let review_results: Vec<_> = results
                    .iter()
                    .map(|r| output::to_review_output(r, &sources))
                    .collect();
                let out = output::MultiReviewOutput {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results: review_results,
                    all_findings,
                    errors: all_errors,
                    warnings: parse_warnings,
                    parse_quality: parse_quality.clone(),
                    diagram_warnings: all_diagram_warnings.clone(),
                };
                println!("{}", serde_json::to_string_pretty(&out)?);
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "mermaid" => {
                let multi_result = MultiSliceResult {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results,
                    findings: all_findings,
                    errors: all_errors,
                    warnings: parse_warnings,
                    parse_quality: parse_quality.clone(),
                    diagram_warnings: vec![],
                };
                let report = output::format_mermaid_report(&multi_result);
                println!("{}", report);
                let warnings = multi_result.aggregate_diagram_warnings();
                emit_warnings_to_stderr(&warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            _ => {
                for w in &parse_warnings {
                    eprintln!("WARNING: {}", w);
                }
                for result in &results {
                    println!("=== {} ===", result.algorithm.name());
                    print!("{}", output::format_slice_result(&result.blocks, &sources));
                }
                let all_diagram_warnings: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.iter().cloned())
                    .collect();
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
        }
    } else {
        // --- Single-algorithm run ---
        let algorithm = algorithms_to_run[0];
        let mut result = run_algorithm(algorithm, &ctx, &diff_input, &config, &cli, repo)?;
        result.warnings = parse_warnings;
        annotate_finding_parse_quality(&mut result.findings, &files);

        match cli.format.as_str() {
            "review" => {
                // Compact review-only path (P1 Change 3): severity floor +
                // block retention + dropped slice_lines/diff_lines.
                let min_rank = output::severity_rank(&cli.review_min_severity);
                let review = output::to_compact_review_output(
                    &result,
                    &sources,
                    min_rank,
                    cli.review_full_slices,
                );
                println!("{}", serde_json::to_string_pretty(&review)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "json" => {
                // json retains the old ReviewOutput structure byte-for-byte
                // (compatibility tests pin this shape — see nav_compat_test.rs).
                let review = output::to_review_output(&result, &sources);
                println!("{}", serde_json::to_string_pretty(&review)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "paper" => {
                let paper_output = output::to_paper_format(&result.blocks);
                println!("{}", serde_json::to_string_pretty(&paper_output)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "mermaid" => {
                let algorithms_run = vec![result.algorithm.name().to_string()];
                let multi_result = MultiSliceResult {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results: vec![result],
                    findings: vec![],
                    errors: vec![],
                    warnings: vec![],
                    parse_quality: BTreeMap::new(),
                    diagram_warnings: vec![],
                };
                let report = output::format_mermaid_report(&multi_result);
                println!("{}", report);
                let warnings = multi_result.aggregate_diagram_warnings();
                emit_warnings_to_stderr(&warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            _ => {
                for w in &result.warnings {
                    eprintln!("WARNING: {}", w);
                }
                print!("{}", output::format_slice_result(&result.blocks, &sources));
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
        }
    }

    Ok(())
}

/// Annotate findings with the parse quality grade of their source file.
fn annotate_finding_parse_quality(
    findings: &mut [SliceFinding],
    files: &BTreeMap<String, ParsedFile>,
) {
    for finding in findings.iter_mut() {
        if let Some(pf) = files.get(&finding.file) {
            let rate = pf.error_rate();
            if rate > 0.01 {
                let q = if rate > 0.3 {
                    "unparseable"
                } else if rate > 0.1 {
                    "poor"
                } else {
                    "degraded"
                };
                finding.parse_quality = Some(q.to_string());
            }
        }
    }
}

/// Run a single slicing algorithm with all CLI-configured parameters.
fn run_algorithm(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext,
    diff_input: &DiffInput,
    config: &SliceConfig,
    cli: &ReviewArgs,
    repo: &std::path::Path,
) -> Result<prism::slice::SliceResult> {
    let mut result = match algorithm {
        SlicingAlgorithm::BarrierSlice => {
            let barrier_config = prism::algorithms::barrier_slice::BarrierConfig {
                max_depth: cli.barrier_depth,
                barrier_symbols: cli
                    .barrier_symbols
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().to_string())
                    .collect(),
                barrier_modules: Vec::new(),
            };
            prism::algorithms::barrier_slice::slice(ctx, diff_input, config, &barrier_config)
        }
        SlicingAlgorithm::Chop => {
            let source = cli
                .chop_source
                .as_ref()
                .context("--chop-source required for chop algorithm")?;
            let sink = cli
                .chop_sink
                .as_ref()
                .context("--chop-sink required for chop algorithm")?;
            let (sf, sl) = parse_file_line(source)?;
            let (kf, kl) = parse_file_line(sink)?;
            prism::algorithms::chop::slice(
                ctx,
                &prism::algorithms::chop::ChopConfig {
                    source_file: sf,
                    source_line: sl,
                    sink_file: kf,
                    sink_line: kl,
                },
            )
        }
        SlicingAlgorithm::Taint => {
            let taint_config = prism::algorithms::taint::TaintConfig {
                sources: cli
                    .taint_source
                    .iter()
                    .filter_map(|s| parse_file_line(s).ok())
                    .collect(),
                taint_from_diff: cli.taint_source.is_empty(),
                extra_sinks: Vec::new(),
            };
            prism::algorithms::taint::slice(ctx, diff_input, &taint_config)
        }
        SlicingAlgorithm::ConditionedSlice => {
            let cond_str = cli
                .condition
                .as_ref()
                .context("--condition required for conditioned algorithm")?;
            let condition = prism::algorithms::conditioned_slice::Condition::parse(cond_str)
                .context(format!("Failed to parse condition: {}", cond_str))?;
            prism::algorithms::conditioned_slice::slice(&ctx, diff_input, config, &condition)
        }
        SlicingAlgorithm::DeltaSlice => {
            let old_repo = cli
                .old_repo
                .as_ref()
                .context("--old-repo required for delta algorithm")?;
            prism::algorithms::delta_slice::slice(ctx, diff_input, old_repo)
        }
        SlicingAlgorithm::SpiralSlice => {
            let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
                max_ring: cli.spiral_max_ring,
                auto_stop_threshold: 0.05,
            };
            prism::algorithms::spiral_slice::slice(ctx, diff_input, config, &spiral_config)
        }
        SlicingAlgorithm::QuantumSlice => prism::algorithms::quantum_slice::slice(
            ctx.files,
            diff_input,
            cli.quantum_var.as_deref(),
        ),
        SlicingAlgorithm::HorizontalSlice => {
            let pattern = match cli.peer_pattern.as_deref() {
                Some(p) if p.starts_with("decorator:") => {
                    prism::algorithms::horizontal_slice::PeerPattern::Decorator(
                        p.strip_prefix("decorator:").unwrap().to_string(),
                    )
                }
                Some(p) if p.starts_with("name:") => {
                    prism::algorithms::horizontal_slice::PeerPattern::NamePattern(
                        p.strip_prefix("name:").unwrap().to_string(),
                    )
                }
                Some(p) if p.starts_with("class:") => {
                    prism::algorithms::horizontal_slice::PeerPattern::ParentClass(
                        p.strip_prefix("class:").unwrap().to_string(),
                    )
                }
                _ => prism::algorithms::horizontal_slice::PeerPattern::Auto,
            };
            prism::algorithms::horizontal_slice::slice(ctx.files, diff_input, &pattern)
        }
        SlicingAlgorithm::VerticalSlice => {
            let vertical_config = prism::algorithms::vertical_slice::VerticalConfig {
                layers: cli
                    .layers
                    .as_deref()
                    .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
            };
            prism::algorithms::vertical_slice::slice(ctx, diff_input, &vertical_config)
        }
        SlicingAlgorithm::AngleSlice => {
            let concern = cli
                .concern
                .as_deref()
                .map(prism::algorithms::angle_slice::Concern::from_str)
                .unwrap_or(prism::algorithms::angle_slice::Concern::ErrorHandling);
            prism::algorithms::angle_slice::slice(ctx.files, diff_input, &concern)
        }
        SlicingAlgorithm::ThreeDSlice => {
            let threed_config = prism::algorithms::threed_slice::ThreeDConfig {
                temporal_days: cli.temporal_days,
                git_dir: repo.to_string_lossy().to_string(),
            };
            prism::algorithms::threed_slice::slice(ctx, diff_input, &threed_config)
        }
        SlicingAlgorithm::ResonanceSlice => {
            let resonance_config = prism::algorithms::resonance_slice::ResonanceConfig {
                git_dir: repo.to_string_lossy().to_string(),
                days: cli.temporal_days,
                ..Default::default()
            };
            prism::algorithms::resonance_slice::slice(ctx.files, diff_input, &resonance_config)
        }
        SlicingAlgorithm::PhantomSlice => {
            let phantom_config = prism::algorithms::phantom_slice::PhantomConfig {
                git_dir: repo.to_string_lossy().to_string(),
                ..Default::default()
            };
            prism::algorithms::phantom_slice::slice(ctx.files, diff_input, &phantom_config)
        }
        SlicingAlgorithm::ContractSlice => {
            if let Some(old_repo) = &cli.old_repo {
                prism::algorithms::contract_slice::slice_delta(ctx.files, diff_input, old_repo)
            } else {
                prism::algorithms::contract_slice::slice(ctx.files, diff_input)
            }
        }
        // Fallback: use run_slicing_inner (not run_slicing) so that the
        // finalize_diagrams call below is the single owner.  run_slicing
        // would finalize and then the call below would finalize again,
        // duplicating all diagram warnings.
        _ => algorithms::run_slicing_inner(ctx, diff_input, config),
    }?;
    prism::algorithms::finalize_diagrams(&mut result, config.diagram_node_cap);
    Ok(result)
}

fn parse_file_line(s: &str) -> Result<(String, usize)> {
    let parts: Vec<&str> = s.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Expected file:line format, got: {}", s);
    }
    let line: usize = parts[0]
        .parse()
        .context(format!("Invalid line number: {}", parts[0]))?;
    Ok((parts[1].to_string(), line))
}

fn emit_warnings_to_stderr(warnings: &[prism::slice::DiagramWarning]) {
    use std::io::Write;
    let mut err = std::io::stderr().lock();
    for w in warnings {
        let title = w.graph_title.as_deref().unwrap_or("(no title)");
        let _ = writeln!(
            err,
            "prism: diagram warning: {}/{} - {:?}: {}",
            w.algorithm, title, w.kind, w.detail
        );
    }
}

fn determine_exit_code(strict: bool, warnings: &[prism::slice::DiagramWarning]) -> i32 {
    if !strict {
        return 0;
    }
    if warnings.iter().any(|w| w.kind.is_bug()) {
        return 2;
    }
    0
}

#[cfg(test)]
mod cli_parse_tests {
    use super::*;

    #[test]
    fn parse_diagram_cap_rejects_too_small() {
        assert!(parse_diagram_cap("0").is_err(), "0 must be rejected");
        assert!(parse_diagram_cap("1").is_err(), "1 must be rejected");
        assert!(parse_diagram_cap("2").is_err(), "2 must be rejected");
        assert!(parse_diagram_cap("3").is_err(), "3 must be rejected");
        assert!(parse_diagram_cap("4").is_ok(), "4 must be accepted");
        assert!(parse_diagram_cap("100").is_ok(), "100 must be accepted");
    }

    #[test]
    fn parse_diagram_cap_rejects_non_integer() {
        assert!(parse_diagram_cap("abc").is_err());
        assert!(parse_diagram_cap("-1").is_err());
        assert!(parse_diagram_cap("").is_err());
    }

    /// Architecture pin: `run_slicing_inner` must be a public symbol in
    /// `prism::algorithms`.  The `run_algorithm` fallback path calls it so that
    /// the single `finalize_diagrams` call at the end of `run_algorithm` is the
    /// only one — if the fallback used `run_slicing` (which finalizes), then the
    /// trailing `finalize_diagrams` call would be a second invocation, duplicating
    /// all diagram warnings in JSON output and `## Diagnostics` sections.
    ///
    /// This test does not call the function at runtime; it merely references the
    /// path so that removing the symbol breaks compilation.
    #[test]
    fn run_slicing_inner_is_accessible_for_no_double_finalize_seam() {
        // Compile-time pin: the symbol must be reachable.
        let _ = prism::algorithms::run_slicing_inner
            as fn(
                &prism::cpg::CpgContext,
                &prism::diff::DiffInput,
                &prism::slice::SliceConfig,
            ) -> anyhow::Result<prism::slice::SliceResult>;
    }
}

#[cfg(test)]
mod exit_tests {
    use super::*;
    use prism::slice::{DiagramWarning, DiagramWarningKind};

    fn warn(kind: DiagramWarningKind) -> DiagramWarning {
        DiagramWarning {
            algorithm: "Taint".to_string(),
            graph_title: None,
            kind,
            detail: "x".to_string(),
        }
    }

    #[test]
    fn determine_exit_code_strict_with_bug_warning() {
        let warns = vec![warn(DiagramWarningKind::DanglingEdge)];
        assert_eq!(determine_exit_code(true, &warns), 2);
    }

    #[test]
    fn determine_exit_code_strict_with_only_informational() {
        let warns = vec![warn(DiagramWarningKind::NodeCapExceeded)];
        assert_eq!(determine_exit_code(true, &warns), 0);
    }

    #[test]
    fn determine_exit_code_strict_off() {
        let warns = vec![warn(DiagramWarningKind::DanglingEdge)];
        assert_eq!(determine_exit_code(false, &warns), 0);
    }

    #[test]
    fn determine_exit_code_strict_no_warnings() {
        let warns: Vec<DiagramWarning> = vec![];
        assert_eq!(determine_exit_code(true, &warns), 0);
    }
}
