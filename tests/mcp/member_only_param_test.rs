#![cfg(feature = "mcp")]
//! PR-C R1 through MCP `taint_reaches`: a field seed never reaches a sibling field; the
//! whole-parameter (symbol) seed reaches every field read (accepted cost).

use assert_cmd::Command;
use serde_json::{json, Value};

const HANDLER: &str = "function handler(req) {\n  sinkBody(req.body);\n  sinkHeaders(req.headers);\n  sinkQuery(req.query);\n}\n";

fn call(repo: &std::path::Path, sources: Value, sink_line: usize) -> Value {
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"prc","version":"0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"taint_reaches","arguments":{
            "sources": sources,
            "sinks": [{"kind":"loc","file":"h.js","line":sink_line}]
        }}}),
    ];
    let stdin = messages
        .iter()
        .map(|m| format!("{m}\n"))
        .collect::<String>();
    let output = Command::cargo_bin("prism-mcp")
        .unwrap()
        .args(["--repo"])
        .arg(repo)
        .args(["--eager", "--no-cache"])
        .write_stdin(stdin)
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let response = stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["id"] == 2)
        .expect("tools/call response");
    assert_eq!(response["result"]["isError"], false, "{response}");
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

fn reachability(evidence: &Value) -> Value {
    evidence["reasoning"]["reachability"].clone()
}

#[test]
fn mcp_taint_reaches_field_seed_stays_on_its_field_and_symbol_seed_fans_out() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("h.js"), HANDLER).unwrap();
    let body = || json!([{"kind":"loc","file":"h.js","line":2}]);
    let handler = || json!([{"kind":"symbol","name":"handler"}]);

    assert_eq!(reachability(&call(dir.path(), body(), 2)), "Reached");
    for sink in [3, 4] {
        assert_ne!(
            reachability(&call(dir.path(), body(), sink)),
            "Reached",
            "req.body seed must not reach line {sink}"
        );
    }
    for sink in [2, 3, 4] {
        assert_eq!(
            reachability(&call(dir.path(), handler(), sink)),
            "Reached",
            "symbol seed `handler` reaches line {sink}"
        );
    }
}
