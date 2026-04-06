use anyhow::{Context, Result};
use clap::Parser;
use prism::algorithms;
use prism::ast::ParsedFile;
use prism::cpg::CpgContext;
use prism::cpg_cache;
use prism::diff::DiffInput;
use prism::languages::Language;
use prism::output;
use prism::slice::{AlgorithmError, MultiSliceResult, SliceConfig, SliceFinding, SlicingAlgorithm};
use prism::type_db::TypeDatabase;
use prism::type_provider::LanguageVersion;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "slicing",
    about = "Code slicing for defect-focused automated code review (arXiv:2505.17928)"
)]
struct Cli {
    /// Path to the repository root
    #[arg(short, long, required_unless_present = "list_algorithms")]
    repo: Option<PathBuf>,

    /// Slicing algorithm (see --list-algorithms for all options)
    #[arg(short, long, default_value = "leftflow")]
    algorithm: String,

    /// Diff input: path to a unified diff file, or a JSON diff spec
    #[arg(short, long, required_unless_present = "list_algorithms")]
    diff: Option<PathBuf>,

    /// Output format: text, json, paper
    #[arg(short, long, default_value = "text")]
    format: String,

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

fn main() -> Result<()> {
    let cli = Cli::parse();

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

        // Try loading from cache.
        let cached_cpg = if use_cache {
            let cache_dir = cli.cache_dir.as_ref().unwrap();
            let hashes = file_hashes.as_ref().unwrap();
            match cpg_cache::load_cache(hashes, cache_dir) {
                Some(cpg) => {
                    eprintln!("CPG loaded from cache ({} files)", hashes.len());
                    Some(cpg)
                }
                None => None,
            }
        } else {
            None
        };

        if let Some(cpg) = cached_cpg {
            CpgContext::build_with_cached_cpg(&files, cpg, type_db.as_ref())
        } else {
            let ctx = if cli.scoped_cpg {
                CpgContext::build_scoped(&files, &diff_input, type_db.as_ref())
            } else {
                CpgContext::build(&files, type_db.as_ref())
            };

            // Save cache after a full build (not for scoped builds).
            if let (Some(cache_dir), Some(hashes)) = (&cli.cache_dir, &file_hashes) {
                if let Err(e) = cpg_cache::save_cache(&ctx.cpg, hashes, cache_dir) {
                    eprintln!("Warning: failed to write CPG cache: {}", e);
                } else {
                    eprintln!("CPG cache written to {}", cache_dir.display());
                }
            }

            ctx
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
                };
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
            "json" => {
                let multi = MultiSliceResult {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results,
                    findings: all_findings,
                    errors: all_errors,
                    warnings: parse_warnings,
                    parse_quality,
                };
                println!("{}", serde_json::to_string_pretty(&multi)?);
            }
            _ => {
                for w in &parse_warnings {
                    eprintln!("WARNING: {}", w);
                }
                for result in &results {
                    println!("=== {} ===", result.algorithm.name());
                    print!("{}", output::format_slice_result(&result.blocks, &sources));
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
            "json" => {
                println!("{}", result.to_json()?);
            }
            "paper" => {
                let paper_output = output::to_paper_format(&result.blocks);
                println!("{}", serde_json::to_string_pretty(&paper_output)?);
            }
            "review" => {
                let review = output::to_review_output(&result, &sources);
                println!("{}", serde_json::to_string_pretty(&review)?);
            }
            _ => {
                for w in &result.warnings {
                    eprintln!("WARNING: {}", w);
                }
                print!("{}", output::format_slice_result(&result.blocks, &sources));
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
    cli: &Cli,
    repo: &std::path::Path,
) -> Result<prism::slice::SliceResult> {
    match algorithm {
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
        _ => algorithms::run_slicing(ctx, diff_input, config),
    }
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
