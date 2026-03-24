use anyhow::{Context, Result};
use clap::Parser;
use slicing::algorithms;
use slicing::ast::ParsedFile;
use slicing::diff::DiffInput;
use slicing::languages::Language;
use slicing::output;
use slicing::slice::{SliceConfig, SlicingAlgorithm};
use std::collections::BTreeMap;
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
        println!("    absence          Missing counterparts (open without close, lock without unlock)");
        println!("    resonance        Files that usually co-change but aren't in this diff");
        println!("    symmetry         Broken symmetry (serialize changed, deserialize not)");
        println!("    gradient         Continuous relevance scoring with distance decay");
        println!("    provenance       Trace data origin (user input, config, database, constant)");
        println!("    phantom          Recently deleted code the diff may depend on");
        println!("    membrane         Module boundary: who calls this API and will they break");
        println!("    echo             Ripple effect: downstream callers missing new error handling");
        return Ok(());
    }

    let algorithm = SlicingAlgorithm::from_str(&cli.algorithm)
        .context(format!(
            "Unknown algorithm: {}. Use --list-algorithms to see options.",
            cli.algorithm
        ))?;

    let config = SliceConfig {
        algorithm,
        max_branch_lines: cli.max_branch_lines,
        include_returns: !cli.no_returns,
        trace_callees: !cli.no_trace_callees,
    };

    let repo = cli.repo.as_ref().context("--repo is required")?;
    let diff_path = cli.diff.as_ref().context("--diff is required")?;

    // Read diff input
    let diff_text =
        fs::read_to_string(diff_path).context(format!("Failed to read diff: {:?}", diff_path))?;

    let diff_input = if diff_text.trim_start().starts_with('{') {
        DiffInput::from_json(&diff_text)?
    } else {
        DiffInput::parse_unified_diff(&diff_text)
    };

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

    // Run slicing — dispatch to algorithm-specific functions for those that need extra config
    let result = match algorithm {
        SlicingAlgorithm::BarrierSlice => {
            let barrier_config = slicing::algorithms::barrier_slice::BarrierConfig {
                max_depth: cli.barrier_depth,
                barrier_symbols: cli
                    .barrier_symbols
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().to_string())
                    .collect(),
                barrier_modules: Vec::new(),
            };
            slicing::algorithms::barrier_slice::slice(&files, &diff_input, &config, &barrier_config)?
        }
        SlicingAlgorithm::Chop => {
            let source = cli.chop_source.as_ref().context("--chop-source required for chop algorithm")?;
            let sink = cli.chop_sink.as_ref().context("--chop-sink required for chop algorithm")?;
            let (sf, sl) = parse_file_line(source)?;
            let (kf, kl) = parse_file_line(sink)?;
            slicing::algorithms::chop::slice(
                &files,
                &slicing::algorithms::chop::ChopConfig {
                    source_file: sf,
                    source_line: sl,
                    sink_file: kf,
                    sink_line: kl,
                },
            )?
        }
        SlicingAlgorithm::Taint => {
            let taint_config = slicing::algorithms::taint::TaintConfig {
                sources: cli
                    .taint_source
                    .iter()
                    .filter_map(|s| parse_file_line(s).ok())
                    .collect(),
                taint_from_diff: cli.taint_source.is_empty(),
                extra_sinks: Vec::new(),
            };
            slicing::algorithms::taint::slice(&files, &diff_input, &taint_config)?
        }
        SlicingAlgorithm::ConditionedSlice => {
            let cond_str = cli.condition.as_ref().context("--condition required for conditioned algorithm")?;
            let condition = slicing::algorithms::conditioned_slice::Condition::parse(cond_str)
                .context(format!("Failed to parse condition: {}", cond_str))?;
            slicing::algorithms::conditioned_slice::slice(&files, &diff_input, &config, &condition)?
        }
        SlicingAlgorithm::DeltaSlice => {
            let old_repo = cli.old_repo.as_ref().context("--old-repo required for delta algorithm")?;
            slicing::algorithms::delta_slice::slice(&files, &diff_input, old_repo)?
        }
        SlicingAlgorithm::SpiralSlice => {
            let spiral_config = slicing::algorithms::spiral_slice::SpiralConfig {
                max_ring: cli.spiral_max_ring,
                auto_stop_threshold: 0.05,
            };
            slicing::algorithms::spiral_slice::slice(&files, &diff_input, &config, &spiral_config)?
        }
        SlicingAlgorithm::QuantumSlice => {
            slicing::algorithms::quantum_slice::slice(
                &files,
                &diff_input,
                cli.quantum_var.as_deref(),
            )?
        }
        SlicingAlgorithm::HorizontalSlice => {
            let pattern = match cli.peer_pattern.as_deref() {
                Some(p) if p.starts_with("decorator:") => {
                    slicing::algorithms::horizontal_slice::PeerPattern::Decorator(
                        p.strip_prefix("decorator:").unwrap().to_string(),
                    )
                }
                Some(p) if p.starts_with("name:") => {
                    slicing::algorithms::horizontal_slice::PeerPattern::NamePattern(
                        p.strip_prefix("name:").unwrap().to_string(),
                    )
                }
                Some(p) if p.starts_with("class:") => {
                    slicing::algorithms::horizontal_slice::PeerPattern::ParentClass(
                        p.strip_prefix("class:").unwrap().to_string(),
                    )
                }
                _ => slicing::algorithms::horizontal_slice::PeerPattern::Auto,
            };
            slicing::algorithms::horizontal_slice::slice(&files, &diff_input, &pattern)?
        }
        SlicingAlgorithm::VerticalSlice => {
            let vertical_config = slicing::algorithms::vertical_slice::VerticalConfig {
                layers: cli
                    .layers
                    .as_deref()
                    .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
            };
            slicing::algorithms::vertical_slice::slice(&files, &diff_input, &vertical_config)?
        }
        SlicingAlgorithm::AngleSlice => {
            let concern = cli
                .concern
                .as_deref()
                .map(slicing::algorithms::angle_slice::Concern::from_str)
                .unwrap_or(slicing::algorithms::angle_slice::Concern::ErrorHandling);
            slicing::algorithms::angle_slice::slice(&files, &diff_input, &concern)?
        }
        SlicingAlgorithm::ThreeDSlice => {
            let threed_config = slicing::algorithms::threed_slice::ThreeDConfig {
                temporal_days: cli.temporal_days,
                git_dir: repo.to_string_lossy().to_string(),
            };
            slicing::algorithms::threed_slice::slice(&files, &diff_input, &threed_config)?
        }
        SlicingAlgorithm::ResonanceSlice => {
            let resonance_config = slicing::algorithms::resonance_slice::ResonanceConfig {
                git_dir: repo.to_string_lossy().to_string(),
                days: cli.temporal_days,
                ..Default::default()
            };
            slicing::algorithms::resonance_slice::slice(&files, &diff_input, &resonance_config)?
        }
        SlicingAlgorithm::PhantomSlice => {
            let phantom_config = slicing::algorithms::phantom_slice::PhantomConfig {
                git_dir: repo.to_string_lossy().to_string(),
                ..Default::default()
            };
            slicing::algorithms::phantom_slice::slice(&files, &diff_input, &phantom_config)?
        }
        _ => algorithms::run_slicing(&files, &diff_input, &config)?,
    };

    // Output
    match cli.format.as_str() {
        "json" => {
            println!("{}", result.to_json()?);
        }
        "paper" => {
            let paper_output = output::to_paper_format(&result.blocks);
            println!("{}", serde_json::to_string_pretty(&paper_output)?);
        }
        _ => {
            print!("{}", output::format_slice_result(&result.blocks, &sources));
        }
    }

    Ok(())
}

fn parse_file_line(s: &str) -> Result<(String, usize)> {
    let parts: Vec<&str> = s.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Expected file:line format, got: {}", s);
    }
    let line: usize = parts[0].parse().context(format!("Invalid line number: {}", parts[0]))?;
    Ok((parts[1].to_string(), line))
}
