//! Unit tests for the `transport` module (split out to keep `transport.rs` under the 600-line guideline; kept in-crate via `#[path]` for private-item access).
use super::*;

fn run(msgs: Vec<&str>) -> Vec<serde_json::Value> {
    let s = crate::mcp::tools::test_support::session(&[("a.py", "def f():\n    return 1\n")]);
    let mut t = InMemoryTransport::new(msgs);
    serve_session(&s, &ToolRegistry::nav_v1(), &mut t).unwrap();
    t.responses().to_vec()
}

fn provider(files: &[(&str, &str)]) -> (tempfile::TempDir, crate::mcp::SessionProvider) {
    provider_with_policy(files, crate::mcp::RefreshPolicy::WarnOnly)
}

fn provider_with_policy(
    files: &[(&str, &str)],
    refresh_policy: crate::mcp::RefreshPolicy,
) -> (tempfile::TempDir, crate::mcp::SessionProvider) {
    let dir = tempfile::tempdir().unwrap();
    for (name, source) in files {
        write_file(dir.path(), name, source);
    }
    let cfg = crate::mcp::ServerConfig {
        repo_root: dir.path().to_path_buf(),
        cache: crate::mcp::CacheMode::NoCache,
        refresh_policy,
        startup: crate::mcp::StartupMode::Eager,
        first_call_wait: std::time::Duration::from_secs(20),
    };
    let provider = crate::mcp::SessionProvider::bootstrap(&cfg).unwrap();
    (dir, provider)
}

fn write_file(root: &std::path::Path, name: &str, source: &str) {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, source).unwrap();
}

fn run_provider(
    provider: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    msgs: Vec<&str>,
) -> Vec<serde_json::Value> {
    let mut t = InMemoryTransport::new(msgs);
    serve_runtime(provider, registry, &mut t).unwrap();
    t.responses().to_vec()
}

fn assert_warming_result(
    response: &serde_json::Value,
    mode: crate::mcp::output::StructuredContentMode,
) {
    let result = &response["result"];
    assert_eq!(result["isError"], true);
    assert_eq!(result["_meta"]["prism/index_state"], "warming");
    assert_eq!(result["_meta"]["prism/retryable"], true);
    let status: serde_json::Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(status["status"], "warming");
    assert!(status["elapsed_secs"].is_number());
    assert!(status["message"].as_str().unwrap().contains("retry"));
    match mode {
        crate::mcp::output::StructuredContentMode::Always => {
            assert_eq!(result["structuredContent"], status);
        }
        crate::mcp::output::StructuredContentMode::OmitDefaultPath => {
            assert!(result.get("structuredContent").is_none());
        }
    }
}

#[test]
fn lazy_runtime_returns_warming_status_in_both_wire_modes_while_ping_remains_ready() {
    use std::sync::{mpsc, Arc, Mutex};

    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "a.py", "def f():\n    return 1\n");
    let mut cfg = crate::mcp::ServerConfig::new(dir.path().to_path_buf());
    cfg.cache = crate::mcp::CacheMode::NoCache;
    let (release, rx) = mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));
    let builder_cfg = cfg.clone();
    let builder: crate::mcp::lazy::SessionBuilder = Arc::new(move || {
        rx.lock().unwrap().recv().unwrap();
        crate::mcp::SessionProvider::bootstrap(&builder_cfg)
    });
    let mut provider = crate::mcp::lazy::LazySessionProvider::with_builder(
        &cfg,
        std::time::Duration::ZERO,
        builder,
    )
    .unwrap();

    let responses = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![INIT, INITED, r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#],
    );
    assert_eq!(responses[1]["result"], serde_json::json!({}));
    assert_eq!(provider.attempts(), 1);

    let request = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_repo_map","arguments":{}}}"#;
    for mode in [
        crate::mcp::output::StructuredContentMode::Always,
        crate::mcp::output::StructuredContentMode::OmitDefaultPath,
    ] {
        let response = call_tool_at_cap_with_mode(
            &mut provider,
            &ToolRegistry::all_v1(),
            request,
            crate::mcp::output::MAX_RESULT_CHARS,
            mode,
        );
        assert_warming_result(&response, mode);
        assert_eq!(provider.attempts(), 1);
    }
    release.send(()).unwrap();
}

#[test]
fn lazy_runtime_validates_bad_calls_before_waiting_or_retrying() {
    use std::sync::{mpsc, Arc, Mutex};

    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "a.py", "def f():\n    return 1\n");
    let mut cfg = crate::mcp::ServerConfig::new(dir.path().to_path_buf());
    cfg.cache = crate::mcp::CacheMode::NoCache;
    let (release, rx) = mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));
    let builder_cfg = cfg.clone();
    let builder: crate::mcp::lazy::SessionBuilder = Arc::new(move || {
        rx.lock().unwrap().recv().unwrap();
        crate::mcp::SessionProvider::bootstrap(&builder_cfg)
    });
    let mut provider = crate::mcp::lazy::LazySessionProvider::with_builder(
        &cfg,
        std::time::Duration::from_secs(1),
        builder,
    )
    .unwrap();

    let requests = [
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_repo_map","arguments":[]}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"unknown","arguments":{}}}"#,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"refresh_index","arguments":{"force":true}}}"#,
    ];
    let started = std::time::Instant::now();
    for request in requests {
        let response = call_tool_at_cap_with_mode(
            &mut provider,
            &ToolRegistry::all_v1(),
            request,
            crate::mcp::output::MAX_RESULT_CHARS,
            crate::mcp::output::StructuredContentMode::Always,
        );
        assert!(
            response["result"]["isError"].as_bool().unwrap_or(false)
                || response["error"]["code"] == -32602
        );
    }
    assert!(started.elapsed() < std::time::Duration::from_millis(100));
    assert_eq!(provider.attempts(), 1);
    release.send(()).unwrap();
}

#[test]
fn lazy_runtime_reports_build_failures_then_retries_until_success() {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "a.py", "def f():\n    return 1\n");
    let mut cfg = crate::mcp::ServerConfig::new(dir.path().to_path_buf());
    cfg.cache = crate::mcp::CacheMode::NoCache;
    let outcomes = Arc::new(Mutex::new(VecDeque::from(["first", "second", "ready"])));
    let builder_cfg = cfg.clone();
    let builder: crate::mcp::lazy::SessionBuilder =
        Arc::new(
            move || match outcomes.lock().unwrap().pop_front().unwrap() {
                "first" => anyhow::bail!("first build failure"),
                "second" => anyhow::bail!("second build failure"),
                "ready" => crate::mcp::SessionProvider::bootstrap(&builder_cfg),
                _ => unreachable!(),
            },
        );
    let mut provider = crate::mcp::lazy::LazySessionProvider::with_builder(
        &cfg,
        std::time::Duration::from_secs(1),
        builder,
    )
    .unwrap();
    let request = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_repo_map","arguments":{}}}"#;

    for (attempt, cause) in [(1, "first build failure"), (2, "second build failure")] {
        let response = call_tool_at_cap_with_mode(
            &mut provider,
            &ToolRegistry::all_v1(),
            request,
            crate::mcp::output::MAX_RESULT_CHARS,
            crate::mcp::output::StructuredContentMode::Always,
        );
        let result = &response["result"];
        assert_eq!(result["isError"], true);
        assert_eq!(result["_meta"]["prism/index_state"], "failed");
        let status: serde_json::Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(status["status"], "build_failed");
        assert_eq!(status["cause"], cause);
        assert_eq!(provider.attempts(), attempt);
    }

    let response = call_tool_at_cap_with_mode(
        &mut provider,
        &ToolRegistry::all_v1(),
        request,
        crate::mcp::output::MAX_RESULT_CHARS,
        crate::mcp::output::StructuredContentMode::Always,
    );
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(provider.attempts(), 3);
    assert_eq!(provider.last_error(), None);
}

struct FailingAutoRuntime {
    provider: crate::mcp::SessionProvider,
}

impl SessionRuntime for FailingAutoRuntime {
    fn ensure_ready(&mut self) -> crate::mcp::lazy::Readiness {
        crate::mcp::lazy::Readiness::Ready
    }

    fn startup_mode(&self) -> crate::mcp::StartupMode {
        crate::mcp::StartupMode::Eager
    }

    fn session(&self) -> &crate::navigation::NavigationSession {
        self.provider.session()
    }

    fn freshness(&self) -> Option<&FreshnessProbe> {
        Some(self.provider.freshness())
    }

    fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        self.provider.known_stale_after_refresh()
    }

    fn refresh_policy(&self) -> crate::mcp::RefreshPolicy {
        crate::mcp::RefreshPolicy::AutoFull
    }

    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        self.provider.refresh()
    }

    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        anyhow::bail!("injected auto-refresh failure")
    }
}

fn run_failing_auto_runtime(
    provider: crate::mcp::SessionProvider,
    registry: &ToolRegistry,
    msgs: Vec<&str>,
) -> Vec<serde_json::Value> {
    let mut runtime = FailingAutoRuntime { provider };
    let mut t = InMemoryTransport::new(msgs);
    serve_runtime(&mut runtime, registry, &mut t).unwrap();
    t.responses().to_vec()
}

fn call_tool_at_cap(
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    request: &str,
    cap: usize,
) -> serde_json::Value {
    let message: serde_json::Value = serde_json::from_str(request).unwrap();
    let obj = message.as_object().unwrap();
    let id = obj.get("id").cloned().unwrap_or(serde_json::Value::Null);
    match call_tool_response_with_cap(obj, id, runtime, registry, cap) {
        Dispatch::Response(response) => response,
        Dispatch::NoResponse => panic!("tools/call must return a response"),
    }
}

fn serialized_len(value: &serde_json::Value) -> usize {
    serde_json::to_vec(value).unwrap().len()
}

/// Canonical Evidence/summary of a REAL-path (env-default) tools/call response. Under the live
/// default (`omit-default-path`, since 2026-07-03) the default path omits `structuredContent`
/// from the wire and `content[0].text` is the canonical carrier; agent views and explicit
/// `always`-mode tests still read `structuredContent` directly.
fn evidence_of(result: &serde_json::Value) -> serde_json::Value {
    result
        .get("structuredContent")
        .cloned()
        .or_else(|| {
            result["content"][0]["text"]
                .as_str()
                .and_then(|text| serde_json::from_str(text).ok())
        })
        .expect("tools/call response carries Evidence in structuredContent or content text")
}

/// S2/S3 test entry point: fixes the cap, structured-content mode, AND concise-shape mode
/// explicitly (never via env var mutation, which would race across parallel test threads) so
/// `omit-default-path` / `slim` behavior can be exercised deterministically alongside the existing
/// default-mode tests.
fn call_tool_at_cap_with_mode(
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    request: &str,
    cap: usize,
    mode: crate::mcp::output::StructuredContentMode,
) -> serde_json::Value {
    call_tool_at_cap_with_modes(
        runtime,
        registry,
        request,
        cap,
        mode,
        crate::mcp::concise_shape::ConciseShapeMode::Legacy,
    )
}

fn call_tool_at_cap_with_modes(
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    request: &str,
    cap: usize,
    mode: crate::mcp::output::StructuredContentMode,
    concise_shape_mode: crate::mcp::concise_shape::ConciseShapeMode,
) -> serde_json::Value {
    let message: serde_json::Value = serde_json::from_str(request).unwrap();
    let obj = message.as_object().unwrap();
    let id = obj.get("id").cloned().unwrap_or(serde_json::Value::Null);
    match call_tool_response_with_cap_and_mode(
        obj,
        id,
        runtime,
        registry,
        cap,
        mode,
        concise_shape_mode,
    ) {
        Dispatch::Response(response) => response,
        Dispatch::NoResponse => panic!("tools/call must return a response"),
    }
}

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

#[test]
fn initialize_result_carries_snapshot_and_view_instructions_once() {
    // S1: the notices formerly appended to every nav tool description now live ONCE in
    // `initialize`'s `instructions` (the protocol-legal home for state-once text).
    let o = run(vec![INIT]);
    let instructions = o[0]["result"]["instructions"]
        .as_str()
        .expect("initialize result must carry an instructions string");
    assert!(
        instructions.contains("repository snapshot loaded when prism-mcp started"),
        "instructions must state the snapshot notice: {instructions}"
    );
    assert!(
        instructions.contains("Optional LLM views are opt-in"),
        "instructions must state the view notice: {instructions}"
    );
    // Stated ONCE, not once per tool.
    assert_eq!(
        instructions.matches("repository snapshot loaded").count(),
        1
    );
}

#[test]
fn lifecycle_list_and_call() {
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
    ]);
    assert_eq!(o[0]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(o[1]["result"]["tools"].as_array().unwrap().len(), 6);
    assert!(o[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("nodes-at:a.py:1"));
    assert_eq!(o[2]["result"]["isError"], false);
}

#[test]
fn freshness_probe_does_not_mark_unedited_session_stale() {
    let (_dir, mut provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    let o = run_provider(
        &mut provider,
        &ToolRegistry::nav_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );
    assert!(o[1]["result"]["_meta"]
        .get("prism/index_freshness")
        .is_none());
}

#[test]
fn omit_default_path_wire_omits_structured_content_but_keeps_content_text_intact() {
    // S2: default (canonical_json) path under `omit-default-path` drops `structuredContent` from
    // the wire; `content[0].text` still carries the identical JSON (nothing is lost, only the
    // redundant second copy).
    let (_dir, mut provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    let always = call_tool_at_cap_with_mode(
        &mut provider,
        &ToolRegistry::nav_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS,
        crate::mcp::output::StructuredContentMode::Always,
    );
    let omitted = call_tool_at_cap_with_mode(
        &mut provider,
        &ToolRegistry::nav_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS,
        crate::mcp::output::StructuredContentMode::OmitDefaultPath,
    );

    assert!(always["result"].get("structuredContent").is_some());
    assert!(
        omitted["result"].get("structuredContent").is_none(),
        "omit-default-path must drop structuredContent from the default-path wire: {omitted}"
    );
    assert_eq!(
        omitted["result"]["content"][0]["text"], always["result"]["content"][0]["text"],
        "content_text must be unaffected by the structuredContent gate"
    );
    let content_text = omitted["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(
        parsed, always["result"]["structuredContent"],
        "content_text JSON must be identical to the omitted structuredContent"
    );
    assert_eq!(omitted["result"]["isError"], false);
}

#[test]
fn omit_default_path_agent_view_keeps_structured_content() {
    // S2: agent-format results are unaffected by the gate — structuredContent stays on the wire
    // regardless of mode, since it is the only canonical-Evidence carrier once content_text has
    // been rewritten into agent_json prose.
    let (_dir, mut provider) = provider(&[
        ("util.py", "def helper():\n    return 1\n"),
        (
            "main.py",
            "from util import helper\n\ndef run():\n    return helper()\n",
        ),
    ]);
    let response = call_tool_at_cap_with_mode(
        &mut provider,
        &ToolRegistry::nav_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"},"format":"agent_json","profile":"dependencies"}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS,
        crate::mcp::output::StructuredContentMode::OmitDefaultPath,
    );
    assert_eq!(response["result"]["isError"], false);
    assert!(
        response["result"].get("structuredContent").is_some(),
        "agent view must keep structuredContent even under omit-default-path: {response}"
    );
}

#[test]
fn omit_default_path_still_emits_freshness_warnings_and_metadata() {
    // S2: the freshness/stale-index contract (metadata + warnings) must survive the wire gate —
    // only `structuredContent`'s presence changes, never the freshness signal itself.
    let (dir, mut provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 12345\n").unwrap();
    let response = call_tool_at_cap_with_mode(
        &mut provider,
        &ToolRegistry::nav_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS,
        crate::mcp::output::StructuredContentMode::OmitDefaultPath,
    );
    let result = &response["result"];
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert_eq!(result["_meta"]["prism/stale_index_total"], 1);
    assert!(result.get("structuredContent").is_none());
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("StaleIndex"));
}

#[test]
fn omit_default_path_refresh_index_keeps_content_text_and_drops_structured_content() {
    // F3: `refresh_index` builds its `McpToolResult` directly (`tools_refresh::refresh_result`),
    // never going through `shape_result` — so the S2 wire gate here is exercised purely at the
    // transport boundary (`to_call_tool_result_value`). Pin that `omit-default-path` drops
    // `structuredContent` for `refresh_index` too, without losing any information (`content_text`
    // still carries the identical JSON) and without panicking.
    let (_dir, mut always_provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    let (_dir2, mut omitted_provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    let request = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#;

    let always = call_tool_at_cap_with_mode(
        &mut always_provider,
        &ToolRegistry::all_v1(),
        request,
        crate::mcp::output::MAX_RESULT_CHARS,
        crate::mcp::output::StructuredContentMode::Always,
    );
    let omitted = call_tool_at_cap_with_mode(
        &mut omitted_provider,
        &ToolRegistry::all_v1(),
        request,
        crate::mcp::output::MAX_RESULT_CHARS,
        crate::mcp::output::StructuredContentMode::OmitDefaultPath,
    );

    assert_eq!(always["result"]["isError"], false);
    assert_eq!(omitted["result"]["isError"], false);
    assert!(always["result"].get("structuredContent").is_some());
    assert!(
        omitted["result"].get("structuredContent").is_none(),
        "refresh_index under omit-default-path must drop structuredContent: {omitted}"
    );
    let content_text = omitted["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(
        parsed, always["result"]["structuredContent"],
        "content_text JSON must be identical to the omitted structuredContent — no information loss"
    );
}

#[test]
fn stale_index_metadata_warning_and_text_are_visible_after_edit() {
    let (dir, mut provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 12345\n").unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::nav_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );
    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert_eq!(result["_meta"]["prism/stale_index_total"], 1);
    assert_eq!(
        result["_meta"]["prism/stale_index_paths"],
        serde_json::json!(["a.py"])
    );
    assert!(evidence_of(&result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("StaleIndex"));
}

#[test]
fn stale_agent_json_clipped_text_remains_bounded_notice() {
    let (dir, mut provider) = provider(&[(
        "a.py",
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )]);
    std::fs::write(
        dir.path().join("a.py"),
        "def target():\n    return 12345\n\ndef caller():\n    return target()\n",
    )
    .unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::nav_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"target","file":"a.py"},"format":"agent_json","max_view_bytes":1}}}"#,
        ],
    );
    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert_eq!(result["_meta"]["prism/view_clipped"], true);
    assert!(result["content"][0]["text"].as_str().unwrap().len() <= 1);
    assert!(evidence_of(&result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn freshness_is_not_added_to_list_ping_unknown_or_input_errors() {
    let (dir, mut provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 12345\n").unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::nav_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":0}}}"#,
        ],
    );
    assert!(o[1].to_string().find("prism/index_freshness").is_none());
    assert!(o[2].to_string().find("prism/index_freshness").is_none());
    assert_eq!(o[3]["result"]["isError"], true);
    assert!(o[3]["result"]["_meta"]
        .get("prism/index_freshness")
        .is_none());
    assert_eq!(o[4]["result"]["isError"], true);
    assert!(o[4]["result"]["_meta"]
        .get("prism/index_freshness")
        .is_none());
}

#[test]
fn taint_reaches_receives_stale_warning() {
    let (dir, mut provider) = provider(&[(
        "app.py",
        "def f():\n    user = input()\n    value = user\n    sink(value)\n",
    )]);
    std::fs::write(
        dir.path().join("app.py"),
        "def f():\n    user = input()\n    value = user\n    sink(value)\n    return value\n",
    )
    .unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"taint_reaches","arguments":{"sources":[{"kind":"loc","file":"app.py","line":2}],"sinks":[{"kind":"loc","file":"app.py","line":4}]}}}"#,
        ],
    );
    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert!(evidence_of(&result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn refresh_index_rebuilds_session_and_clears_stale_warning() {
    let (dir, mut provider) = provider(&[("a.py", "def old():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );

    let stale_result = &o[1]["result"];
    assert_eq!(stale_result["_meta"]["prism/index_freshness"], "stale");
    assert!(evidence_of(&stale_result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));

    let refresh = &o[2]["result"];
    assert_eq!(refresh["isError"], false);
    assert!(refresh["_meta"].get("prism/index_freshness").is_none());
    assert_eq!(refresh["_meta"]["prism/refresh_strategy"], "full");
    assert!(refresh["_meta"]
        .get("prism/refresh_fallback_reason")
        .is_none());
    assert_eq!(evidence_of(&refresh)["status"], "refreshed");
    assert_eq!(evidence_of(&refresh)["strategy"], "full");
    assert!(evidence_of(&refresh)["fallback_reason"].is_null());
    assert_eq!(evidence_of(&refresh)["generation"], 1);
    assert_eq!(evidence_of(&refresh)["stale_before_refresh"], true);
    assert_eq!(
        evidence_of(&refresh)["stale_index_paths_before_refresh"],
        serde_json::json!(["a.py"])
    );
    let refresh_text: serde_json::Value =
        serde_json::from_str(refresh["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(refresh_text, evidence_of(&refresh));

    let fresh_result = &o[3]["result"];
    assert!(fresh_result["_meta"].get("prism/index_freshness").is_none());
    assert!(fresh_result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("fresh"));
    assert!(!evidence_of(&fresh_result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn refresh_index_without_edits_reports_fresh_prior_snapshot() {
    let (_dir, mut provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#,
        ],
    );
    let refresh = &o[1]["result"];
    assert_eq!(refresh["isError"], false);
    assert_eq!(evidence_of(&refresh)["strategy"], "full");
    assert!(evidence_of(&refresh)["fallback_reason"].is_null());
    assert_eq!(evidence_of(&refresh)["generation"], 1);
    assert_eq!(evidence_of(&refresh)["stale_before_refresh"], false);
    assert_eq!(evidence_of(&refresh)["stale_index_total_before_refresh"], 0);
}

#[test]
fn refresh_index_requires_no_arguments() {
    let (_dir, mut provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"refresh_index","arguments":{"force":true}}}"#,
        ],
    );
    assert_eq!(o[1]["result"]["isError"], true);
    assert!(o[1]["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("takes no arguments"));
}

#[test]
fn refresh_index_is_unavailable_in_static_serve_session() {
    let s = crate::mcp::tools::test_support::session(&[("a.py", "def f():\n    return 1\n")]);
    let mut t = InMemoryTransport::new(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#,
    ]);
    serve_session(&s, &ToolRegistry::all_v1(), &mut t).unwrap();
    let o = t.responses();
    assert_eq!(o[1]["result"]["isError"], true);
    assert!(o[1]["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("provider-backed"));
}

#[test]
fn auto_full_refresh_rebuilds_before_tool_and_clears_stale_warning() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def old():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoFull,
    );
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["isError"], false);
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "full");
    assert!(result["_meta"]
        .get("prism/refresh_fallback_reason")
        .is_none());
    assert_eq!(result["_meta"]["prism/refresh_generation"], 1);
    assert_eq!(result["_meta"]["prism/stale_index_total_before_refresh"], 1);
    assert_eq!(
        result["_meta"]["prism/stale_index_paths_before_refresh"],
        serde_json::json!(["a.py"])
    );
    assert!(result["_meta"].get("prism/index_freshness").is_none());
    assert_eq!(
        result["_meta"]["anthropic/maxResultSizeChars"],
        crate::mcp::output::MAX_RESULT_CHARS
    );
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("fresh"));
    assert!(!evidence_of(&result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn auto_full_refresh_uses_new_repo_map_after_file_addition() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def a():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoFull,
    );
    std::fs::write(dir.path().join("b.py"), "def b():\n    return 2\n").unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"b.py","line":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "full");
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("nodes-at:b.py:1"));
    // Slim (the live default) drops `location` when it duplicates the symbol span, so accept
    // the file from either carrier.
    assert!(evidence_of(&result)["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["location"]["file"] == "b.py"
                || item["symbol"]
                    .as_object()
                    .and_then(|s| s.values().next())
                    .is_some_and(|inner| inner["file"] == "b.py")
        }));
}

#[test]
fn auto_full_refresh_uses_new_callers_after_edit() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def target():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoFull,
    );
    std::fs::write(
        dir.path().join("a.py"),
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )
    .unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"target","file":"a.py"},"depth":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "full");
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("caller"));
    assert!(evidence_of(&result)["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["symbol"]["Function"]["name"] == "caller"));
}

#[test]
fn auto_incremental_refresh_uses_new_callers_after_edit() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def target():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoIncremental,
    );
    std::fs::write(
        dir.path().join("a.py"),
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )
    .unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"target","file":"a.py"},"depth":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "incremental");
    assert!(result["_meta"]
        .get("prism/refresh_fallback_reason")
        .is_none());
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("caller"));
    assert!(evidence_of(&result)["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["symbol"]["Function"]["name"] == "caller"));
}

#[test]
fn auto_incremental_refresh_recomputes_c_indirect_callers_after_assignment_edit() {
    let device_src =
        "struct Device { void (*callback)(); };\nvoid run(struct Device *d) { d->callback(); }\n";
    let (dir, mut provider) = provider_with_policy(
        &[
            ("device.c", device_src),
            (
                "setup.c",
                "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = old_handler; }\n",
            ),
        ],
        crate::mcp::RefreshPolicy::AutoIncremental,
    );
    write_file(
        dir.path(),
        "setup.c",
        "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = new_handler; }\n",
    );
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"new_handler","file":"setup.c"},"depth":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "incremental");
    assert!(result["_meta"]
        .get("prism/refresh_fallback_reason")
        .is_none());
    assert!(evidence_of(&result)["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["symbol"]["Function"]["file"] == "device.c"
            && item["symbol"]["Function"]["name"] == "run"));
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("run"));
}

#[test]
fn auto_incremental_refresh_uses_unbounded_changed_file_set() {
    let initial_files = (0..8)
        .map(|i| {
            (
                format!("f{i}.py"),
                format!("def target_{i}():\n    return {i}\n"),
            )
        })
        .collect::<Vec<_>>();
    let initial_refs = initial_files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect::<Vec<_>>();
    let (dir, mut provider) =
        provider_with_policy(&initial_refs, crate::mcp::RefreshPolicy::AutoIncremental);

    for i in 0..8 {
        write_file(
            dir.path(),
            &format!("f{i}.py"),
            &format!(
                "def target_{i}():\n    return {i}\n\ndef caller_{i}():\n    return target_{i}()\n"
            ),
        );
    }
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_callers","arguments":{"seed":{"kind":"symbol","name":"target_7","file":"f7.py"},"depth":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "incremental");
    assert!(evidence_of(&result)["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["symbol"]["Function"]["file"] == "f7.py"
            && item["symbol"]["Function"]["name"] == "caller_7"));
}

#[test]
fn auto_incremental_falls_back_to_full_on_file_addition() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def a():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoIncremental,
    );
    std::fs::write(dir.path().join("b.py"), "def b():\n    return 2\n").unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"b.py","line":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "full");
    assert_eq!(
        result["_meta"]["prism/refresh_fallback_reason"],
        "file_set_changed"
    );
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("nodes-at:b.py:1"));
}

#[test]
fn auto_incremental_falls_back_to_full_on_file_deletion() {
    let (dir, mut provider) = provider_with_policy(
        &[
            ("a.py", "def a():\n    return 1\n"),
            ("b.py", "def b():\n    return 2\n"),
        ],
        crate::mcp::RefreshPolicy::AutoIncremental,
    );
    std::fs::remove_file(dir.path().join("b.py")).unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "full");
    assert_eq!(
        result["_meta"]["prism/refresh_fallback_reason"],
        "file_set_changed"
    );
}

#[test]
fn auto_incremental_falls_back_to_full_on_manifest_change() {
    let (dir, mut provider) = provider_with_policy(
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            ),
            ("src/lib.rs", "pub fn f() -> i32 { 1 }\n"),
        ],
        crate::mcp::RefreshPolicy::AutoIncremental,
    );
    write_file(
        dir.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.2.0\"\nedition = \"2021\"\n",
    );
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"src/lib.rs","line":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "full");
    assert_eq!(
        result["_meta"]["prism/refresh_fallback_reason"],
        "manifest_changed"
    );
}

#[test]
fn auto_full_refresh_failure_keeps_old_session_and_stale_warning() {
    let (dir, provider) = provider(&[("a.py", "def old():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();
    let o = run_failing_auto_runtime(
        provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "failed");
    assert!(result["_meta"]["prism/auto_refresh_error"]
        .as_str()
        .unwrap()
        .contains("injected"));
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("old"));
    assert!(evidence_of(&result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn auto_full_raced_stale_refresh_retries_on_next_request_and_clears_warning() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def old():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoFull,
    );
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();
    provider.force_next_verification_for_tests(RefreshVerification::Diverged(
        FreshnessReport::from_changed_paths(["a.py".to_string()]),
    ));
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );

    let raced = &o[1]["result"];
    assert_eq!(raced["_meta"]["prism/auto_refresh"], "raced_stale");
    assert_eq!(raced["_meta"]["prism/refresh_strategy"], "full");
    assert_eq!(raced["_meta"]["prism/index_freshness"], "stale");
    assert!(evidence_of(&raced)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));

    let retried = &o[2]["result"];
    assert_eq!(retried["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(retried["_meta"]["prism/refresh_strategy"], "full");
    assert!(retried["_meta"].get("prism/index_freshness").is_none());
    assert!(evidence_of(&retried)["warnings"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn auto_incremental_raced_stale_refresh_retries_and_clears_via_no_semantic_change() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def old():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoIncremental,
    );
    write_file(dir.path(), "a.py", "def fresh():\n    return 2\n");
    provider.force_next_verification_for_tests(RefreshVerification::Diverged(
        FreshnessReport::from_changed_paths(["a.py".to_string()]),
    ));
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );

    let raced = &o[1]["result"];
    assert_eq!(raced["_meta"]["prism/auto_refresh"], "raced_stale");
    assert_eq!(raced["_meta"]["prism/refresh_strategy"], "incremental");
    assert_eq!(raced["_meta"]["prism/index_freshness"], "stale");
    assert!(evidence_of(&raced)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));

    let retried = &o[2]["result"];
    assert_eq!(retried["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(retried["_meta"]["prism/refresh_strategy"], "full");
    assert_eq!(
        retried["_meta"]["prism/refresh_fallback_reason"],
        "no_semantic_change"
    );
    assert!(retried["_meta"].get("prism/index_freshness").is_none());
    assert!(evidence_of(&retried)["warnings"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn auto_full_refresh_then_tool_error_preserves_tool_error_shape() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def old():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoFull,
    );
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();
    let o = run_provider(
        &mut provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":0}}}"#,
        ],
    );

    let result = &o[1]["result"];
    assert_eq!(result["isError"], true);
    assert!(result["_meta"].get("prism/auto_refresh").is_none());
    assert!(result["_meta"].get("prism/index_freshness").is_none());
}

#[test]
fn auto_full_clean_success_keeps_content_text_byte_identical_to_fresh_result() {
    let (dir, mut stale_provider) = provider_with_policy(
        &[("a.py", "def old():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoFull,
    );
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();
    let stale = run_provider(
        &mut stale_provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );
    let cfg = crate::mcp::ServerConfig {
        repo_root: dir.path().to_path_buf(),
        cache: crate::mcp::CacheMode::NoCache,
        refresh_policy: crate::mcp::RefreshPolicy::WarnOnly,
        startup: crate::mcp::StartupMode::Eager,
        first_call_wait: std::time::Duration::from_secs(20),
    };
    let mut fresh_provider = crate::mcp::SessionProvider::bootstrap(&cfg).unwrap();
    let fresh = run_provider(
        &mut fresh_provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        ],
    );

    assert_eq!(
        stale[1]["result"]["content"][0]["text"],
        fresh[1]["result"]["content"][0]["text"]
    );
    assert_eq!(
        stale[1]["result"]["structuredContent"],
        fresh[1]["result"]["structuredContent"]
    );
    assert_eq!(
        stale[1]["result"]["_meta"]["anthropic/maxResultSizeChars"],
        crate::mcp::output::MAX_RESULT_CHARS
    );
}

#[test]
fn auto_refresh_reserve_floor_covers_all_post_shape_metadata() {
    assert!(
        crate::mcp::output::MAX_RESULT_CHARS_FLOOR
            >= FRESHNESS_RESERVE_BYTES
                + AUTO_REFRESH_RESERVE_BYTES
                + MIN_MUTATING_TOOL_CAP_BYTES
                + ENVELOPE_RESERVE
    );
}

#[test]
fn auto_full_clean_under_floor_cap_preserves_refresh_metadata() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def old():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoFull,
    );
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();

    let response = call_tool_at_cap(
        &mut provider,
        &ToolRegistry::all_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS_FLOOR,
    );

    assert!(serialized_len(&response) <= crate::mcp::output::MAX_RESULT_CHARS_FLOOR);
    let result = &response["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "full");
    assert!(result["_meta"].get("prism/index_freshness").is_none());
}

#[test]
fn auto_incremental_under_floor_cap_preserves_strategy_metadata() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def old():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoIncremental,
    );
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();

    let response = call_tool_at_cap(
        &mut provider,
        &ToolRegistry::all_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS_FLOOR,
    );

    assert!(serialized_len(&response) <= crate::mcp::output::MAX_RESULT_CHARS_FLOOR);
    let result = &response["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "refreshed");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "incremental");
    assert!(result["_meta"].get("prism/index_freshness").is_none());
}

#[test]
fn auto_full_raced_stale_under_floor_cap_preserves_warning_metadata() {
    let (dir, mut provider) = provider_with_policy(
        &[("a.py", "def old():\n    return 1\n")],
        crate::mcp::RefreshPolicy::AutoFull,
    );
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();
    provider.force_next_verification_for_tests(RefreshVerification::Diverged(
        FreshnessReport::from_changed_paths(["a.py".to_string()]),
    ));

    let response = call_tool_at_cap(
        &mut provider,
        &ToolRegistry::all_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS_FLOOR,
    );

    assert!(serialized_len(&response) <= crate::mcp::output::MAX_RESULT_CHARS_FLOOR);
    let result = &response["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "raced_stale");
    assert_eq!(result["_meta"]["prism/refresh_strategy"], "full");
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert!(evidence_of(&result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn auto_full_refresh_failure_under_floor_cap_preserves_warning_metadata() {
    let (dir, provider) = provider(&[("a.py", "def old():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();
    let mut runtime = FailingAutoRuntime { provider };

    let response = call_tool_at_cap(
        &mut runtime,
        &ToolRegistry::all_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS_FLOOR,
    );

    assert!(serialized_len(&response) <= crate::mcp::output::MAX_RESULT_CHARS_FLOOR);
    let result = &response["result"];
    assert_eq!(result["_meta"]["prism/auto_refresh"], "failed");
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert!(evidence_of(&result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn warn_only_stale_under_floor_cap_preserves_warning_metadata() {
    let (dir, mut provider) = provider(&[("a.py", "def old():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 2\n").unwrap();

    let response = call_tool_at_cap(
        &mut provider,
        &ToolRegistry::all_v1(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_nodes_at","arguments":{"file":"a.py","line":1}}}"#,
        crate::mcp::output::MAX_RESULT_CHARS_FLOOR,
    );

    assert!(serialized_len(&response) <= crate::mcp::output::MAX_RESULT_CHARS_FLOOR);
    let result = &response["result"];
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert!(evidence_of(&result)["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn ping_returns_empty() {
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#,
    ]);
    assert!(o[1]["result"].is_object());
}

#[test]
fn notification_no_response() {
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":9}}"#,
    ]);
    assert_eq!(o.len(), 1);
    /* only initialize replied */
}

#[test]
fn tools_call_before_initialized_is_32600() {
    let o = run(vec![
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nav_repo_map","arguments":{}}}"#,
    ]);
    assert_eq!(o[0]["error"]["code"], -32600);
}

#[test]
fn initialized_notification_before_initialize_does_not_initialize() {
    // holistic-review MAJOR regression: a stray notifications/initialized must NOT complete the
    // handshake without a prior valid initialize, so a following tools/call is still -32600.
    let o = run(vec![
        INITED, // ignored (no prior initialize)
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nav_repo_map","arguments":{}}}"#,
    ]);
    assert_eq!(o[0]["error"]["code"], -32600);
}

#[test]
fn repeat_initialize_does_not_downgrade() {
    // re-review MAJOR regression: a second initialize after the session is Initialized must NOT
    // downgrade the lifecycle — the following tools/call must still succeed.
    let o = run(vec![
        INIT,
        INITED,
        INIT, // repeat initialize once already Initialized
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_repo_map","arguments":{}}}"#,
    ]);
    let last = o.last().unwrap();
    assert!(
        last.get("error").is_none(),
        "tools/call after a repeat initialize must work: {last}"
    );
    assert_eq!(last["result"]["isError"], false);
}

#[test]
fn id_bearing_initialized_notification_transitions_not_deadlocks() {
    // round-8 MINOR regression: a non-conformant client that attaches an `id` to
    // notifications/initialized must still transition the session (MCP §9: transition "regardless of
    // its body"), producing no response — NOT a -32600 that permanently deadlocks all later tools/*.
    let o = run(vec![
        INIT,
        r#"{"jsonrpc":"2.0","id":99,"method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    ]);
    // the id-bearing notification yields no response: only initialize + tools/list replied.
    assert_eq!(
        o.len(),
        2,
        "id-bearing notification must not produce a response: {o:?}"
    );
    assert!(
        o[1].get("error").is_none(),
        "tools/list after an id-bearing initialized notification must succeed (no deadlock): {}",
        o[1]
    );
    assert_eq!(o[1]["result"]["tools"].as_array().unwrap().len(), 6);
}

#[test]
fn oversized_id_is_32600() {
    // re-review MAJOR regression: a huge echoed id is rejected before it can bloat the envelope.
    let big = "x".repeat(300); // serializes to > MAX_ID_BYTES (256)
    let msg = format!(r#"{{"jsonrpc":"2.0","id":"{big}","method":"ping"}}"#);
    let o = run(vec![&msg]);
    assert_eq!(o[0]["error"]["code"], -32600);
}

#[test]
fn non_scalar_id_is_32600() {
    // re-review MINOR: JSON-RPC ids must be string|number|null; an object/array id is rejected.
    let o = run(vec![r#"{"jsonrpc":"2.0","id":{"x":1},"method":"ping"}"#]);
    assert_eq!(o[0]["error"]["code"], -32600);
}

#[test]
fn malformed_envelope_with_non_scalar_id_echoes_null() {
    // round-5 MAJOR: a malformed envelope (missing/bad jsonrpc) carrying a non-scalar id must NOT
    // echo that id — the error response id must be null (the malformed paths run before id_ok).
    let o = run(vec![r#"{"id":{"x":1},"jsonrpc":"1.0","method":"ping"}"#]);
    assert_eq!(o[0]["error"]["code"], -32600);
    assert!(o[0]["id"].is_null(), "must echo null, got {}", o[0]["id"]);
}

#[test]
fn malformed_envelope_with_oversized_id_echoes_null() {
    // round-5 MAJOR: a malformed envelope carrying an oversized string id must echo null, not the
    // unbounded id (would bloat the response envelope past the cap).
    let big = "x".repeat(300); // serializes to > MAX_ID_BYTES (256)
    let msg = format!(r#"{{"id":"{big}","jsonrpc":"1.0","method":"ping"}}"#);
    let o = run(vec![&msg]);
    assert_eq!(o[0]["error"]["code"], -32600);
    assert!(o[0]["id"].is_null(), "must echo null, got {}", o[0]["id"]);
}

#[test]
fn reserve_covers_envelope_and_max_id() {
    // re-review MAJOR: pin the cross-module invariant — the success envelope with a max-size id and
    // an empty result must fit within ENVELOPE_RESERVE (the budget shape_result subtracts from the
    // cap), so value(≤ cap-reserve) + envelope(≤ reserve) ≤ cap on the wire, for any accepted id.
    let max_id = Value::String("x".repeat(MAX_ID_BYTES - 2)); // ~256 bytes serialized (quotes)
    assert!(serde_json::to_string(&max_id).unwrap().len() <= MAX_ID_BYTES);
    let response = success_response(max_id, json!({}));
    let envelope_overhead = serde_json::to_string(&response).unwrap().len() + 1; // + framing newline
    assert!(
        envelope_overhead <= ENVELOPE_RESERVE,
        "envelope+max id ({envelope_overhead}) must fit in ENVELOPE_RESERVE ({ENVELOPE_RESERVE})",
    );
}

#[test]
fn unknown_method_is_32601() {
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","id":2,"method":"resources/read"}"#,
    ]);
    assert_eq!(o[1]["error"]["code"], -32601);
}

#[test]
fn unparseable_is_32700() {
    let o = run(vec![INIT, INITED, r#"{not json"#]);
    assert_eq!(o[1]["error"]["code"], -32700);
}

#[test]
fn missing_call_name_is_32602() {
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"arguments":{}}}"#,
    ]);
    assert_eq!(o[1]["error"]["code"], -32602);
}

#[test]
fn non_object_arguments_is_32602() {
    // round-6 MAJOR (protocol): per MCP the `arguments` member must be an object, so a wrong-TYPE
    // `arguments` (here `[]`) is a protocol error (-32602), distinct from a bad tool INPUT (isError).
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_repo_map","arguments":[]}}"#,
    ]);
    assert_eq!(o[1]["error"]["code"], -32602);
    assert!(
        o[1].get("result").is_none(),
        "non-object arguments must be a protocol error, not a result/isError"
    );
}

#[test]
fn object_and_absent_arguments_still_succeed() {
    // The fix must NOT regress the valid paths: explicit object arguments and absent arguments
    // (defaulted to `{}`) both still produce a successful result.
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_repo_map","arguments":{}}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_repo_map"}}"#,
    ]);
    assert_eq!(o[1]["result"]["isError"], false);
    assert_eq!(o[2]["result"]["isError"], false);
}

#[test]
fn unknown_tool_name_is_iserror() {
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#,
    ]);
    assert_eq!(o[1]["result"]["isError"], true);
}

#[test]
fn unknown_tool_with_huge_name_stays_under_floor_cap() {
    // round-5 MAJOR: error-path results bypass shape_result, so a hostile huge tool name must be
    // clamped at the source — the serialized CallToolResult must stay well under the floor cap.
    let huge = "x".repeat(5000);
    let msg = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"{huge}","arguments":{{}}}}}}"#
    );
    let o = run(vec![INIT, INITED, &msg]);
    assert_eq!(o[1]["result"]["isError"], true);
    let len = o[1]["result"].to_string().len();
    assert!(
        len < 4000,
        "isError result must stay under floor cap, got {len}"
    );
}

/// Drive `serve_session` over a real `StdioTransport` whose reader is raw bytes (so we can exercise
/// the byte-level oversized/bad-UTF-8 → Malformed → recover path), returning the parsed responses.
fn run_bytes(input: Vec<u8>) -> Vec<serde_json::Value> {
    let s = crate::mcp::tools::test_support::session(&[("a.py", "def f():\n    return 1\n")]);
    let mut out: Vec<u8> = Vec::new();
    {
        let mut t = StdioTransport::new(std::io::Cursor::new(input), &mut out);
        serve_session(&s, &ToolRegistry::nav_v1(), &mut t).unwrap();
    }
    String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn oversized_frame_is_recoverable_parse_error() {
    // round-5 MAJOR: an oversized inbound line must yield -32700 AND leave the server alive so the
    // subsequent valid initialize+tools/list still succeed (warm index not dropped).
    let oversized = "x".repeat(MAX_REQUEST_BYTES + 10);
    let input = format!("{oversized}\n{INIT}\n{INITED}\n{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}}\n");
    let o = run_bytes(input.into_bytes());
    assert_eq!(o[0]["error"]["code"], -32700);
    assert_eq!(o[1]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(o[2]["result"]["tools"].as_array().unwrap().len(), 6);
}

#[test]
fn bad_utf8_frame_is_recoverable_parse_error() {
    // round-5 MAJOR: a non-UTF-8 byte in one line must yield -32700 (recoverable), not kill the
    // server — the following valid initialize+tools/list still succeed.
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(&[0xff, 0xfe]); // invalid UTF-8
    input.push(b'\n');
    input.extend_from_slice(INIT.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(INITED.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(br#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    input.push(b'\n');
    let o = run_bytes(input);
    assert_eq!(o[0]["error"]["code"], -32700);
    assert_eq!(o[1]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(o[2]["result"]["tools"].as_array().unwrap().len(), 6);
}

#[test]
fn idless_request_is_32600() {
    let o = run(vec![
        INIT,
        INITED,
        r#"{"jsonrpc":"2.0","method":"tools/list"}"#,
    ]);
    assert_eq!(o[1]["error"]["code"], -32600);
}

#[test]
fn unknown_method_before_initialized_is_32600() {
    let o = run(vec![
        INIT,
        r#"{"jsonrpc":"2.0","id":2,"method":"resources/read"}"#,
    ]);
    assert_eq!(o[1]["error"]["code"], -32600);
}
