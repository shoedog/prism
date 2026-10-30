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
