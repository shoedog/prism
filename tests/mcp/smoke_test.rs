#![cfg(feature = "mcp")]

use assert_cmd::Command;
use serde_json::Value;

#[test]
fn prism_mcp_protocol_smoke() {
    let mut command = Command::cargo_bin("prism-mcp").expect("prism-mcp binary");
    let output = command
        .arg("--repo")
        .arg(".")
        .write_stdin(lifecycle_messages())
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let responses = parse_json_rpc_stdout(&stdout);
    let tools_list = response_with_id(&responses, 2);
    let tools = tools_list["result"]["tools"]
        .as_array()
        .expect("tools/list result tools array");
    assert_eq!(tools.len(), 6, "tools/list should return the six nav tools");
    for tool in tools {
        assert_eq!(
            tool["annotations"]["readOnlyHint"], true,
            "tool {tool:?} should be read-only"
        );
    }

    let tools_call = response_with_id(&responses, 3);
    let result = &tools_call["result"];
    assert_eq!(result["isError"], false, "tools/call should succeed");
    let evidence = result
        .get("structuredContent")
        .cloned()
        .or_else(|| {
            result["content"][0]["text"]
                .as_str()
                .and_then(|text| serde_json::from_str::<Value>(text).ok())
        })
        .expect("tools/call Evidence");
    let items = evidence["items"].as_array().expect("Evidence items array");
    assert!(
        items.iter().any(cross_file_function_item),
        "expected at least one cross-file callee for file-qualified run_slicing_inner seed"
    );
}

fn lifecycle_messages() -> String {
    [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"mcp-smoke","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run_slicing_inner","file":"src/algorithms/mod.rs"}}}}"#,
    ]
    .join("\n")
        + "\n"
}

fn parse_json_rpc_stdout(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let value = serde_json::from_str::<Value>(line)
                .unwrap_or_else(|err| panic!("stdout line is not JSON: {err}: {line:?}"));
            assert_eq!(value["jsonrpc"], "2.0", "stdout line is not JSON-RPC");
            assert!(
                value.get("result").is_some() || value.get("error").is_some(),
                "stdout line is missing JSON-RPC result/error: {value:?}"
            );
            value
        })
        .collect()
}

fn response_with_id(responses: &[Value], id: i64) -> &Value {
    responses
        .iter()
        .find(|response| response["id"] == id)
        .unwrap_or_else(|| panic!("missing JSON-RPC response id {id}; responses: {responses:?}"))
}

fn cross_file_function_item(item: &Value) -> bool {
    item["symbol"]["Function"]["file"]
        .as_str()
        .is_some_and(|file| file != "src/algorithms/mod.rs")
}
