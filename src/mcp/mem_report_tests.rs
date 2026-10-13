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
    assert!(text.starts_with("{\"prism_mem_report\":2,"), "{text}");
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
    assert_eq!(report["prism_mem_report"], 2);
    // Every PRISM_* setting is echoed except the variable that selects this mode.
    let env = report["env"].as_object().expect("env section");
    assert!(env.keys().all(|name| name.starts_with("PRISM_")), "{env:?}");
    assert!(!env.contains_key("PRISM_MCP_MEM_REPORT"), "{env:?}");
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

    let report = report(&cfg, &options(Some(workload.clone()), true));
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
    let digest = format!("{:x}", Sha256::digest(std::fs::read(&workload).unwrap()));
    assert_eq!(report["workload"]["sha256"], digest.as_str());
    assert_eq!(digest.len(), 64);
    assert_eq!(report["refresh"]["generation"], 1);
    assert_eq!(report["refresh"]["status"], "refreshed");
}

#[test]
fn only_prism_settings_are_echoed_and_never_the_mode_switch() {
    let vars = [
        ("PRISM_MCP_MEM_REPORT", "1"),
        ("PRISM_MCP_CONCISE_SHAPE", "slim"),
        ("PRISM_A", ""),
        ("PATH", "/bin"),
        ("NOT_PRISM_B", "x"),
        ("prism_lower", "y"),
    ]
    .map(|(name, value)| (name.into(), value.into()));
    let got = prism_settings(vars.into_iter());
    assert_eq!(
        got.into_iter().collect::<Vec<_>>(),
        [
            ("PRISM_A".to_string(), String::new()),
            ("PRISM_MCP_CONCISE_SHAPE".to_string(), "slim".to_string()),
        ]
    );
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
