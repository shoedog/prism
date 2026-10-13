#![cfg(feature = "mcp")]

use assert_cmd::Command;

#[test]
fn invalid_output_env_value_warns_once_per_server() {
    let repo = tempfile::tempdir().expect("temp repo");
    std::fs::write(repo.path().join("a.py"), "def f():\n    return 1\n").expect("write a.py");
    let call = |id: u32| {
        format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"nav_callees","arguments":{{"seed":{{"kind":"symbol","name":"f","file":"a.py"}}}}}}}}"#
        )
    };
    let stdin = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#.to_string(),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string(),
        call(2),
        call(3),
    ]
    .join("\n")
        + "\n";

    let output = Command::cargo_bin("prism-mcp")
        .expect("prism-mcp binary")
        .arg("--repo")
        .arg(repo.path())
        .arg("--no-cache")
        .arg("--eager")
        .env("PRISM_MCP_CONCISE_SHAPE", "bogus")
        .env_remove("PRISM_MCP_STRUCTURED_CONTENT")
        .env_remove("PRISM_MCP_MAX_RESULT_CHARS")
        .write_stdin(stdin)
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let responses: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout line is JSON"))
        .collect();
    assert_eq!(responses.len(), 3, "initialize and two calls");
    assert_eq!(
        responses[1]["result"]["isError"], false,
        "the server still serves"
    );
    assert_eq!(responses[2]["result"]["isError"], false);

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert_eq!(
        stderr.matches("PRISM_MCP_CONCISE_SHAPE=").count(),
        1,
        "one warning for the whole server, not one per call: {stderr}"
    );
}
