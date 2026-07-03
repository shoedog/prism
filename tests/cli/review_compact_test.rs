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

    // The `findings` assertion above passes even if Change 1 (skip
    // unparseable seed files at seeding time) is reverted, because Change 2
    // independently gates the `taint_source` finding on reaching an emitted
    // sink — and Cargo.toml's untraceable seed never reaches one either way.
    // Empirically verified by temporarily reverting the Change-1 skip: the
    // discriminating observable is that an untraced Cargo.toml line still
    // becomes its own (empty) block in `all_tainted`/`result.blocks`, which
    // `--format json` (uncompacted, no block-retention filtering) surfaces
    // as a `slices` entry with `file == "Cargo.toml"`. With Change 1 present
    // (this branch), Cargo.toml is never added to `taint_seeds`, so it never
    // reaches `all_tainted` and no such block is ever created; reverting the
    // skip locally reintroduces a `{"file": "Cargo.toml", "slice_lines": [],
    // "slice_text": ""}` entry in `slices`. Pin that here too so this test
    // actually fails if Change 1 regresses.
    let json_output = prism_cmd()
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
        json_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let json_value: Value = serde_json::from_slice(&json_output.stdout).unwrap();
    let slices = json_value["slices"].as_array().unwrap();
    assert!(
        !slices.iter().any(|b| b["file"] == "Cargo.toml"),
        "unparseable Cargo.toml must not seed any block at all (Change 1); \
         slices: {slices:#?}"
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

// --- Review-fix wave F1: cross-file path sources are licensed too ----------

/// `handler.c` line 3 (`char *data = input;`) is the diff-seeded source;
/// `log_message(data)` forwards it to `logger.c`'s `printf(msg)` sink — a
/// genuine cross-file FlowPath. Before the F1 fix, `sink_to_path_sources`
/// recorded the cross-file source but the `taint_source` licensing only
/// considered the same-file "chosen" source, so the real source's finding
/// was silently dropped even though it reached an emitted sink.
#[test]
fn cross_file_path_source_gets_taint_source_finding() {
    let (_tmp, repo, diff) = write_repo(
        &[
            (
                "handler.c",
                "\nvoid process_request(const char *input) {\n    char *data = input;\n    log_message(data);\n}\n",
            ),
            (
                "logger.c",
                "\nvoid log_message(const char *msg) {\n    printf(msg);\n}\n",
            ),
        ],
        &[("handler.c", &[3]), ("logger.c", &[])],
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
        findings
            .iter()
            .any(|f| f["category"] == "taint_sink" && f["file"] == "logger.c" && f["line"] == 3),
        "logger.c's sink must be reported; findings: {findings:#?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f["category"] == "taint_source" && f["file"] == "handler.c" && f["line"] == 3),
        "handler.c's cross-file source reaches logger.c's sink and must have \
         a taint_source finding; findings: {findings:#?}"
    );
}

// --- Review-fix wave F2: bash unquoted-expansion licenses its source -------

/// `script.sh` line 4 (`local file="$1"`) is the diff-seeded source; it
/// flows to the unquoted `cat $file` on line 6, whose only sink-style
/// finding is `unquoted_expansion` (bash-specific, emitted after the sink
/// loop). Before the F2 fix, the gated source-emission loop ran BEFORE the
/// unquoted-expansion block, so it never saw this licensing and silently
/// dropped the source's `taint_source` finding.
#[test]
fn unquoted_expansion_licenses_its_reaching_source() {
    let (_tmp, repo, diff) = write_repo(
        &[(
            "script.sh",
            "#!/bin/bash\n\nprocess() {\n    local file=\"$1\"\n    echo \"processing\"\n    cat $file\n}\n\nprocess \"$@\"\n",
        )],
        &[("script.sh", &[4])],
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
        findings
            .iter()
            .any(|f| f["category"] == "unquoted_expansion" && f["file"] == "script.sh"),
        "sanity: the unquoted `cat $file` must still be flagged; findings: {findings:#?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f["category"] == "taint_source" && f["file"] == "script.sh" && f["line"] == 4),
        "the seed line reaching the unquoted-expansion finding must have a \
         taint_source finding; findings: {findings:#?}"
    );
}

/// Same as above but the variable is quoted, so no unquoted-expansion
/// finding (or any other sink-style finding) fires — the source reaches
/// nothing and must contribute no `taint_source` finding.
#[test]
fn quoted_expansion_source_gets_no_taint_source_finding() {
    let (_tmp, repo, diff) = write_repo(
        &[(
            "script.sh",
            "#!/bin/bash\n\nprocess() {\n    local file=\"$1\"\n    echo \"processing\"\n    cat \"$file\"\n}\n\nprocess \"$@\"\n",
        )],
        &[("script.sh", &[4])],
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
            .any(|f| f["category"] == "unquoted_expansion"),
        "sanity: the quoted `cat \"$file\"` must not be flagged; findings: {findings:#?}"
    );
    assert!(
        !findings.iter().any(|f| f["category"] == "taint_source"),
        "the source reaches nothing (properly quoted) and must contribute no \
         taint_source finding; findings: {findings:#?}"
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

    // The severity floor must ALSO apply to each per-result `findings` array
    // (`CompactReviewOutput::findings`), not just the aggregated
    // `all_findings` (F5 coverage: `to_compact_review_output` filters
    // `result.findings` independently of the `all_findings` aggregation in
    // main.rs, and a regression in one need not show up in the other).
    let results = json["results"].as_array().unwrap();
    assert!(
        !results.is_empty(),
        "sanity: at least one algorithm result must be present"
    );
    let mut saw_any_per_result_finding = false;
    for result in results {
        let findings = result["findings"].as_array().unwrap();
        saw_any_per_result_finding |= !findings.is_empty();
        assert!(
            findings
                .iter()
                .all(|f| f["severity"] != "info" && f["severity"] != "suggestion"),
            "per-result findings must also be floored at the default severity: {findings:#?}"
        );
    }
    assert!(
        saw_any_per_result_finding,
        "sanity: at least one result must retain a >= warning finding"
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
