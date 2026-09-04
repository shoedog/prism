//! CLI end-to-end tests for the parts of `--format sarif` that are about the
//! RUN rather than the finding mapping: the `--format` allow-list (§2.2.3 /
//! §7.2.5), algorithm-error notifications (§7.2.3), byte determinism (§7.2.4)
//! and the degraded-parse demotion (§7.2.10).
//!
//! Split from `sarif_test.rs` (which owns §7.2.1/§7.2.2 and the in-process
//! document tests) to keep both files well under the repo's 600-line limit.
//! Per repo convention each CLI test file carries its own `prism_cmd` /
//! `write_repo` helpers rather than sharing a module.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn prism_cmd() -> Command {
    Command::cargo_bin("prism").unwrap()
}

/// Write `files` (relative path -> content) into a fresh temp repo, plus a
/// JSON diff spec (the `DiffInput::from_json` shape) marking `diff_lines`
/// per file. Returns `(TempDir, repo_dir, diff_json_path)` — keep the
/// `TempDir` alive for the duration of the test.
fn write_repo(
    files: &[(&str, &str)],
    diff_files: &[(&str, &[usize])],
) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().to_path_buf();
    for (path, content) in files {
        let full = repo.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, content).unwrap();
    }

    let files_json: Vec<Value> = diff_files
        .iter()
        .map(|(path, lines)| {
            serde_json::json!({
                "file_path": path,
                "modify_type": "Modified",
                "diff_lines": lines,
            })
        })
        .collect();
    let spec = serde_json::json!({ "files": files_json });
    let diff_path = repo.join("diff.json");
    fs::write(&diff_path, serde_json::to_string_pretty(&spec).unwrap()).unwrap();

    (tmp, repo, diff_path)
}

/// Fixture A (absence): `a.py` opens a file on line 2 inside `def read():`
/// and never closes it. Diff = `a.py` line 2.
fn fixture_absence() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    write_repo(
        &[("a.py", "def read():\n    f = open(\"x\")\n    return f\n")],
        &[("a.py", &[2])],
    )
}

/// Fixture C (degraded): fixture A with a syntax error line above `read`, so
/// tree-sitter grades `a.py` `degraded` (>1% ERROR nodes).
fn fixture_degraded() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    write_repo(
        &[(
            "a.py",
            "def broken(:\ndef read():\n    f = open(\"x\")\n    return f\n",
        )],
        &[("a.py", &[3])],
    )
}

fn run_sarif(repo: &std::path::Path, diff: &std::path::Path, algorithm: &str) -> (Vec<u8>, Value) {
    let out = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            algorithm,
            "--format",
            "sarif",
        ])
        .output()
        .unwrap();
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}); stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.stdout, doc)
}

fn run(doc: &Value) -> &Value {
    &doc["runs"][0]
}

fn results(doc: &Value) -> &Vec<Value> {
    run(doc)["results"].as_array().unwrap()
}

/// §2.2.1: `results[i].ruleIndex` indexes `results[i].ruleId` in the driver's
/// rule array — asserted for EVERY result, not just the first.
fn assert_rule_index_correspondence(doc: &Value) {
    let rules = run(doc)["tool"]["driver"]["rules"].as_array().unwrap();
    for r in results(doc) {
        let idx = r["ruleIndex"].as_u64().unwrap() as usize;
        assert_eq!(
            rules[idx]["id"], r["ruleId"],
            "rules[{idx}].id must equal result.ruleId {:?}",
            r["ruleId"]
        );
    }
}

/// §7.2.3
#[test]
fn algorithm_error_becomes_notification_and_properties_error() {
    let (_tmp, repo, diff) = fixture_absence();
    let (_bytes, doc) = run_sarif(&repo, &diff, "chop,absence");

    let inv = &run(&doc)["invocations"][0];
    assert_eq!(inv["executionSuccessful"], false);
    let notes = inv["toolExecutionNotifications"].as_array().unwrap();
    let errors: Vec<&Value> = notes.iter().filter(|n| n["level"] == "error").collect();
    assert_eq!(errors.len(), 1, "one notification per AlgorithmError");
    let text = errors[0]["message"]["text"].as_str().unwrap();
    assert!(
        text.contains("--chop-source required"),
        "notification keeps the error text: {text}"
    );
    assert!(
        text.starts_with("Chop: "),
        "notification is '{{algorithm}}: {{error}}': {text}"
    );

    assert_eq!(run(&doc)["properties"]["errors"][0]["algorithm"], "Chop");
    assert!(run(&doc)["properties"]["errors"][0]["error"]
        .as_str()
        .unwrap()
        .contains("--chop-source required"));

    assert!(
        results(&doc)
            .iter()
            .any(|r| r["ruleId"] == "prism/absence/missing_counterpart"),
        "a failing algorithm must not suppress the ones that succeeded"
    );
}

/// §7.2.4
#[test]
fn sarif_is_byte_deterministic() {
    let (_tmp, repo, diff) = fixture_absence();
    let (first, _) = run_sarif(&repo, &diff, "absence");
    let (second, _) = run_sarif(&repo, &diff, "absence");
    assert_eq!(
        first, second,
        "two runs over identical input must be byte-identical"
    );
    assert!(first.ends_with(b"\n"), "document ends with a newline");
}

/// §7.2.5 — the `--format` allow-list (§2.2.3). Values that previously fell
/// through to the text renderer are now clap errors.
///
/// Divergence from the spec's literal wording: clap reports an EMPTY value on
/// a possible-values argument as "a value is required ... but none was
/// supplied", not "invalid value" — the same message the pre-existing
/// `--review-min-severity` allow-list produces, so it is clap's behaviour, not
/// this argument's configuration. The substantive observable (exit 2, nothing
/// rendered, the allow-list shown) holds for all three values.
#[test]
fn unknown_format_values_are_rejected() {
    let (_tmp, repo, diff) = fixture_absence();
    let reject = |bad: &str| -> (Option<i32>, String, Vec<u8>) {
        let out = prism_cmd()
            .args([
                "--repo",
                repo.to_str().unwrap(),
                "--diff",
                diff.to_str().unwrap(),
                "--algorithm",
                "absence",
                "--format",
                bad,
            ])
            .output()
            .unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).to_string(),
            out.stdout,
        )
    };

    for bad in ["bogus", "Json", "txt", "sarif2"] {
        let (code, stderr, stdout) = reject(bad);
        assert_eq!(code, Some(2), "--format {bad:?} must be a clap parse error");
        assert!(
            stderr.contains("invalid value"),
            "--format {bad:?} stderr must say 'invalid value': {stderr}"
        );
        assert!(stdout.is_empty(), "--format {bad:?} must render nothing");
    }

    let (code, stderr, stdout) = reject("");
    assert_eq!(code, Some(2), "--format \"\" must be a clap parse error");
    assert!(
        stderr.contains("a value is required"),
        "--format \"\" is rejected by clap's empty-value rule: {stderr}"
    );
    assert!(stdout.is_empty(), "--format \"\" must render nothing");

    // Every rejection prints the allow-list, so the user sees the fix.
    for bad in ["bogus", ""] {
        let (_, stderr, _) = reject(bad);
        assert!(
            stderr.contains("possible values: text, json, paper, review, callers, mermaid, sarif"),
            "--format {bad:?} must show the allow-list: {stderr}"
        );
    }
}

/// Every allow-listed value still parses (the allow-list must not narrow the
/// supported set).
#[test]
fn allow_listed_format_values_are_accepted() {
    let (_tmp, repo, diff) = fixture_absence();
    for good in [
        "text", "json", "paper", "review", "callers", "mermaid", "sarif",
    ] {
        let out = prism_cmd()
            .args([
                "--repo",
                repo.to_str().unwrap(),
                "--diff",
                diff.to_str().unwrap(),
                "--algorithm",
                "absence",
                "--format",
                good,
            ])
            .output()
            .unwrap();
        assert_ne!(
            out.status.code(),
            Some(2),
            "--format {good} must not be a parse error: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// §7.2.10
#[test]
fn degraded_parse_demotes_to_candidate() {
    let (_tmp, repo, diff) = fixture_degraded();
    let (_bytes, doc) = run_sarif(&repo, &diff, "absence");

    let results = results(&doc);
    assert!(
        !results.is_empty(),
        "absence still fires on a degraded parse"
    );
    for r in results {
        assert_eq!(r["ruleId"], "prism/absence/missing_counterpart");
        assert_eq!(
            r["properties"]["parse_quality"], "degraded",
            "the sparse map's grade reaches SARIF"
        );
        assert_eq!(
            r["properties"]["tier"], "candidate",
            "a non-clean evidence file demotes the tier (§5.1)"
        );
        assert_eq!(
            r["properties"]["confidence"], "exact",
            "confidence is the algorithm's evidence kind, unchanged by parse quality"
        );
    }
    assert_rule_index_correspondence(&doc);
}
