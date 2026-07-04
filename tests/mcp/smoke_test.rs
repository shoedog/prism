#![cfg(feature = "mcp")]

use assert_cmd::Command;
use serde_json::Value;

#[test]
fn prism_mcp_protocol_smoke() {
    let repo = tempfile::tempdir().expect("temp repo");
    std::fs::write(repo.path().join("util.py"), "def helper():\n    return 1\n")
        .expect("write util.py");
    std::fs::write(
        repo.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .expect("write main.py");

    let mut command = Command::cargo_bin("prism-mcp").expect("prism-mcp binary");
    let output = command
        .arg("--repo")
        .arg(repo.path())
        // Pin the DEFAULT wire contract against ambient env: unset means the live defaults
        // (omit-default-path + slim) resolved in transport.rs.
        .env_remove("PRISM_MCP_STRUCTURED_CONTENT")
        .env_remove("PRISM_MCP_CONCISE_SHAPE")
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
    assert_eq!(
        tools.len(),
        8,
        "tools/list should return the six nav tools plus taint_reaches and refresh_index"
    );
    assert!(
        tools.iter().any(|tool| tool["name"] == "taint_reaches"),
        "tools/list should include taint_reaches"
    );
    assert!(
        tools.iter().any(|tool| tool["name"] == "refresh_index"),
        "tools/list should include refresh_index"
    );
    for tool in tools {
        if tool["name"] == "refresh_index" {
            assert_eq!(
                tool["annotations"]["readOnlyHint"], false,
                "refresh_index mutates the MCP session"
            );
            assert_eq!(
                tool["annotations"]["destructiveHint"], false,
                "refresh_index should not be destructive"
            );
            assert_eq!(
                tool["annotations"]["idempotentHint"], false,
                "refresh_index increments generation and refreshes state"
            );
            continue;
        }
        assert_eq!(
            tool["annotations"]["readOnlyHint"], true,
            "tool {tool:?} should be read-only"
        );
    }

    let tools_call = response_with_id(&responses, 3);
    let result = &tools_call["result"];
    assert_eq!(result["isError"], false, "tools/call should succeed");
    assert!(
        result.get("structuredContent").is_none(),
        "default path omits structuredContent under the live default (omit-default-path); \
         content[0].text is the canonical carrier: {result}"
    );
    let evidence: Value = result["content"][0]["text"]
        .as_str()
        .map(|text| serde_json::from_str(text).expect("content text JSON"))
        .expect("tools/call content text Evidence");
    let items = evidence["items"].as_array().expect("Evidence items array");
    let cross_file = items
        .iter()
        .find(|item| cross_file_function_item(item))
        .expect("expected cross-file callee for file-qualified run seed");
    assert!(
        cross_file["symbol"]["Function"].get("start_byte").is_none()
            && cross_file["symbol"]["Function"].get("ordinal").is_none(),
        "default Concise items use the slim shape (no byte offsets/ordinal): {cross_file}"
    );

    let agent_tools_call = response_with_id(&responses, 4);
    let agent_result = &agent_tools_call["result"];
    assert_eq!(
        agent_result["isError"], false,
        "agent tools/call should succeed"
    );
    let agent_evidence = agent_result
        .get("structuredContent")
        .cloned()
        .expect("agent tools/call structuredContent Evidence (agent views always keep it)");
    assert_eq!(
        agent_evidence["query"], evidence["query"],
        "agent view answers the same query as the default path"
    );
    let agent_items = agent_evidence["items"]
        .as_array()
        .expect("agent Evidence items array");
    let agent_cross_file = agent_items
        .iter()
        .find(|item| cross_file_function_item(item))
        .expect("agent view carries the same cross-file callee");
    assert!(
        agent_cross_file["symbol"]["Function"]
            .get("start_byte")
            .is_some(),
        "agent structuredContent stays FULL canonical (slim never reaches it): {agent_cross_file}"
    );
    let agent_view: Value =
        serde_json::from_str(agent_result["content"][0]["text"].as_str().unwrap())
            .expect("agent_json content text");
    assert_eq!(agent_view["meta"]["schema_version"], "0.4");
    assert_eq!(agent_view["meta"]["indexing_policy"], "code_role_v1");
    assert_eq!(agent_result["_meta"]["prism/view_schema_version"], "0.4");
    assert_eq!(
        agent_result["_meta"]["prism/view_indexing_policy"],
        "code_role_v1"
    );
}

fn lifecycle_messages() -> String {
    [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"mcp-smoke","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"},"format":"agent_json","profile":"dependencies"}}}"#,
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
        .is_some_and(|file| file == "util.py")
}
