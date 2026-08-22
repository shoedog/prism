//! CLI end-to-end tests for `--review-no-diagrams` (Item C / P1 residual).
//!
//! `--format review` diagram payloads (`diagrams: Vec<SliceGraph>` at result
//! level and per-finding) dominate compacted review output. This flag strips
//! them from every copy — `results[*]`, `results[*].findings[*]`, AND the
//! top-level `all_findings[*]` aggregate in the multi-algorithm path, which
//! is built independently in `src/main.rs` rather than via
//! `to_compact_review_output` (spec-review delta 1) — while leaving
//! `diagram_warnings` untouched: `finalize_diagrams` (the sole producer of
//! `DiagramWarning`s) still runs unconditionally, so `--strict-diagrams`
//! exit-code semantics are unaffected (delta 2). Non-review formats silently
//! ignore the flag, matching `--review-min-severity` / `--review-full-slices`
//! (delta 3).

use assert_cmd::Command;
use serde_json::Value;

fn prism_cmd() -> Command {
    Command::cargo_bin("prism").unwrap()
}

const REPO: &str = "tests/fixtures/review_no_diagrams";
const DIFF: &str = "tests/fixtures/review_no_diagrams/test.patch";

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/review_no_diagrams/golden/{name}")).unwrap()
}

/// Recursively check whether `value` (or anything nested inside it) has an
/// object key named `key`. Used to prove `diagrams` is gone from EVERY
/// location in the tree without hand-enumerating each path — enumerating
/// paths would silently miss a location a future refactor adds (exactly the
/// bug spec-review delta 1 flagged: a second, independently-built copy).
fn contains_key_recursively(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key(key) || map.values().any(|v| contains_key_recursively(v, key))
        }
        Value::Array(items) => items.iter().any(|v| contains_key_recursively(v, key)),
        _ => false,
    }
}

/// `taint,vertical` on this fixture reliably yields diagrams in all three
/// locations at once: `results[0]` (Taint) has a finding with per-finding
/// diagrams; `results[1]` (VerticalSlice) has result-level diagrams; and the
/// top-level `all_findings` aggregate (built independently in main.rs)
/// carries the same finding-level diagrams as `results[0].findings[*]`.
/// `--review-min-severity info` is needed to retain the taint_source
/// (info-severity) finding alongside the sink.
const MULTI_ALGO_ARGS: [&str; 4] = ["--algorithm", "taint,vertical", "--format", "review"];

#[test]
fn unflagged_multi_review_is_byte_identical_to_golden() {
    let output = prism_cmd()
        .args(["--repo", REPO, "--diff", DIFF])
        .args(MULTI_ALGO_ARGS)
        .args(["--review-min-severity", "info"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        golden("review_unflagged.json"),
        "unflagged --format review output must stay byte-identical to the \
         pre-flag golden — adding --review-no-diagrams must not change \
         default behavior"
    );
}

#[test]
fn sanity_unflagged_output_carries_diagrams_in_all_three_locations() {
    // Precondition for `review_no_diagrams_strips_diagrams_from_every_location`
    // to be a meaningful (non-vacuous) test: without the flag, `diagrams`
    // really does show up in results[*], results[*].findings[*], AND
    // all_findings[*] (the second copy) on this fixture.
    let output = prism_cmd()
        .args(["--repo", REPO, "--diff", DIFF])
        .args(MULTI_ALGO_ARGS)
        .args(["--review-min-severity", "info"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    let results = json["results"].as_array().unwrap();
    assert!(
        results.iter().any(|r| r.get("diagrams").is_some()),
        "sanity: at least one result must carry result-level diagrams \
         (VerticalSlice); results: {results:#?}"
    );
    assert!(
        results.iter().any(|r| r["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f.get("diagrams").is_some())),
        "sanity: at least one finding must carry diagrams (Taint's sink \
         finding); results: {results:#?}"
    );
    let all_findings = json["all_findings"].as_array().unwrap();
    assert!(
        all_findings.iter().any(|f| f.get("diagrams").is_some()),
        "sanity: the top-level all_findings second copy must also carry \
         diagrams by default; all_findings: {all_findings:#?}"
    );
}

#[test]
fn review_no_diagrams_strips_diagrams_from_every_location_in_multi_run() {
    let output = prism_cmd()
        .args(["--repo", REPO, "--diff", DIFF])
        .args(MULTI_ALGO_ARGS)
        .args(["--review-min-severity", "info", "--review-no-diagrams"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(
        !contains_key_recursively(&json, "diagrams"),
        "--review-no-diagrams must strip 'diagrams' from every location \
         (results[*], results[*].findings[*], all_findings[*]); full JSON: {json:#}"
    );

    // Safe-failure-direction check: only the diagram payload is gone —
    // findings and slices must survive untouched.
    let all_findings = json["all_findings"].as_array().unwrap();
    assert!(
        !all_findings.is_empty(),
        "findings must survive --review-no-diagrams"
    );
    let results = json["results"].as_array().unwrap();
    assert!(
        results
            .iter()
            .any(|r| !r["slices"].as_array().unwrap().is_empty()),
        "slices must survive --review-no-diagrams"
    );

    // diagram_warnings must survive at both the top level and per-result.
    assert!(
        json.get("diagram_warnings").is_some(),
        "top-level diagram_warnings must stay in JSON when diagrams are \
         suppressed: {json:#}"
    );
    let taint_result = results
        .iter()
        .find(|r| r["algorithm"] == "Taint")
        .expect("Taint result must be present");
    assert!(
        taint_result.get("diagram_warnings").is_some(),
        "per-result diagram_warnings must also survive: {taint_result:#}"
    );
}

#[test]
fn review_no_diagrams_strips_diagrams_in_single_algorithm_run() {
    let output = prism_cmd()
        .args(["--repo", REPO, "--diff", DIFF])
        .args([
            "--algorithm",
            "taint",
            "--format",
            "review",
            "--review-min-severity",
            "info",
            "--review-no-diagrams",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        !contains_key_recursively(&json, "diagrams"),
        "--review-no-diagrams must strip 'diagrams' in the single-algorithm \
         review path too; full JSON: {json:#}"
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.is_empty(),
        "findings must survive --review-no-diagrams (single-algo path)"
    );
}

#[test]
fn strict_diagrams_exit_and_stderr_unaffected_by_review_no_diagrams() {
    // LabelTruncated (informational, not bug-class) is naturally produced by
    // this fixture's finalize_diagrams pass (the mermaid label for line 3
    // exceeds 80 chars). --strict-diagrams must still exit 0 for it, and the
    // flag must not suppress the stderr emission — proving
    // --review-no-diagrams did not skip finalize_diagrams (delta 2) and did
    // not touch the exit-code/stderr plumbing (both keyed off
    // `result.diagram_warnings`, independent of the CompactReviewOutput this
    // flag edits).
    let output = prism_cmd()
        .args(["--repo", REPO, "--diff", DIFF])
        .args(MULTI_ALGO_ARGS)
        .args([
            "--review-min-severity",
            "info",
            "--review-no-diagrams",
            "--strict-diagrams",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "informational-only warnings must not trip --strict-diagrams; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LabelTruncated"),
        "diagram warning must still be emitted to stderr under \
         --review-no-diagrams: {stderr}"
    );
}

#[test]
fn format_json_ignores_review_no_diagrams_flag() {
    let without_flag = prism_cmd()
        .args(["--repo", REPO, "--diff", DIFF])
        .args(["--algorithm", "taint,vertical", "--format", "json"])
        .output()
        .unwrap();
    let with_flag = prism_cmd()
        .args(["--repo", REPO, "--diff", DIFF])
        .args(["--algorithm", "taint,vertical", "--format", "json"])
        .args(["--review-no-diagrams"])
        .output()
        .unwrap();

    assert!(
        without_flag.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&without_flag.stderr)
    );
    assert!(
        with_flag.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&with_flag.stderr)
    );
    assert_eq!(
        without_flag.status.code(),
        with_flag.status.code(),
        "--format json exit status must be identical with/without \
         --review-no-diagrams"
    );
    assert_eq!(
        without_flag.stdout, with_flag.stdout,
        "--format json stdout must be byte-identical with/without \
         --review-no-diagrams (non-review formats silently ignore the flag)"
    );
}

#[test]
fn review_no_diagrams_flag_is_accepted_by_clap() {
    prism_cmd()
        .args(["--repo", REPO, "--diff", DIFF])
        .args([
            "--algorithm",
            "taint",
            "--format",
            "review",
            "--review-no-diagrams",
        ])
        .assert()
        .success();
}
