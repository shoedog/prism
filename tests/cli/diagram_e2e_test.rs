/// End-to-end CLI tests that invoke the `prism` binary and assert that
/// diagram fields are populated in JSON/mermaid output.
///
/// These tests exercise the full dispatch path — unlike unit tests that call
/// `run_slicing_compat` directly, these catch regressions in the CLI's
/// `run_algorithm` dispatcher (e.g., missing `finalize_diagrams` calls,
/// `ReviewOutput` omitting diagrams, etc.).
///
/// JSON structure for a single-algorithm run:
///   { "algorithm": "Taint", "slices": [...], "findings": [...], "diagrams": [...] }
/// For a multi-algorithm run (comma-list or preset):
///   { "version": "...", "results": [ { "algorithm": ..., ... }, ... ] }
use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

/// Write a Django-style Python `views.py` fixture into a temp directory.
/// `request.GET["q"]` is a recognised taint source; `cursor.execute(...)` with
/// an f-string is a recognised SQL sink.  The patch touches line 2 (the
/// `request.GET` assignment) so the entire function is in the diff context.
///
/// Returns `(TempDir, repo_root, patch_path)`.  Keep `TempDir` alive for the
/// duration of the test so the temp directory is not deleted.
fn write_django_repo() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().to_path_buf();

    // views.py — Django-style view with request.GET source → cursor.execute sink.
    fs::write(
        repo.join("views.py"),
        "def lookup_view(request):\n\
         \tq = request.GET[\"q\"]\n\
         \tcursor.execute(f\"SELECT * FROM users WHERE name = '{q}'\")\n",
    )
    .unwrap();

    // Unified diff — touches line 2 (request.GET assignment).
    let patch = repo.join("test.patch");
    let patch_text = "diff --git a/views.py b/views.py\n\
index 0000001..0000002 100644\n\
--- a/views.py\n\
+++ b/views.py\n\
@@ -1,3 +1,3 @@\n\
 def lookup_view(request):\n\
-\tq = request.GET[\"q\"]\n\
+\tq = request.GET[\"query\"]\n\
 \tcursor.execute(f\"SELECT * FROM users WHERE name = '{q}'\")\n";
    fs::write(&patch, patch_text).unwrap();

    (tmp, repo, patch)
}

/// Run prism with `--format json` on a single algorithm and return the parsed
/// JSON.  Single-algorithm runs emit a flat `SliceResult` object (not
/// `MultiSliceResult`), so the top-level keys are `algorithm`, `slices`,
/// `findings`, and `diagrams`.
fn run_prism_single_json(repo: &std::path::Path, patch: &std::path::Path, algo: &str) -> Value {
    let output = Command::cargo_bin("prism")
        .unwrap()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            patch.to_str().unwrap(),
            "--algorithm",
            algo,
            "--format",
            "json",
        ])
        .output()
        .expect("prism binary failed to launch");

    assert!(
        output.status.success(),
        "prism exited non-zero for algo '{}': stderr={}",
        algo,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("non-UTF-8 stdout");
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "prism produced non-JSON for algo '{}': {} — stdout: {}",
            algo, e, stdout
        )
    })
}

/// Taint JSON output must include per-finding `diagrams` arrays that are
/// non-empty when the algorithm detects a source→sink path.
///
/// The Django request.GET → cursor.execute pattern reliably triggers taint.
#[test]
fn cli_taint_json_includes_per_finding_diagrams() {
    let (_tmp, repo, patch) = write_django_repo();
    let v = run_prism_single_json(&repo, &patch, "taint");

    // Single-algo output: top-level "findings" array.
    let findings = v["findings"]
        .as_array()
        .expect("expected top-level 'findings' array");

    assert!(
        !findings.is_empty(),
        "Taint should produce at least one finding for the Django request.GET → SQL sink path; \
         full JSON: {}",
        v
    );

    let any_diagram = findings.iter().any(|f| {
        f["diagrams"]
            .as_array()
            .map(|d| !d.is_empty())
            .unwrap_or(false)
    });

    assert!(
        any_diagram,
        "At least one Taint finding should carry a non-empty 'diagrams' array; \
         findings: {:?}",
        findings
    );
}

/// VerticalSlice JSON output must include result-level `diagrams` when slices
/// (blocks) are present.
///
/// Single-algo output uses `"slices"` (not `"blocks"`) as the key.
#[test]
fn cli_vertical_json_includes_result_level_diagrams() {
    let (_tmp, repo, patch) = write_django_repo();
    let v = run_prism_single_json(&repo, &patch, "vertical");

    // Single-algo: top-level "slices" array for blocks, "diagrams" for graphs.
    let slices = v["slices"]
        .as_array()
        .expect("expected top-level 'slices' array");

    if !slices.is_empty() {
        // When blocks exist a diagram must have been built and finalised.
        let diagrams = v["diagrams"]
            .as_array()
            .map(|d| d.as_slice())
            .unwrap_or(&[]);
        assert!(
            !diagrams.is_empty(),
            "VerticalSlice produced slices but no diagrams — \
             finalize_diagrams was not called or diagram builder is broken; \
             full JSON: {}",
            v
        );
    }
    // If no slices, diagrams are expected to be absent — that is acceptable.
}

/// EchoSlice JSON output must include result-level `diagrams` when findings
/// exist.  This fixture (single-function Django view) is not guaranteed to
/// trigger Echo, so the assertion is conditional.
#[test]
fn cli_echo_json_includes_result_level_diagrams_when_findings_exist() {
    let (_tmp, repo, patch) = write_django_repo();
    let v = run_prism_single_json(&repo, &patch, "echo");

    // Single-algo: top-level "findings".
    let findings = v["findings"]
        .as_array()
        .map(|f| f.as_slice())
        .unwrap_or(&[]);

    if !findings.is_empty() {
        let diagrams = v["diagrams"]
            .as_array()
            .map(|d| d.as_slice())
            .unwrap_or(&[]);
        assert!(
            !diagrams.is_empty(),
            "EchoSlice produced findings but no result-level diagrams — \
             diagram builder is broken; full JSON: {}",
            v
        );
    }
}

/// `--format mermaid` output must start with the expected report header and,
/// for the Django fixture that triggers a taint finding, must include at least
/// one `flowchart` block.
#[test]
fn cli_mermaid_format_emits_flowchart_for_taint() {
    let (_tmp, repo, patch) = write_django_repo();

    let output = Command::cargo_bin("prism")
        .unwrap()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            patch.to_str().unwrap(),
            "--algorithm",
            "taint",
            "--format",
            "mermaid",
        ])
        .output()
        .expect("prism binary failed to launch");

    assert!(
        output.status.success(),
        "prism exited non-zero: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("non-UTF-8 stdout");

    assert!(
        stdout.starts_with("# Prism diagram report"),
        "mermaid output should start with '# Prism diagram report'; got: {}",
        &stdout[..stdout.len().min(120)]
    );

    // The Django fixture reliably triggers a taint finding with a Chain diagram.
    assert!(
        stdout.contains("flowchart"),
        "mermaid output should contain a 'flowchart' block for the taint finding; \
         got:\n{}",
        stdout
    );
}
