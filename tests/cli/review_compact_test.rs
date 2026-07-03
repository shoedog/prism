//! CLI end-to-end tests for the `--format review` output collapse (Task P1).
//!
//! Covers:
//!   1. Taint seeding skips diff files prism can't parse (Change 1).
//!   2. `taint_source` findings are only emitted for sources tied to an
//!      emitted sink finding (Change 2).
//!   3. The compact review-only serialization: severity floor, dropped
//!      `slice_lines`/`diff_lines`, block retention (Change 3).
//!   4. `--format json` keeps the old (uncompacted) shape.

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

/// `a.py`: `x` (line 3) flows to the `os.system(x)` sink (line 4) — a real
/// source-reaches-sink pair.
/// `b.py`: `y` (line 2) is diff-seeded taint but flows only to `return y`
/// (line 3) — never reaches a sink.
fn write_taint_repo() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    write_repo(
        &[
            (
                "a.py",
                "import os\ndef handle(req):\n    x = req.GET['x']\n    os.system(x)\n",
            ),
            (
                "b.py",
                "def compute(req):\n    y = req.GET['y']\n    return y\n",
            ),
        ],
        &[("a.py", &[3, 4]), ("b.py", &[2])],
    )
}

/// Same as `write_taint_repo`, plus `c.py` whose `WEAK_HASH_FOR_IDENTITY`
/// (PrimitiveSlice) finding sits outside the diff-touched function, so it is
/// emitted at `suggestion` severity.
fn write_taint_and_suggestion_repo() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    write_repo(
        &[
            (
                "a.py",
                "import os\ndef handle(req):\n    x = req.GET['x']\n    os.system(x)\n",
            ),
            (
                "b.py",
                "def compute(req):\n    y = req.GET['y']\n    return y\n",
            ),
            (
                "c.py",
                "import hashlib\n\ndef cache_helper():\n    cache_id = hashlib.md5(b\"x\").hexdigest()\n    return cache_id\n\ndef touched():\n    z = 1\n    return z\n",
            ),
        ],
        &[("a.py", &[3, 4]), ("b.py", &[2]), ("c.py", &[8])],
    )
}

// --- Change 1: skip unparseable seed files ---------------------------------

#[test]
fn unparseable_seed_file_yields_no_taint_source_finding() {
    let (_tmp, repo, diff) = write_repo(
        &[
            ("main.py", "def f(x):\n    y = x\n    return y\n"),
            (
                "Cargo.toml",
                "[package]\nname = \"old\"\nversion = \"0.1.0\"\n",
            ),
        ],
        &[("main.py", &[2]), ("Cargo.toml", &[2])],
    );

    let output = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            "taint",
            "--format",
            "review",
            "--review-min-severity",
            "info",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["category"] == "taint_source" && f["file"] == "Cargo.toml"),
        "unparseable Cargo.toml must not seed a taint_source finding even at info floor; \
         findings: {findings:#?}"
    );
}

// --- Change 2: per-source findings gated on an emitted sink ----------------

#[test]
fn taint_source_only_emitted_when_it_reaches_a_sink() {
    let (_tmp, repo, diff) = write_taint_repo();

    let output = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            "taint",
            "--format",
            "review",
            "--review-min-severity",
            "info",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    assert!(
        findings
            .iter()
            .any(|f| f["category"] == "taint_source" && f["file"] == "a.py" && f["line"] == 3),
        "a.py's source (line 3) reaches the sink and must have a taint_source finding; \
         findings: {findings:#?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f["category"] == "taint_sink" && f["file"] == "a.py"),
        "a.py's sink must still be reported; findings: {findings:#?}"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f["category"] == "taint_source" && f["file"] == "b.py"),
        "b.py's source reaches nothing and must contribute no taint_source finding; \
         findings: {findings:#?}"
    );
}

// --- Change 3: severity floor -----------------------------------------------

#[test]
fn default_format_review_hides_info_and_suggestion_findings() {
    let (_tmp, repo, diff) = write_taint_and_suggestion_repo();

    let output = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            "primitive,taint",
            "--format",
            "review",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let all_findings = json["all_findings"].as_array().unwrap();

    assert!(
        !all_findings.is_empty(),
        "sanity: the taint_sink (warning) finding should survive the default floor"
    );
    assert!(
        all_findings
            .iter()
            .all(|f| f["severity"] != "info" && f["severity"] != "suggestion"),
        "default --format review must hide info/suggestion findings; all_findings: {all_findings:#?}"
    );
}

#[test]
fn review_min_severity_info_restores_info_and_suggestion_findings() {
    let (_tmp, repo, diff) = write_taint_and_suggestion_repo();

    let output = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            "primitive,taint",
            "--format",
            "review",
            "--review-min-severity",
            "info",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let all_findings = json["all_findings"].as_array().unwrap();

    assert!(
        all_findings.iter().any(|f| f["severity"] == "info"),
        "--review-min-severity info must restore info findings; all_findings: {all_findings:#?}"
    );
    assert!(
        all_findings.iter().any(|f| f["severity"] == "suggestion"),
        "--review-min-severity info must restore suggestion findings; \
         all_findings: {all_findings:#?}"
    );
}

// --- Change 3: dropped slice_lines/diff_lines + block retention ------------

#[test]
fn default_review_json_omits_slice_lines_and_diff_lines_keys() {
    let (_tmp, repo, diff) = write_taint_repo();

    let output = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            "taint",
            "--format",
            "review",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let slices = json["slices"].as_array().unwrap();
    assert!(
        !slices.is_empty(),
        "sanity: at least one block must survive"
    );
    for block in slices {
        assert!(
            block.get("slice_lines").is_none(),
            "compact review blocks must omit slice_lines: {block:#?}"
        );
        assert!(
            block.get("diff_lines").is_none(),
            "compact review blocks must omit diff_lines: {block:#?}"
        );
        assert!(
            block.get("slice_text").is_some(),
            "compact review blocks must keep slice_text: {block:#?}"
        );
    }
}

#[test]
fn default_review_omits_finding_less_blocks() {
    let (_tmp, repo, diff) = write_taint_repo();

    let output = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            "taint",
            "--format",
            "review",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let slices = json["slices"].as_array().unwrap();

    assert!(
        slices.iter().any(|b| b["file"] == "a.py"),
        "a.py's block carries the retained sink finding and must survive: {slices:#?}"
    );
    assert!(
        !slices.iter().any(|b| b["file"] == "b.py"),
        "b.py has no >= warning finding and must be dropped by default: {slices:#?}"
    );
}

#[test]
fn review_full_slices_restores_finding_less_blocks() {
    let (_tmp, repo, diff) = write_taint_repo();

    let output = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            "taint",
            "--format",
            "review",
            "--review-full-slices",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let slices = json["slices"].as_array().unwrap();

    assert!(slices.iter().any(|b| b["file"] == "a.py"));
    assert!(
        slices.iter().any(|b| b["file"] == "b.py"),
        "--review-full-slices must keep finding-less blocks too: {slices:#?}"
    );
}

// --- --format json is untouched by the compact review path -----------------

#[test]
fn format_json_retains_slice_lines_and_diff_lines_keys() {
    let (_tmp, repo, diff) = write_taint_repo();

    let output = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            "taint",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let slices = json["slices"].as_array().unwrap();
    assert!(!slices.is_empty());
    assert!(
        slices
            .iter()
            .all(|b| b.get("slice_lines").is_some() && b.get("diff_lines").is_some()),
        "--format json must keep the old ReviewBlock shape: {slices:#?}"
    );
    // --format json must retain ALL blocks regardless of severity floor
    // defaults (json has no severity floor / block retention at all).
    assert!(
        slices.iter().any(|b| b["file"] == "b.py"),
        "--format json must not drop finding-less blocks: {slices:#?}"
    );
}
