//! Retry sequence coverage, synchronized with build publication rather than build latency.
use super::*;
use crate::mcp::lazy::{LazyTestHooks, SessionBuilder};
use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

fn call_repo_map(provider: &mut LazySessionProvider) -> Value {
    let request = json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": {"name": "nav_repo_map", "arguments": {}}
    });
    match call_tool_response_with_cap_and_mode(
        request.as_object().unwrap(),
        json!(3),
        provider,
        &ToolRegistry::all_v1(),
        super::super::output::MAX_RESULT_CHARS,
        StructuredContentMode::Always,
        ConciseShapeMode::Legacy,
    ) {
        Dispatch::Response(response) => response,
        Dispatch::NoResponse => panic!("tools/call must return a response"),
    }
}

fn assert_answer(response: &Value, body: Value, meta: Value, is_error: bool) {
    // Compare both complete wire views, ignoring only whitespace in the JSON text.
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("missing content text; full response: {response:#}"));
    let content: Value = serde_json::from_str(text).unwrap_or_else(|error| {
        panic!("invalid JSON content: {error}; full response: {response:#}")
    });
    let mut normalized = response.clone();
    normalized["result"]["content"][0]["text"] = content;
    assert_eq!(
        normalized,
        json!({
            "jsonrpc": "2.0", "id": 3,
            "result": {
                "content": [{"type": "text", "text": body}],
                "structuredContent": body,
                "isError": is_error,
                "_meta": meta
            }
        }),
        "full response: {response:#}"
    );
}

#[test]
fn lazy_runtime_reports_build_failures_then_retries_until_success() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    let mut cfg = crate::mcp::ServerConfig::new(dir.path().to_path_buf());
    cfg.cache = crate::mcp::CacheMode::NoCache;
    let outcomes = Mutex::new(VecDeque::from(["first", "second", "ready"]));
    let builder_cfg = cfg.clone();
    let builder: SessionBuilder = Arc::new(move || {
        let outcome = outcomes.lock().unwrap().pop_front().unwrap();
        match outcome {
            "first" => anyhow::bail!("first build failure"),
            "second" => anyhow::bail!("second build failure"),
            "ready" => crate::mcp::SessionProvider::bootstrap(&builder_cfg),
            _ => unreachable!(),
        }
    });
    let (published, published_rx) = mpsc::channel();
    let published_rx = Mutex::new(published_rx);
    let hooks = LazyTestHooks {
        published: Some(Arc::new(move || published.send(()).unwrap())),
        before_wait: Some(Arc::new(move || {
            // This test pins the retry sequence, not latency. The outcome is queued before
            // recv_timeout runs, even when building takes longer than the call's wait budget.
            // This timeout is only a watchdog for a worker that never publishes.
            published_rx
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(30))
                .expect("build worker must publish its outcome");
        })),
    };
    let mut provider =
        LazySessionProvider::with_builder_and_hooks(&cfg, Duration::from_nanos(1), builder, hooks)
            .unwrap();

    for (attempt, cause) in [(1, "first build failure"), (2, "second build failure")] {
        let response = call_repo_map(&mut provider);
        assert_answer(
            &response,
            json!({
                "status": "build_failed", "cause": cause,
                "message": "the server keeps running; the next tool call retries the build"
            }),
            json!({
                "prism/schema_version": SCHEMA_VERSION,
                "prism/index_state": "failed", "prism/retryable": true
            }),
            true,
        );
        assert_eq!(provider.attempts(), attempt, "full response: {response:#}");
        assert_eq!(
            provider.last_error(),
            Some(cause),
            "full response: {response:#}"
        );
    }

    let response = call_repo_map(&mut provider);
    assert_answer(
        &response,
        json!({
            "query": "repo-map", "items": [], "truncated": false, "warnings": [],
            "graph": {
                "nodes": [{"symbol": null, "location": {
                    "file": "a.py", "start_line": 1, "end_line": 1,
                    "start_byte": 0, "end_byte": 0
                }}],
                "edges": []
            }
        }),
        json!({
            "prism/schema_version": SCHEMA_VERSION,
            "anthropic/maxResultSizeChars": super::super::output::MAX_RESULT_CHARS
        }),
        false,
    );
    assert_eq!(provider.attempts(), 3, "full response: {response:#}");
    assert_eq!(provider.last_error(), None, "full response: {response:#}");
    provider.join_build_for_tests();
}
