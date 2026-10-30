use assert_cmd::Command;
use serde_json::json;

fn check(format: &str, line: usize, kind: &str, message: &str) {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(
        repo.path().join("a.py"),
        "# comment\ndef run():\n    return 1\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "callers", "--repo"])
        .arg(repo.path())
        .args(["--location", &format!("a.py:{line}"), "--format", format])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let text = String::from_utf8(out.stdout).unwrap();
    if format == "json" {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&text).unwrap(),
            json!({"error": {kind: {"file": "a.py", "line": line}}})
        );
    } else {
        assert_eq!(text.trim(), format!("error: {message}: a.py:{line}"));
    }
}

#[test]
fn location_seed_cli_text_no_callable() {
    check("text", 1, "NoEnclosingCallable", "no enclosing callable");
}

#[test]
fn location_seed_cli_json_no_callable() {
    check("json", 1, "NoEnclosingCallable", "no enclosing callable");
}

#[test]
fn location_seed_cli_past_eof_unchanged() {
    for format in ["text", "json"] {
        check(format, 4, "LocationOutOfRange", "location out of range");
    }
}

fn anonymous_only(file: &str, source: &str) {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join(file), source).unwrap();
    let mut failures = Vec::new();
    for tool in ["callers", "callees", "ego", "symbol-spans"] {
        // Comment, blank line, and body statement, respectively.
        for line in [3, 4, 5] {
            for format in ["json", "text"] {
                let out = Command::cargo_bin("prism")
                    .unwrap()
                    .args(["nav", "--no-cache", tool, "--repo"])
                    .arg(repo.path())
                    .args(["--location", &format!("{file}:{line}"), "--format", format])
                    .output()
                    .unwrap();
                let text = String::from_utf8(out.stdout).unwrap();
                // JS Return statements are usable ego nodes even in anonymous callables.
                let usable_ego = tool == "ego" && file == "a.js" && line == 5;
                let correct = if usable_ego {
                    out.status.success()
                        && if format == "json" {
                            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
                            value["query"] == "ego:a.js:5"
                                && value["graph"]["nodes"]
                                    .as_array()
                                    .is_some_and(|nodes| !nodes.is_empty())
                        } else {
                            text.starts_with("ego:a.js:5\n")
                        }
                } else {
                    out.status.code() == Some(3)
                        && if format == "json" {
                            serde_json::from_str::<serde_json::Value>(&text).unwrap()
                                == json!({"error":{"NoNamedEnclosingCallable":{"file":file,"line":line}}})
                        } else {
                            text.trim()
                                == format!("error: no named enclosing callable: {file}:{line}")
                        }
                };
                if !correct {
                    failures.push(format!("{tool} {file}:{line} {format}: {text}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn location_seed_cli_anonymous_js() {
    anonymous_only(
        "a.js",
        "// module\nsetTimeout(function () {\n  // inside\n\n  return 1;\n}, 10);\n",
    );
}

#[test]
fn location_seed_cli_anonymous_go() {
    anonymous_only(
        "a.go",
        "package sample\nvar h = func() int {\n  // inside\n\n  return 1\n}\n",
    );
}

#[test]
fn location_seed_cli_usable_anonymous_ego_control() {
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args([
            "nav",
            "--no-cache",
            "ego",
            "--repo",
            "tests/fixtures/hapi-4552-source/test",
            "--location",
            "transmit.js:17",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["query"], "ego:transmit.js:17");
    assert!(
        result["graph"]["nodes"]
            .as_array()
            .is_some_and(|nodes| !nodes.is_empty()),
        "{result}"
    );
}
