//! Unit tests for the `transport` module (split out to keep `transport.rs` under the 600-line guideline; kept in-crate via `#[path]` for private-item access).
use super::*;

fn run(msgs: Vec<&str>) -> Vec<serde_json::Value> {
    let s = crate::mcp::tools::test_support::session(&[("a.py", "def f():\n    return 1\n")]);
    let mut t = InMemoryTransport::new(msgs);
    serve_session(&s, &ToolRegistry::nav_v1(), &mut t).unwrap();
    t.responses().to_vec()
}

fn provider(files: &[(&str, &str)]) -> (tempfile::TempDir, crate::mcp::SessionProvider) {
    let dir = tempfile::tempdir().unwrap();
    for (name, source) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, source).unwrap();
    }
    let cfg = crate::mcp::ServerConfig {
        repo_root: dir.path().to_path_buf(),
        cache: crate::mcp::CacheMode::NoCache,
    };
    let provider = crate::mcp::SessionProvider::bootstrap(&cfg).unwrap();
    (dir, provider)
}

fn run_provider(
    provider: &crate::mcp::SessionProvider,
    registry: &ToolRegistry,
    msgs: Vec<&str>,
) -> Vec<serde_json::Value> {
    let mut t = InMemoryTransport::new(msgs);
    serve_session_with_freshness(
        provider.session(),
        Some(provider.freshness()),
        registry,
        &mut t,
    )
    .unwrap();
    t.responses().to_vec()
}

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

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
    let (_dir, provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    let o = run_provider(
        &provider,
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
fn stale_index_metadata_warning_and_text_are_visible_after_edit() {
    let (dir, provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 12345\n").unwrap();
    let o = run_provider(
        &provider,
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
    assert!(result["structuredContent"]["warnings"]
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
    let (dir, provider) = provider(&[(
        "a.py",
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )]);
    std::fs::write(
        dir.path().join("a.py"),
        "def target():\n    return 12345\n\ndef caller():\n    return target()\n",
    )
    .unwrap();
    let o = run_provider(
        &provider,
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
    assert!(result["structuredContent"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == "StaleIndex"));
}

#[test]
fn freshness_is_not_added_to_list_ping_unknown_or_input_errors() {
    let (dir, provider) = provider(&[("a.py", "def f():\n    return 1\n")]);
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 12345\n").unwrap();
    let o = run_provider(
        &provider,
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
    let (dir, provider) = provider(&[(
        "app.py",
        "def f():\n    user = input()\n    value = user\n    sink(value)\n",
    )]);
    std::fs::write(
        dir.path().join("app.py"),
        "def f():\n    user = input()\n    value = user\n    sink(value)\n    return value\n",
    )
    .unwrap();
    let o = run_provider(
        &provider,
        &ToolRegistry::all_v1(),
        vec![
            INIT,
            INITED,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"taint_reaches","arguments":{"sources":[{"kind":"loc","file":"app.py","line":2}],"sinks":[{"kind":"loc","file":"app.py","line":4}]}}}"#,
        ],
    );
    let result = &o[1]["result"];
    assert_eq!(result["_meta"]["prism/index_freshness"], "stale");
    assert!(result["structuredContent"]["warnings"]
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
