//! Clap CLI grammar: the `Cli` derive tree.
//!
//! Moved here from `src/main.rs` (spec §2.5.8 / §6) as a pure move — bodies
//! unchanged, visibility raised to `pub` only where `main.rs` (a separate,
//! downstream binary crate) needs it — so that `tests/cli/readme_test.rs`
//! can call [`Cli::try_parse_from`] to parse (never execute) every `prism
//! ...` invocation quoted in README.md. `prism --help`/`--version` are
//! byte-identical before and after this move (`scripts/phase0-byte-control.sh`
//! plus a direct `--help`/`--version` `cmp`).

use crate::finding_confidence::{MinConfidence, ResolutionMode};
use clap::Parser;
use std::path::PathBuf;

/// Validate `--diagram-node-cap`: must be >= 4.
///
/// 4 is the smallest cap that meaningfully fits head + ghost + tail with at
/// least one node on each side of the elision point.  Values below 4 cause
/// `truncate_to_cap`'s arithmetic to produce more nodes than the cap allows
/// (the internal clamp handles it defensively, but we surface a clear error
/// at the CLI boundary so users understand why their cap was rejected).
pub fn parse_diagram_cap(s: &str) -> Result<usize, String> {
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
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub review: ReviewArgs,
}

impl Cli {
    /// Reject an explicitly requested confidence floor on traversal-only
    /// formats before repository or diff loading begins.
    pub fn validate_min_confidence(
        &self,
        min_confidence_explicit: bool,
    ) -> Result<(), clap::Error> {
        if self.command.is_none()
            && min_confidence_explicit
            && matches!(
                self.review.format.as_str(),
                "text" | "paper" | "mermaid" | "callers"
            )
        {
            return Err(clap::Error::raw(
                clap::error::ErrorKind::ArgumentConflict,
                format!(
                    "--min-confidence cannot be used with --format {}: that format has no stable finding projection",
                    self.review.format
                ),
            ));
        }
        Ok(())
    }
}

#[derive(clap::Args, Debug)]
pub struct ReviewArgs {
    /// Path to the repository root
    #[arg(short, long, required_unless_present = "list_algorithms")]
    pub repo: Option<PathBuf>,

    /// Slicing algorithm (see --list-algorithms for all options)
    #[arg(short, long, default_value = "leftflow")]
    pub algorithm: String,

    /// Diff input: path to a unified diff file, or a JSON diff spec
    #[arg(short, long, required_unless_present = "list_algorithms")]
    pub diff: Option<PathBuf>,

    /// Output format: text, json, paper, review, callers, mermaid, sarif
    #[arg(
        short,
        long,
        default_value = "text",
        value_parser = ["text", "json", "paper", "review", "callers", "mermaid", "sarif"]
    )]
    pub format: String,

    /// Minimum confidence for finding-bearing output. `nameonly` retains Exact,
    /// NameOnly, and ungraded Unlabeled findings to preserve legacy output.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::finding_confidence::DEFAULT_MIN_CONFIDENCE
    )]
    pub min_confidence: MinConfidence,

    /// Confidence projection: `nominal` reports CPG findings as unlabeled;
    /// `scoped` reports retained evidence labels. `precise` and `auto` are
    /// deferred to roadmap item 3 because they require an external index.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::finding_confidence::DEFAULT_RESOLUTION
    )]
    pub resolution: ResolutionMode,

    /// Maximum number of nodes a single Mermaid diagram may render before truncation.
    /// Must be >= 4 (the minimum that fits head + ghost + tail).
    #[arg(long, default_value_t = 40, value_parser = parse_diagram_cap)]
    pub diagram_node_cap: usize,

    /// Exit non-zero if any bug-class diagram warning is produced.
    #[arg(long, default_value_t = false)]
    pub strict_diagrams: bool,

    /// Maximum branch lines to include fully (default: 5)
    #[arg(long, default_value = "5")]
    pub max_branch_lines: usize,

    /// Don't include return statements in LeftFlow/FullFlow
    #[arg(long)]
    pub no_returns: bool,

    /// Don't trace into called functions (FullFlow only)
    #[arg(long)]
    pub no_trace_callees: bool,

    /// List all available algorithms and exit
    #[arg(long)]
    pub list_algorithms: bool,

    // --- Algorithm-specific flags ---
    /// Barrier slice: max call depth (default: 2)
    #[arg(long, default_value_t = crate::api::DEFAULT_BARRIER_DEPTH)]
    pub barrier_depth: usize,

    /// Barrier slice: comma-separated function names to not trace into
    #[arg(long, default_value = "")]
    pub barrier_symbols: String,

    /// Chop: source location (file:line)
    #[arg(long)]
    pub chop_source: Option<String>,

    /// Chop: sink location (file:line)
    #[arg(long)]
    pub chop_sink: Option<String>,

    /// Taint: explicit source location (file:line), can be repeated
    #[arg(long)]
    pub taint_source: Vec<String>,

    /// Taint: follow singleton-Exact callee return values to caller LHSs.
    #[arg(long, default_value_t = false)]
    pub taint_return_flow: bool,

    /// Conditioned slice: condition predicate (e.g., "x==5", "x!=null")
    #[arg(long)]
    pub condition: Option<String>,

    /// Delta slice: path to old version of the repository
    #[arg(long)]
    pub old_repo: Option<PathBuf>,

    /// Spiral slice: maximum ring level (1-6)
    #[arg(long, default_value_t = crate::api::DEFAULT_SPIRAL_MAX_RING)]
    pub spiral_max_ring: usize,

    /// Quantum slice: target variable name
    #[arg(long)]
    pub quantum_var: Option<String>,

    /// Horizontal slice: peer pattern (e.g., "decorator:@app.route", "name:test_*")
    #[arg(long)]
    pub peer_pattern: Option<String>,

    /// Vertical slice: comma-separated layer names (highest to lowest)
    #[arg(long)]
    pub layers: Option<String>,

    /// Angle slice: concern to trace (error_handling, logging, auth, caching, or custom keywords)
    #[arg(long)]
    pub concern: Option<String>,

    /// 3D slice: how many days back to look in git history
    #[arg(long, default_value_t = crate::api::DEFAULT_TEMPORAL_DAYS)]
    pub temporal_days: usize,

    /// Max caller-graph traversal depth for --format callers (default: 5)
    #[arg(long, default_value = "5")]
    pub caller_depth: usize,

    /// --format review only: minimum finding severity to include (findings
    /// below this floor are dropped, along with any block that then has no
    /// remaining finding on one of its lines). Does not affect --format
    /// json/text/paper.
    #[arg(long, default_value = "warning", value_parser = ["info", "suggestion", "warning", "concern"])]
    pub review_min_severity: String,

    /// --format review only: keep every block regardless of whether it has
    /// a retained finding. This restores block retention ONLY — it does not
    /// lower the severity floor (pair with --review-min-severity info to
    /// also see low-severity findings) and it does not restore slice_lines/
    /// diff_lines in review output; for the full pre-compaction shape use
    /// --format json. Does not affect --format json/text/paper.
    #[arg(long, default_value_t = false)]
    pub review_full_slices: bool,

    /// --format review only: omit diagram payloads (`diagrams` on each
    /// result and each finding, including the top-level `all_findings`
    /// aggregate in multi-algorithm runs) from the output. This is a
    /// payload-size reduction only: `finalize_diagrams` still runs, so
    /// `diagram_warnings` are unaffected and `--strict-diagrams` exit-code
    /// semantics are unchanged. Does not affect --format json/text/paper.
    #[arg(long, default_value_t = false)]
    pub review_no_diagrams: bool,

    /// Only process these files from the diff (comma-separated paths).
    /// If omitted, process all files in the diff.
    #[arg(long)]
    pub files: Option<String>,

    /// Build CPG from only diff-changed files + direct callers/callees.
    /// Reduces construction time for large repos with small diffs.
    #[arg(long)]
    pub scoped_cpg: bool,

    /// Path to compile_commands.json for C/C++ type enrichment.
    /// Enables precise whole-struct detection, typedef resolution,
    /// and virtual dispatch via class hierarchy analysis.
    #[arg(long)]
    pub compile_commands: Option<PathBuf>,

    /// Directory to cache the CPG for faster subsequent runs.
    /// On the first run, the CPG is serialized to this directory.
    /// On subsequent runs, the cache is loaded if all file hashes match.
    ///
    /// Note: the cache covers only the files referenced in the current diff,
    /// not the entire repository. This means it is per-MR: re-running the
    /// same diff is a cache hit, but a different diff touching different
    /// files will miss and trigger a full rebuild.
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,

    /// Ignore any existing cache and force a full CPG rebuild.
    #[arg(long)]
    pub no_cache: bool,

    // --- Target language version flags (stored, informational in Phase 1) ---
    /// Target Python version (e.g., "3.8", "3.11"). Stored for future use.
    #[arg(long)]
    pub python_version: Option<String>,

    /// Target Go version (e.g., "1.21"). Stored for future use.
    #[arg(long)]
    pub go_version: Option<String>,

    /// Target Node.js version (e.g., "18", "20"). Stored for future use.
    #[arg(long)]
    pub node_version: Option<String>,

    /// Target TypeScript version (e.g., "5.0"). Stored for future use.
    #[arg(long)]
    pub typescript_version: Option<String>,

    /// Target Java version (e.g., "17", "21"). Stored for future use.
    #[arg(long)]
    pub java_version: Option<String>,

    /// Target Rust edition/version (e.g., "2021"). Stored for future use.
    #[arg(long)]
    pub rust_version: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct TargetsArgs {
    /// Path to the repository root.
    #[arg(long)]
    pub repo: PathBuf,

    /// Diff input: path to a unified diff file, or a JSON diff spec.
    #[arg(long)]
    pub diff: PathBuf,

    /// Finding-producing algorithms to project.
    #[arg(long, default_value = "echo,absence,contract,provenance,membrane")]
    pub algorithm: String,

    /// Only process these files from the diff (comma-separated paths).
    #[arg(long)]
    pub files: Option<String>,

    /// Path to compile_commands.json for C/C++ type enrichment.
    #[arg(long)]
    pub compile_commands: Option<PathBuf>,

    /// Build CPG from only diff-changed files + direct callers/callees.
    #[arg(long)]
    pub scoped_cpg: bool,

    /// Directory to cache the CPG.
    #[arg(long, conflicts_with = "no_cache")]
    pub cache_dir: Option<PathBuf>,

    /// Ignore any existing cache and force a full CPG rebuild.
    #[arg(long)]
    pub no_cache: bool,

    /// Old repository tree used by contract/delta analysis.
    #[arg(long)]
    pub old_repo: Option<PathBuf>,

    /// Minimum finding severity to retain.
    #[arg(long, default_value = "info", value_parser = ["info", "suggestion", "warning", "concern"])]
    pub min_severity: String,

    /// Minimum evidence tier to retain.
    #[arg(long, default_value = "candidate", value_parser = ["asserted", "candidate"])]
    pub min_tier: String,

    /// Minimum confidence for emitted targets. `nameonly` also retains
    /// ungraded Unlabeled findings so the default preserves legacy output.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::finding_confidence::DEFAULT_MIN_CONFIDENCE
    )]
    pub min_confidence: MinConfidence,

    /// Confidence projection. `precise` and `auto` are deferred to roadmap
    /// item 3 because they require an external index.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::finding_confidence::DEFAULT_RESOLUTION
    )]
    pub resolution: ResolutionMode,

    /// Exit 3 when one or more requested algorithms fail.
    #[arg(long)]
    pub strict: bool,

    /// Write the JSON document to a file instead of stdout.
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Targets contract output format.
    #[arg(long, default_value = "json", value_parser = ["json"])]
    pub format: String,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Whole-repo navigation/architecture queries.
    Nav(NavArgs),
    /// Project findings into the targets contract v1.0.
    Targets(TargetsArgs),
}

#[derive(clap::Args, Debug)]
pub struct NavArgs {
    /// Ignore the whole-repo navigation cache and force a full CPG rebuild.
    #[arg(long, conflicts_with = "cache_dir")]
    pub no_cache: bool,

    /// Directory to use for the whole-repo navigation cache.
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub query: NavQuery,
}

#[derive(clap::Subcommand, Debug)]
pub enum NavQuery {
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
    /// Exact read-only coordinates for a uniquely resolved callable.
    #[command(group(clap::ArgGroup::new("symbol_spans_seed").required(true).args(["symbol", "location"])))]
    SymbolSpans {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long, conflicts_with = "location")]
        symbol: Option<String>,
        #[arg(long, requires = "symbol", conflicts_with = "location")]
        file: Option<String>,
        /// `file:line`
        #[arg(long, conflicts_with_all = ["symbol", "file"])]
        location: Option<String>,
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
    /// Compact deterministic project orientation over one cached nav build.
    Onboard {
        #[arg(long)]
        repo: std::path::PathBuf,
        #[arg(long, default_value = "markdown", value_parser = ["markdown", "json"])]
        format: String,
        /// Create a new report file instead of writing to stdout; refuses overwrite.
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// Whole-repo call-resolution telemetry.
    CallStats {
        #[arg(long)]
        repo: std::path::PathBuf,
        /// Emit deterministic JSONL custody for every raw call site.
        #[arg(long)]
        dump_sites: bool,
    },
    /// Whole-repo DataFlow confidence telemetry.
    DfgStats {
        #[arg(long)]
        repo: std::path::PathBuf,
        /// Emit deterministic JSONL custody for every labeled DataFlow edge.
        #[arg(long)]
        edges: bool,
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
    /// Tier-2 forward taint reachability from source seeds, optionally to sink
    /// seeds. Omit `--sink` for frontier mode (no verdict, just the tainted
    /// frontier); pass one or more `--sink` for witness mode (per-sink
    /// Reached/NotReached/BoundaryExited/Sanitized verdicts + witness graph).
    /// Sanitized means a recognized sanitizer call is proven to sit ON the
    /// witness chain (not just present somewhere in the source function).
    TaintReaches {
        #[arg(long)]
        repo: std::path::PathBuf,
        /// `file:line` source seed, repeatable.
        #[arg(long = "source")]
        source: Vec<String>,
        /// `file:line` sink seed, repeatable. Omit for frontier mode.
        #[arg(long = "sink")]
        sink: Vec<String>,
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },
}

#[cfg(test)]
mod tests {
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
}
