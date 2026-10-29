//! Memory report (measurement only): stage-by-stage process memory for one eager index build,
//! an optional query workload and an optional refresh, plus component counts that attribute the
//! footprint (shared-daemon spec §6.1). Not part of serving; `prism-mcp` runs it instead of the
//! server when `PRISM_MCP_MEM_REPORT=1`.

use super::mem_attribution::{attribute, Attribution};
use super::mem_sample::{ru_maxrss_bytes, sample, MemSample};
use super::mem_tree_alloc::live_bytes;
use super::registry::ToolRegistry;
use super::transport::{serve_session_with_freshness, ReadOutcome, Transport};
use super::{BuildStage, CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider};
use crate::cpg::CpgNode;
use anyhow::Context;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[path = "mem_changed_refresh.rs"]
mod changed;

const INIT: &str = r#"{"jsonrpc":"2.0","id":"prism-mem-init","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"prism-mem-report","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

pub struct MemReportOptions {
    /// A file of `tools/call` requests, one JSON object per line, run after the build.
    pub workload: Option<PathBuf>,
    /// Run one full refresh at the end, for the refresh peak.
    pub refresh: bool,
    /// Preregistered private-mirror edit (non-default memreport-changed-refresh feature).
    pub changed_refresh: Option<PathBuf>,
    /// How long to wait after the workload before the steady-state sample.
    pub settle: Duration,
}

#[derive(serde::Serialize)]
struct Stage {
    name: &'static str,
    elapsed_ms: u64,
    #[serde(flatten)]
    memory: MemSample,
    /// Usable bytes of every live tree-sitter allocation: the retained syntax trees plus prism's
    /// compiled query cache (about 54 KB, fixed per process). `null` when the counting hooks are
    /// not installed (the binary installs them in report mode).
    tree_sitter_heap_bytes: Option<u64>,
}

struct Recorder {
    started: Instant,
    stages: Vec<Stage>,
}

impl Recorder {
    fn start() -> Self {
        let mut recorder = Self {
            started: Instant::now(),
            stages: Vec::new(),
        };
        recorder.mark("start");
        recorder
    }

    fn mark(&mut self, name: &'static str) {
        self.stages.push(Stage {
            name,
            elapsed_ms: self.started.elapsed().as_millis() as u64,
            memory: sample(),
            tree_sitter_heap_bytes: live_bytes(),
        });
    }
}

#[derive(Default, serde::Serialize)]
struct WorkloadStats {
    requests: u64,
    responses: u64,
    /// JSON-RPC errors plus tool results with `isError`. A workload that does not fit the corpus
    /// shows up here, and such a run measures nothing.
    errors: u64,
    response_bytes: u64,
    /// Lowercase hex SHA-256 of the workload file's bytes exactly as this process read them.
    /// Empty for a workload generated in process and not read from a file.
    sha256: String,
}

/// Feeds a handshake and the workload to the serve loop and counts what comes back, keeping no
/// response, so the transport itself adds nothing to the footprint being measured.
struct WorkloadTransport {
    lines: VecDeque<String>,
    handshake_answered: bool,
    stats: WorkloadStats,
}

impl Transport for WorkloadTransport {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        Ok(self.lines.pop_front().map(ReadOutcome::Message))
    }

    fn write_message(&mut self, response: Value) -> anyhow::Result<()> {
        if !self.handshake_answered {
            self.handshake_answered = true;
            anyhow::ensure!(
                response.get("result").is_some(),
                "the in-process handshake failed: {response}"
            );
            return Ok(());
        }
        self.stats.responses += 1;
        self.stats.response_bytes += serde_json::to_vec(&response)?.len() as u64 + 1;
        if response.get("error").is_some() || response["result"]["isError"] == true {
            self.stats.errors += 1;
        }
        Ok(())
    }
}

fn run_workload(provider: &SessionProvider, requests: &[String]) -> anyhow::Result<WorkloadStats> {
    let mut lines = VecDeque::with_capacity(requests.len() + 2);
    lines.push_back(INIT.to_string());
    lines.push_back(INITED.to_string());
    lines.extend(requests.iter().cloned());
    let mut transport = WorkloadTransport {
        lines,
        handshake_answered: false,
        stats: WorkloadStats {
            requests: requests.len() as u64,
            ..WorkloadStats::default()
        },
    };
    serve_session_with_freshness(
        provider.session(),
        Some(provider.freshness()),
        &ToolRegistry::all_v1(),
        &mut transport,
    )?;
    Ok(transport.stats)
}

/// A validated workload file: its requests and the digest of the bytes they were parsed from.
struct Workload {
    requests: Vec<String>,
    sha256: String,
}

/// Read a workload file: every non-empty line is a `tools/call` request with an id. The digest
/// is taken over the same buffer that is parsed, not over a second read.
fn load_workload(path: &Path) -> anyhow::Result<Workload> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading workload {}", path.display()))?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let text = String::from_utf8(bytes)
        .with_context(|| format!("reading workload {}: not UTF-8", path.display()))?;
    let mut requests = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let number = index + 1;
        let value: Value = serde_json::from_str(line).map_err(|error| {
            anyhow::anyhow!(
                "workload {} line {number}: not JSON: {error}",
                path.display()
            )
        })?;
        anyhow::ensure!(
            value["method"] == "tools/call" && !value["id"].is_null(),
            "workload {} line {number}: every line must be a tools/call request with an id",
            path.display()
        );
        requests.push(line.to_string());
    }
    anyhow::ensure!(
        !requests.is_empty(),
        "workload {} has no requests",
        path.display()
    );
    Ok(Workload { requests, sha256 })
}

#[derive(serde::Serialize)]
struct BuildSection {
    package_version: &'static str,
    git_sha: &'static str,
    build_identity: &'static str,
    binary_input_dirty: bool,
    grammar_fingerprint: &'static str,
    cpg_cache_version: u32,
    /// The crate installs no global allocator. Update this if it ever does.
    allocator: &'static str,
}

#[derive(serde::Serialize)]
struct PlatformSection {
    os: &'static str,
    arch: &'static str,
}

#[derive(serde::Serialize)]
struct ConfigSection {
    repo_root: String,
    cache: String,
    /// Echoes the `--refresh-policy` flag. The workload itself is always served by the static,
    /// warn-only runtime, so the policy does not affect workload results.
    refresh_policy: &'static str,
    workload: Option<String>,
    refresh: bool,
    settle_secs: u64,
}

/// Positive evidence of which caches this run actually used, read right after the call-edge index
/// is forced. A run that is supposed to be warm is admissible only when both are `true`.
#[derive(serde::Serialize)]
struct CacheSection {
    /// The CPG came from an exact cache hit; `null` when the index has no cache store (`--no-cache`).
    cpg_hit: Option<bool>,
    /// The resolved call-edge index was loaded from its sidecar (`false`: it was computed, including
    /// when caching is off); `null` only if the index was never forced.
    call_edges_loaded: Option<bool>,
}

#[derive(serde::Serialize)]
struct Report {
    /// Report format version. First field, so the line starts with `{"prism_mem_report":`.
    prism_mem_report: u32,
    build: BuildSection,
    platform: PlatformSection,
    config: ConfigSection,
    /// `MallocSpaceEfficient` and every `PRISM_*` variable set in this process, sorted by name,
    /// except the report-mode switch: settings that can change what is measured.
    env: BTreeMap<String, String>,
    cache: CacheSection,
    /// Worker count of the rayon pool that parses and builds the index (`RAYON_NUM_THREADS` and the
    /// host decide it).
    threads: usize,
    stages: Vec<Stage>,
    ru_maxrss_bytes: Option<u64>,
    attribution: Attribution,
    workload: Option<WorkloadStats>,
    refresh: Option<RefreshSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed_refresh: Option<Value>,
}

fn build_section() -> BuildSection {
    let info = crate::api::build_info();
    BuildSection {
        package_version: info.package_version,
        git_sha: info.git_sha,
        build_identity: info.build_identity,
        binary_input_dirty: info.binary_input_dirty,
        grammar_fingerprint: info.grammar_fingerprint,
        cpg_cache_version: crate::cpg_cache::current_cache_format_version(),
        allocator: "system",
    }
}

fn config_section(cfg: &ServerConfig, options: &MemReportOptions) -> ConfigSection {
    ConfigSection {
        repo_root: cfg.repo_root.display().to_string(),
        cache: match &cfg.cache {
            CacheMode::NoCache => "no-cache".to_string(),
            CacheMode::Default => "default".to_string(),
            CacheMode::Dir(base) => format!("dir:{}", base.display()),
        },
        refresh_policy: match cfg.refresh_policy {
            RefreshPolicy::WarnOnly => "warn-only",
            RefreshPolicy::AutoFull => "auto-full",
            RefreshPolicy::AutoIncremental => "auto-incremental",
        },
        workload: options
            .workload
            .as_ref()
            .map(|path| path.display().to_string()),
        refresh: options.refresh || options.changed_refresh.is_some(),
        settle_secs: options.settle.as_secs(),
    }
}

fn env_section() -> BTreeMap<String, String> {
    prism_settings(std::env::vars_os())
}

/// Allocator mode and `PRISM_*` entries, minus the variable that selects report mode.
fn prism_settings(
    vars: impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
) -> BTreeMap<String, String> {
    vars.filter_map(|(name, value)| {
        let name = name.to_string_lossy().into_owned();
        ((name.starts_with("PRISM_") && name != "PRISM_MCP_MEM_REPORT")
            || name == "MallocSpaceEfficient")
            .then(|| (name, value.to_string_lossy().into_owned()))
    })
    .collect()
}

/// Build the index in observed stages, run the optional workload and refresh, and write one
/// JSON line describing the process's memory at each stage.
pub fn run(
    cfg: &ServerConfig,
    options: &MemReportOptions,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    // Reject a bad workload before paying for the build.
    let workload_file = options.workload.as_deref().map(load_workload).transpose()?;
    anyhow::ensure!(
        options.changed_refresh.is_none() || !options.refresh,
        "choose one refresh scenario"
    );
    let edit = options
        .changed_refresh
        .as_deref()
        .map(|path| changed::Edit::load(&cfg.repo_root, path))
        .transpose()?;

    let mut recorder = Recorder::start();
    let mut provider = SessionProvider::bootstrap_observed(cfg, &mut |stage| {
        recorder.mark(match stage {
            BuildStage::RepoLoaded => "after_load_repo",
            BuildStage::IndexBuilt => "after_index",
        })
    })?;

    // The resolved call-edge index is built by the first caller/callee query. Force it here so
    // its cost is a stage of its own instead of hiding inside the workload.
    let _ = provider.session().index.resolved_call_edges();
    recorder.mark("after_call_edges");
    let cache = CacheSection {
        cpg_hit: provider.session().index.cpg_loaded_from_cache(),
        call_edges_loaded: provider.session().index.call_edges_loaded_from_cache(),
    };

    let workload = match &workload_file {
        Some(file) => {
            let mut stats = run_workload(&provider, &file.requests)?;
            stats.sha256 = file.sha256.clone();
            recorder.mark("after_workload");
            Some(stats)
        }
        None => None,
    };

    std::thread::sleep(options.settle);
    recorder.mark("after_settle");

    let mut changed_refresh = None;
    let refresh = if let Some(edit) = &edit {
        anyhow::ensure!(
            cache.cpg_hit == Some(true) && cache.call_edges_loaded == Some(true),
            "changed-refresh requires true-hit CPG and sidecar bootstrap"
        );
        anyhow::ensure!(
            provider.session().repo.file_hashes.get(&edit.path) == Some(&edit.before_sha256),
            "edit target not in pinned indexed snapshot"
        );
        let (summary, record) = edit.run(&cfg.repo_root, || {
            let summary = provider.refresh()?;
            recorder.mark("after_refresh");
            Ok(summary)
        })?;
        changed_refresh = Some(record);
        Some(summary)
    } else if options.refresh {
        let summary = provider.refresh()?;
        recorder.mark("after_refresh");
        Some(summary)
    } else {
        None
    };

    // Attribution allocates; it runs after the last sample.
    let report = Report {
        prism_mem_report: 3,
        build: build_section(),
        platform: PlatformSection {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        },
        config: config_section(cfg, options),
        env: env_section(),
        cache,
        threads: crate::build_pool::build_pool().current_num_threads(),
        stages: recorder.stages,
        ru_maxrss_bytes: ru_maxrss_bytes(),
        attribution: attribute(provider.session()),
        workload,
        refresh,
        changed_refresh,
    };
    serde_json::to_writer(&mut *out, &report)?;
    out.write_all(b"\n")?;
    Ok(())
}

fn requests_for(first_id: u64, file: &str, name: &str, start_line: usize) -> Vec<String> {
    let seed = json!({"kind": "symbol", "name": name, "file": file});
    [
        ("nav_callers", json!({ "seed": seed.clone() })),
        ("nav_callees", json!({ "seed": seed.clone() })),
        ("nav_ego_graph", json!({ "seed": seed })),
        ("nav_nodes_at", json!({"file": file, "line": start_line})),
    ]
    .into_iter()
    .enumerate()
    .map(|(offset, (tool, arguments))| {
        json!({
            "jsonrpc": "2.0",
            "id": first_id + offset as u64,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments }
        })
        .to_string()
    })
    .collect()
}

/// Write a workload for this repository: up to `functions` functions, evenly spaced through the
/// sorted `(file, name)` index, four queries each. A function is kept only if all four of its
/// queries succeed against this build, so the workload is admissible by construction; when one
/// does not, the next function in order is tried, up to the next evenly spaced position.
pub fn emit_workload(
    cfg: &ServerConfig,
    functions: usize,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    anyhow::ensure!(functions > 0, "ask for at least one function");
    let provider = SessionProvider::bootstrap(cfg)?;
    let index = &provider.session().index;
    let candidates: Vec<(&str, &str, usize)> = index
        .name_index
        .iter()
        .filter_map(
            |((file, name), nodes)| match index.cpg().node(*nodes.first()?) {
                CpgNode::Function { start_line, .. } => {
                    Some((file.as_str(), name.as_str(), *start_line))
                }
                _ => None,
            },
        )
        .collect();
    anyhow::ensure!(
        !candidates.is_empty(),
        "no functions are indexed under {}",
        cfg.repo_root.display()
    );

    let wanted = functions.min(candidates.len());
    let stride = candidates.len() / wanted;
    let mut next_id = 1u64;
    let mut emitted = 0usize;
    for slot in 0..wanted {
        for offset in 0..stride {
            let (file, name, start_line) = candidates[slot * stride + offset];
            let requests = requests_for(next_id, file, name, start_line);
            if run_workload(&provider, &requests)?.errors == 0 {
                for request in &requests {
                    writeln!(out, "{request}")?;
                }
                next_id += requests.len() as u64;
                emitted += 1;
                break;
            }
        }
    }
    anyhow::ensure!(
        emitted > 0,
        "no function under {} answers all four workload queries",
        cfg.repo_root.display()
    );
    Ok(())
}

#[cfg(test)]
#[path = "mem_report_tests.rs"]
mod tests;
