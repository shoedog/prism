#![cfg(feature = "mcp")]

use assert_cmd::Command;
use serde_json::{json, Value};

#[path = "../common/callable_wrappers.rs"]
mod callable_wrappers;

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
            assert_eq!(hint, "No recognized callable encloses this indexed line; seed a named function's line or symbol. If a function failed to parse, fix the syntax and refresh the index.", "{tool}");
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

fn anonymous_only(file: &str, source: &str) {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join(file), source).unwrap();
    let mut messages = vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"anonymous-seed","version":"0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    ];
    let mut expected = Vec::new();
    for tool in [
        "nav_callers",
        "nav_callees",
        "nav_ego_graph",
        "nav_symbol_spans",
    ] {
        for line in [3, 4, 5] {
            let id = expected.len() + 2;
            messages.push(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":{"seed":{"kind":"loc","file":file,"line":line}}}}));
            expected.push((id, tool, line));
        }
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
    let mut failures = Vec::new();
    for (id, tool, line) in expected {
        let response = responses.iter().find(|r| r["id"] == id).unwrap();
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        // Preserve usable Return statement nodes; enclosure errors are only the fallback.
        if tool == "nav_ego_graph" && file == "a.js" && line == 5 {
            let value: Value = serde_json::from_str(text).unwrap();
            assert_eq!(response["result"]["isError"], false);
            assert_eq!(value["query"], "ego:a.js:5");
            assert!(value["graph"]["nodes"]
                .as_array()
                .is_some_and(|nodes| !nodes.is_empty()));
            continue;
        }
        let wanted = json!({"error":{"NoNamedEnclosingCallable":{"file":file,"line":line}}});
        let correct = response["result"]["isError"] == true && text.rsplit_once('\n').is_some_and(|(error, hint)| {
            serde_json::from_str::<Value>(error).ok() == Some(wanted)
                && hint == "Navigation needs a named callable in the index; seed a named function's line or symbol. For anonymous JS/TS/TSX variable flow, use taint_reaches on a line with usable variable evidence."
        });
        if !correct {
            failures.push(format!("{tool} {file}:{line}: {response}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn location_seed_mcp_anonymous_js() {
    anonymous_only(
        "a.js",
        "// module\nsetTimeout(function () {\n  // inside\n\n  return 1;\n}, 10);\n",
    );
}

#[test]
fn location_seed_mcp_anonymous_go() {
    anonymous_only(
        "a.go",
        "package sample\nvar h = func() int {\n  // inside\n\n  return 1\n}\n",
    );
}

#[test]
fn location_seed_mcp_usable_anonymous_ego_control() {
    let stdin = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"ego-control","version":"0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nav_ego_graph","arguments":{"seed":{"kind":"loc","file":"transmit.js","line":17}}}}),
    ].iter().map(|m| format!("{m}\n")).collect::<String>();
    let out = Command::cargo_bin("prism-mcp")
        .unwrap()
        .args([
            "--repo",
            "tests/fixtures/hapi-4552-source/test",
            "--eager",
            "--no-cache",
        ])
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
    let response = responses.iter().find(|r| r["id"] == 2).unwrap();
    assert_ne!(response["result"]["isError"], true, "{response}");
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("ego:transmit.js:17"),
        "{response}"
    );
}

fn wrapper_requests(file: &str, source: &str, lines: &[usize], named: Option<&str>) {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join(file), source).unwrap();
    let mut messages = vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"wrapper-seed","version":"0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    ];
    let mut expected = Vec::new();
    for tool in [
        "nav_callers",
        "nav_callees",
        "nav_ego_graph",
        "nav_symbol_spans",
        "taint_reaches",
    ] {
        if named.is_some() && tool == "taint_reaches" {
            continue;
        }
        for &line in lines {
            let seed = json!({"kind":"loc","file":file,"line":line});
            let arguments = if tool == "taint_reaches" {
                json!({"sources":[seed]})
            } else {
                json!({"seed":seed})
            };
            let id = expected.len() + 2;
            messages.push(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":arguments}}));
            expected.push((id, tool, line));
        }
    }
    let out = Command::cargo_bin("prism-mcp")
        .unwrap()
        .arg("--repo")
        .arg(repo.path())
        .args(["--eager", "--no-cache"])
        .write_stdin(
            messages
                .iter()
                .map(|m| format!("{m}\n"))
                .collect::<String>(),
        )
        .assert()
        .success()
        .get_output()
        .clone();
    let responses = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let mut failures = Vec::new();
    for (id, tool, line) in expected {
        let response = responses.iter().find(|r| r["id"] == id).unwrap();
        let result = &response["result"];
        let text = result["content"][0]["text"].as_str().unwrap();
        let correct = if let Some(name) = named {
            result["isError"] != true
                && serde_json::from_str::<Value>(text).is_ok_and(|value| {
                    if tool == "nav_ego_graph" {
                        value["graph"]["nodes"]
                            .as_array()
                            .is_some_and(|nodes| !nodes.is_empty())
                    } else {
                        text.contains(name)
                    }
                })
        } else {
            result["isError"] == true
                && text.rsplit_once('\n').is_some_and(|(error, hint)| {
                    serde_json::from_str::<Value>(error).ok()
                        == Some(json!({"error":{"NoEnclosingCallable":{"file":file,"line":line}}}))
                        && hint.starts_with("No recognized callable encloses this indexed line;")
                })
        };
        if !correct {
            failures.push(format!("{tool} {file}:{line}: {response}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn wrapper_class(case: usize) {
    let (file, source, lines) = callable_wrappers::CLASS_CASES[case];
    wrapper_requests(file, source, lines, None);
}

#[test]
fn location_seed_wrapper_dataclass() {
    wrapper_class(0);
}
#[test]
fn location_seed_wrapper_attrs() {
    wrapper_class(1);
}
#[test]
fn location_seed_wrapper_template_class() {
    wrapper_class(2);
}
#[test]
fn location_seed_wrapper_plain_class_controls() {
    wrapper_class(3);
    wrapper_class(4);
}
#[test]
fn location_seed_wrapper_callable_controls() {
    for &(file, source, lines, name) in callable_wrappers::CALLABLE_CONTROLS {
        wrapper_requests(file, source, lines, Some(name));
    }
}
