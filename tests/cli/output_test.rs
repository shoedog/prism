use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn prism_cmd() -> Command {
    Command::cargo_bin("prism").unwrap()
}

fn fixture_path(relative: &str) -> String {
    format!("tests/fixtures/{}", relative)
}

#[test]
fn test_default_algorithm_text_output() {
    prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
        ])
        .assert()
        .success();
}

#[test]
fn test_json_output_format() {
    let output = prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--algorithm",
            "leftflow",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(
        json.get("algorithm").is_some(),
        "JSON should have 'algorithm' field"
    );
    assert!(
        json.get("slices").is_some(),
        "JSON should have 'slices' field (ReviewOutput structure with slice_text)"
    );
    // Verify slice_text is populated
    if let Some(slices) = json.get("slices").and_then(|s| s.as_array()) {
        if let Some(first) = slices.first() {
            let text = first
                .get("slice_text")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            assert!(
                !text.is_empty(),
                "slice_text should contain rendered source code"
            );
        }
    }
}

#[test]
fn test_paper_output_format() {
    let output = prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--algorithm",
            "originaldiff",
            "--format",
            "paper",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Paper format should be valid JSON");
}

#[test]
fn test_review_output_format_single() {
    let output = prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--algorithm",
            "leftflow",
            "--format",
            "review",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Review format should be valid JSON");
    assert!(json.get("algorithm").is_some() || json.get("blocks").is_some());
}

#[test]
fn test_multi_algorithm_text_output() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "leftflow,parentfunction",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== LeftFlow ==="))
        .stdout(predicate::str::contains("=== ParentFunction ==="));
}

#[test]
fn test_multi_algorithm_review_format() {
    let output = prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "leftflow,thin",
            "--format",
            "review",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Multi review JSON");
    assert!(json.get("algorithms_run").is_some());
    assert!(json.get("results").is_some());
}

#[test]
fn test_invalid_chop_source_format_fails() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "chop",
            "--chop-source",
            "no_colon_here",
            "--chop-sink",
            "calc.py:7",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("file:line"));
}
