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
use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const INIT: &str = r#"{"jsonrpc":"2.0","id":"prism-mem-init","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"prism-mem-report","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

pub struct MemReportOptions {
    /// A file of `tools/call` requests, one JSON object per line, run after the build.
    pub workload: Option<PathBuf>,
    /// Run one full refresh at the end, for the refresh peak.
    pub refresh: bool,
    /// How long to wait after the workload before the steady-state sample.
    pub settle: Duration,
}

#[derive(serde::Serialize)]
struct Stage {
    name: &'static str,
    elapsed_ms: u64,
    #[serde(flatten)]
    memory: MemSample,
    /// Usable bytes of live allocations made by tree-sitter (the retained syntax trees), or
    /// `null` when the counting hooks are not installed (the binary installs them in report mode).
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

/// Read a workload file: every non-empty line is a `tools/call` request with an id.
fn load_workload(path: &Path) -> anyhow::Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading workload {}", path.display()))?;
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
    Ok(requests)
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
    refresh_policy: &'static str,
    workload: Option<String>,
    refresh: bool,
    settle_secs: u64,
}

#[derive(serde::Serialize)]
struct Report {
    /// Report format version. First field, so the line starts with `{"prism_mem_report":`.
    prism_mem_report: u32,
    build: BuildSection,
    platform: PlatformSection,
    config: ConfigSection,
    stages: Vec<Stage>,
    ru_maxrss_bytes: Option<u64>,
    attribution: Attribution,
    workload: Option<WorkloadStats>,
    refresh: Option<RefreshSummary>,
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
        refresh: options.refresh,
        settle_secs: options.settle.as_secs(),
    }
}

/// Build the index in observed stages, run the optional workload and refresh, and write one
/// JSON line describing the process's memory at each stage.
pub fn run(
    cfg: &ServerConfig,
    options: &MemReportOptions,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    // Reject a bad workload before paying for the build.
    let requests = options.workload.as_deref().map(load_workload).transpose()?;

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

    let workload = match &requests {
        Some(requests) => {
            let stats = run_workload(&provider, requests)?;
            recorder.mark("after_workload");
            Some(stats)
        }
        None => None,
    };

    std::thread::sleep(options.settle);
    recorder.mark("after_settle");

    let refresh = if options.refresh {
        let summary = provider.refresh()?;
        recorder.mark("after_refresh");
        Some(summary)
    } else {
        None
    };

    // Attribution allocates; it runs after the last sample.
    let report = Report {
        prism_mem_report: 1,
        build: build_section(),
        platform: PlatformSection {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        },
        config: config_section(cfg, options),
        stages: recorder.stages,
        ru_maxrss_bytes: ru_maxrss_bytes(),
        attribution: attribute(provider.session()),
        workload,
        refresh,
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
mod tests {
    use super::*;
    use crate::mcp::{CacheMode, RefreshPolicy, StartupMode};
    use serde_json::Value;

    fn repo() -> (tempfile::TempDir, ServerConfig) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n").unwrap();
        std::fs::write(
            dir.path().join("main.py"),
            "from util import helper\n\ndef run():\n    return helper()\n",
        )
        .unwrap();
        let cfg = ServerConfig {
            repo_root: dir.path().to_path_buf(),
            cache: CacheMode::NoCache,
            refresh_policy: RefreshPolicy::WarnOnly,
            startup: StartupMode::Eager,
            first_call_wait: std::time::Duration::from_secs(20),
        };
        (dir, cfg)
    }

    fn options(workload: Option<PathBuf>, refresh: bool) -> MemReportOptions {
        MemReportOptions {
            workload,
            refresh,
            settle: Duration::ZERO,
        }
    }

    fn report(cfg: &ServerConfig, options: &MemReportOptions) -> Value {
        let mut out = Vec::new();
        run(cfg, options, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("{\"prism_mem_report\":1,"), "{text}");
        assert_eq!(text.matches('\n').count(), 1, "exactly one line");
        serde_json::from_str(&text).unwrap()
    }

    fn stage_names(report: &Value) -> Vec<&str> {
        report["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|stage| stage["name"].as_str().unwrap())
            .collect()
    }

    fn workload_file(dir: &tempfile::TempDir, cfg: &ServerConfig, functions: usize) -> PathBuf {
        let mut lines = Vec::new();
        emit_workload(cfg, functions, &mut lines).unwrap();
        let path = dir.path().join("workload.jsonl");
        std::fs::write(&path, lines).unwrap();
        path
    }

    #[test]
    fn a_plain_report_has_the_build_stages_and_no_optional_ones() {
        let (_dir, cfg) = repo();
        let report = report(&cfg, &options(None, false));
        assert_eq!(
            stage_names(&report),
            [
                "start",
                "after_load_repo",
                "after_index",
                "after_call_edges",
                "after_settle"
            ]
        );
        assert!(report["workload"].is_null());
        assert!(report["refresh"].is_null());
        assert_eq!(report["attribution"]["files"], 2);
        assert_eq!(report["config"]["cache"], "no-cache");
        assert_eq!(report["config"]["refresh_policy"], "warn-only");
        assert_eq!(
            report["build"]["package_version"],
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(report["build"]["allocator"], "system");
        assert!(report["build"]["cpg_cache_version"].as_u64().unwrap() > 0);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn the_lifetime_peak_never_decreases_across_stages() {
        let (_dir, cfg) = repo();
        let report = report(&cfg, &options(None, true));
        let peaks: Vec<u64> = report["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|stage| stage["peak_bytes"].as_u64().expect("peak on this platform"))
            .collect();
        assert!(peaks.windows(2).all(|pair| pair[0] <= pair[1]), "{peaks:?}");
        let elapsed: Vec<u64> = report["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|stage| stage["elapsed_ms"].as_u64().unwrap())
            .collect();
        assert!(
            elapsed.windows(2).all(|pair| pair[0] <= pair[1]),
            "{elapsed:?}"
        );
    }

    #[test]
    fn a_generated_workload_runs_with_no_errors_and_adds_its_stage() {
        let (dir, cfg) = repo();
        let workload = workload_file(&dir, &cfg, 2);
        let lines = std::fs::read_to_string(&workload).unwrap().lines().count();
        assert_eq!(lines, 8, "four queries for each of two functions");

        let report = report(&cfg, &options(Some(workload), true));
        assert_eq!(
            stage_names(&report),
            [
                "start",
                "after_load_repo",
                "after_index",
                "after_call_edges",
                "after_workload",
                "after_settle",
                "after_refresh"
            ]
        );
        assert_eq!(report["workload"]["requests"], 8);
        assert_eq!(report["workload"]["responses"], 8);
        assert_eq!(report["workload"]["errors"], 0);
        assert!(report["workload"]["response_bytes"].as_u64().unwrap() > 0);
        assert_eq!(report["refresh"]["generation"], 1);
        assert_eq!(report["refresh"]["status"], "refreshed");
    }

    #[test]
    fn workload_generation_is_deterministic_and_bounded_by_the_function_count() {
        let (_dir, cfg) = repo();
        let emit = |functions| {
            let mut out = Vec::new();
            emit_workload(&cfg, functions, &mut out).unwrap();
            String::from_utf8(out).unwrap()
        };
        assert_eq!(emit(2), emit(2));
        // The fixture has two functions; asking for more yields all of them, not an error.
        assert_eq!(emit(50).lines().count(), 8);
        for line in emit(1).lines() {
            let request: Value = serde_json::from_str(line).unwrap();
            assert_eq!(request["method"], "tools/call");
            assert!(request["id"].is_u64());
        }
    }

    #[test]
    fn workload_errors_are_counted() {
        let (dir, cfg) = repo();
        let path = dir.path().join("bad-seeds.jsonl");
        std::fs::write(
            &path,
            concat!(
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"no_such_function","file":"main.py"}}}}"#,
                "\n",
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#,
                "\n"
            ),
        )
        .unwrap();
        let report = report(&cfg, &options(Some(path), false));
        assert_eq!(report["workload"]["requests"], 2);
        assert_eq!(report["workload"]["responses"], 2);
        assert_eq!(report["workload"]["errors"], 1);
    }

    #[test]
    fn a_workload_file_is_validated_before_the_build() {
        let (dir, cfg) = repo();
        let write = |name: &str, contents: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, contents).unwrap();
            path
        };
        let error = |path: PathBuf| {
            let mut out = Vec::new();
            let error = run(&cfg, &options(Some(path), false), &mut out).unwrap_err();
            assert!(
                out.is_empty(),
                "no report is written for a rejected workload"
            );
            error.to_string()
        };

        let message = error(write(
            "not-a-call.jsonl",
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{}}\n\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n",
        ));
        assert!(message.contains("line 3"), "{message}");
        assert!(message.contains("tools/call"), "{message}");

        let message = error(write("not-json.jsonl", "nope\n"));
        assert!(
            message.contains("line 1") && message.contains("not JSON"),
            "{message}"
        );

        let message = error(write("empty.jsonl", "\n\n"));
        assert!(message.contains("has no requests"), "{message}");

        let message = error(dir.path().join("missing.jsonl"));
        assert!(message.contains("missing.jsonl"), "{message}");
    }

    #[test]
    fn an_empty_repository_cannot_generate_a_workload() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = ServerConfig::new(dir.path().to_path_buf());
        cfg.cache = CacheMode::NoCache;
        cfg.startup = StartupMode::Eager;
        let mut out = Vec::new();
        let error = emit_workload(&cfg, 5, &mut out).unwrap_err().to_string();
        assert!(error.contains("no functions are indexed"), "{error}");
        assert!(out.is_empty());
    }
}
