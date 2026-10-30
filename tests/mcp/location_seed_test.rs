#![cfg(feature = "mcp")]

use assert_cmd::Command;
use serde_json::{json, Value};

fn check(tool: &str) {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(
        repo.path().join("a.py"),
        "# comment\ndef run():\n    value = input()\n    sink(value)\n",
    )
    .unwrap();
    let mut messages = vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"location-seed","version":"0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    ];
    let mut expected = Vec::new();
    for line in [1, 5, 99] {
        let seed = json!({"kind":"loc","file":"a.py","line":line});
        let arguments = if tool == "taint_reaches" {
            json!({"sources":[seed],"sinks":[{"kind":"loc","file":"a.py","line":4}]})
        } else {
            json!({"seed":seed})
        };
        let id = expected.len() + 2;
        messages.push(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":arguments}}));
        expected.push((id, tool, line));
    }
    let stdin = messages
        .iter()
        .map(|m| format!("{m}\n"))
        .collect::<String>();
    let out = Command::cargo_bin("prism-mcp")
        .unwrap()
        .arg("--repo")
        .arg(repo.path())
        .args(["--eager", "--no-cache"])
        .write_stdin(stdin)
        .assert()
        .success()
        .get_output()
        .clone();
    let responses = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    for (id, tool, line) in expected {
        let response = responses.iter().find(|r| r["id"] == id).unwrap();
        assert_eq!(response["result"]["isError"], true, "{tool}: {response}");
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let (error, hint) = text.rsplit_once('\n').unwrap();
        let kind = if line == 1 {
            "NoEnclosingCallable"
        } else {
            "LocationOutOfRange"
        };
        assert_eq!(
            serde_json::from_str::<Value>(error).unwrap(),
            json!({"error":{kind:{"file":"a.py","line":line}}}),
            "{tool}: {text}"
        );
        if line == 1 {
            assert_eq!(hint, "This line is outside any callable; seed a line inside a function body, or use a symbol seed.", "{tool}");
        } else {
            assert_eq!(
                hint, "Choose a line that exists in the indexed file and retry.",
                "{tool}"
            );
        }
    }
}

#[test]
fn location_seed_mcp_symbol_spans() {
    check("nav_symbol_spans");
}

#[test]
fn location_seed_mcp_callers() {
    check("nav_callers");
}

#[test]
fn location_seed_mcp_callees() {
    check("nav_callees");
}

#[test]
fn location_seed_mcp_ego() {
    check("nav_ego_graph");
}

#[test]
fn location_seed_mcp_reasoning() {
    check("taint_reaches");
}
