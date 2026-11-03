//! #636: a weak `taint_sink` finding is a `suggestion`: hidden by the default review floor
//! (with a `findings_below_floor` count), shown by `--review-min-severity suggestion`, level
//! `note` in SARIF, and present in `--format json`. A run that hides nothing has no count key.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn repo(source: &str) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::write(dir.join("a.py"), source).unwrap();
    let spec = serde_json::json!({ "files": [
        { "file_path": "a.py", "modify_type": "Modified", "diff_lines": [4, 5] }
    ]});
    let diff = dir.join("diff.json");
    fs::write(&diff, spec.to_string()).unwrap();
    (tmp, dir, diff)
}

const WEAK: &str = "import os\nQ = 'SELECT a FROM t WHERE id = %s'\ndef handle(req, cur):\n    x = req.GET['x']\n    cur.execute(Q, (x,))\n";
const STRONG: &str = "import os\nQ = 'SELECT a FROM t WHERE id = %s'\ndef handle(req, cur):\n    x = req.GET['x']\n    os.system(x)\n";

fn run(source: &str, extra: &[&str]) -> Value {
    let (_tmp, dir, diff) = repo(source);
    let mut cmd = Command::cargo_bin("prism").unwrap();
    cmd.args([
        "--repo",
        dir.to_str().unwrap(),
        "--diff",
        diff.to_str().unwrap(),
        "--algorithm",
        "taint",
    ]);
    cmd.args(extra);
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn sinks(doc: &Value) -> Vec<&Value> {
    doc["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["category"] == "taint_sink")
        .collect()
}

#[test]
fn weak_only_run_is_hidden_by_default_and_counted() {
    let doc = run(WEAK, &["--format", "review"]);
    assert!(sinks(&doc).is_empty(), "weak sink shown by default: {doc}");
    assert_eq!(doc["findings_below_floor"], 1, "{doc}");
}

#[test]
fn suggestion_floor_shows_the_weak_finding() {
    let doc = run(
        WEAK,
        &["--format", "review", "--review-min-severity", "suggestion"],
    );
    let shown = sinks(&doc);
    assert_eq!(shown.len(), 1, "{doc}");
    assert_eq!(shown[0]["severity"], "suggestion");
    assert!(doc.get("findings_below_floor").is_none(), "{doc}");
}

#[test]
fn strong_run_has_no_count_key() {
    let doc = run(STRONG, &["--format", "review"]);
    assert_eq!(sinks(&doc).len(), 1, "{doc}");
    assert!(doc.get("findings_below_floor").is_none(), "{doc}");
}

#[test]
fn json_keeps_the_weak_finding() {
    let doc = run(WEAK, &["--format", "json"]);
    let shown: Vec<_> = doc["results"][0]["findings"]
        .as_array()
        .or_else(|| doc["findings"].as_array())
        .unwrap()
        .iter()
        .filter(|f| f["category"] == "taint_sink")
        .collect();
    assert_eq!(shown.len(), 1, "{doc}");
    assert_eq!(shown[0]["severity"], "suggestion");
}

#[test]
fn sarif_level_of_a_weak_finding_is_note() {
    let doc = run(WEAK, &["--format", "sarif"]);
    let results = doc["runs"][0]["results"].as_array().unwrap();
    let sink: Vec<&Value> = results
        .iter()
        .filter(|r| r["properties"]["category"] == "taint_sink")
        .collect();
    assert_eq!(sink.len(), 1, "{doc}");
    assert_eq!(sink[0]["level"], "note", "{doc}");
}
