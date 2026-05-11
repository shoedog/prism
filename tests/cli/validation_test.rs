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
fn test_version_flag_reports_package_version() {
    prism_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_relevant_slice_cli() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "relevant",
        ])
        .assert()
        .success();
}

#[test]
fn test_circular_slice_cli() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "circular",
        ])
        .assert()
        .success();
}

#[test]
fn test_absence_slice_cli() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "absence",
        ])
        .assert()
        .success();
}

#[test]
fn test_symmetry_slice_cli() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "symmetry",
        ])
        .assert()
        .success();
}

#[test]
fn test_gradient_slice_cli() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "gradient",
        ])
        .assert()
        .success();
}

#[test]
fn test_provenance_slice_cli() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "provenance",
        ])
        .assert()
        .success();
}

#[test]
fn test_membrane_slice_cli() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "membrane",
        ])
        .assert()
        .success();
}

#[test]
fn test_echo_slice_cli() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "echo",
        ])
        .assert()
        .success();
}

#[test]
fn test_delta_slice_with_old_repo() {
    let tmp = TempDir::new().unwrap();
    let old_calc = tmp.path().join("calc.py");
    fs::write(
        &old_calc,
        "def multiply(x, y):\n    product = x * y\n    return product\n",
    )
    .unwrap();

    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "delta",
            "--old-repo",
            &tmp.path().to_string_lossy(),
        ])
        .assert()
        .success();
}

#[test]
fn test_delta_missing_old_repo_fails() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "delta",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("old-repo"));
}

#[test]
fn test_threed_slice_cli() {
    // 3d needs git history — use the repo itself as both --repo and git dir
    // Create a JSON diff referencing a file that exists at the repo root
    let tmp = TempDir::new().unwrap();
    let diff_json = tmp.path().join("diff.json");
    fs::write(
        &diff_json,
        r#"{"files": [{"file_path": "src/main.rs", "modify_type": "Modified", "diff_lines": [1, 2]}]}"#,
    ).unwrap();

    prism_cmd()
        .args([
            "--repo",
            ".",
            "--diff",
            &diff_json.to_string_lossy(),
            "--algorithm",
            "3d",
            "--temporal-days",
            "30",
        ])
        .assert()
        .success();
}

#[test]
fn test_resonance_slice_cli() {
    let tmp = TempDir::new().unwrap();
    let diff_json = tmp.path().join("diff.json");
    fs::write(
        &diff_json,
        r#"{"files": [{"file_path": "src/main.rs", "modify_type": "Modified", "diff_lines": [1, 2]}]}"#,
    ).unwrap();

    prism_cmd()
        .args([
            "--repo",
            ".",
            "--diff",
            &diff_json.to_string_lossy(),
            "--algorithm",
            "resonance",
            "--temporal-days",
            "30",
        ])
        .assert()
        .success();
}

#[test]
fn test_phantom_slice_cli() {
    let tmp = TempDir::new().unwrap();
    let diff_json = tmp.path().join("diff.json");
    fs::write(
        &diff_json,
        r#"{"files": [{"file_path": "src/main.rs", "modify_type": "Modified", "diff_lines": [1, 2]}]}"#,
    ).unwrap();

    prism_cmd()
        .args([
            "--repo",
            ".",
            "--diff",
            &diff_json.to_string_lossy(),
            "--algorithm",
            "phantom",
        ])
        .assert()
        .success();
}

#[test]
fn test_max_branch_lines_flag() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "leftflow",
            "--max-branch-lines",
            "10",
        ])
        .assert()
        .success();
}

#[test]
fn test_no_returns_flag() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "leftflow",
            "--no-returns",
        ])
        .assert()
        .success();
}

#[test]
fn test_no_trace_callees_flag() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "fullflow",
            "--no-trace-callees",
        ])
        .assert()
        .success();
}

#[test]
fn test_files_filter() {
    // Use a multi-file diff fixture if available; with single-file, just verify it works
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "leftflow",
            "--files",
            "calc.py",
        ])
        .assert()
        .success();
}

#[test]
fn test_files_filter_nonexistent_file() {
    // Filtering to a file not in the diff should produce empty output
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "leftflow",
            "--files",
            "nonexistent.py",
        ])
        .assert()
        .success();
}

#[test]
fn test_json_diff_input() {
    let tmp = TempDir::new().unwrap();
    let json_diff = tmp.path().join("diff.json");
    fs::write(
        &json_diff,
        r#"{"files": [{"file_path": "calc.py", "modify_type": "Modified", "diff_lines": [6]}]}"#,
    )
    .unwrap();

    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &json_diff.to_string_lossy(),
            "--algorithm",
            "leftflow",
        ])
        .assert()
        .success();
}

#[test]
fn test_unknown_algorithm_fails() {
    prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--algorithm",
            "nonexistent",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown algorithm"));
}

#[test]
fn test_unknown_in_comma_list_fails() {
    prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--algorithm",
            "leftflow,bogus",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown algorithm"));
}

#[test]
fn test_missing_repo_fails() {
    prism_cmd()
        .args(["--diff", &fixture_path("c/timer_uaf.diff")])
        .assert()
        .failure();
}

#[test]
fn test_missing_diff_fails() {
    prism_cmd().args(["--repo", "."]).assert().failure();
}

#[test]
fn test_nonexistent_diff_file_fails() {
    prism_cmd()
        .args(["--repo", &fixture_path("c"), "--diff", "nonexistent.diff"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read diff"));
}

#[test]
fn test_all_algorithms_json() {
    // "all" runs every algorithm; some may fail (e.g. delta needs --old-repo),
    // but the multi-run captures errors gracefully
    let output = prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "all",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("all suite JSON");
    let algos = json
        .get("algorithms_run")
        .expect("Should have algorithms_run");
    assert!(
        algos.as_array().unwrap().len() > 20,
        "All suite should list 26 algorithms"
    );
}

#[test]
fn test_unsupported_language_warns() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("data.csv");
    fs::write(&src, "a,b,c\n1,2,3\n").unwrap();

    let diff_json = tmp.path().join("diff.json");
    fs::write(
        &diff_json,
        r#"{"files": [{"file_path": "data.csv", "modify_type": "Modified", "diff_lines": [2]}]}"#,
    )
    .unwrap();

    // When all files are unsupported, prism warns on stderr and produces empty output
    prism_cmd()
        .args([
            "--repo",
            &tmp.path().to_string_lossy(),
            "--diff",
            &diff_json.to_string_lossy(),
        ])
        .assert()
        .stderr(predicate::str::contains("unsupported language"));
}

#[test]
fn test_compile_commands_nonexistent_file_warns() {
    // When --compile-commands points to a non-existent file, prism should warn
    // on stderr but still produce output (graceful degradation).
    prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--compile-commands",
            "/nonexistent/compile_commands.json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("failed to load type database"));
}

#[test]
fn test_compile_commands_invalid_json_warns() {
    let tmp = TempDir::new().unwrap();
    let cc_path = tmp.path().join("compile_commands.json");
    fs::write(&cc_path, "not valid json").unwrap();

    prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--compile-commands",
            &cc_path.to_string_lossy(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("failed to load type database"));
}

#[test]
fn test_compile_commands_empty_array_succeeds() {
    let tmp = TempDir::new().unwrap();
    let cc_path = tmp.path().join("compile_commands.json");
    fs::write(&cc_path, "[]").unwrap();

    prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--compile-commands",
            &cc_path.to_string_lossy(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Type enrichment: 0 records, 0 typedefs",
        ));
}

/// Write a small Django-style `views.py` fixture into a temp directory.
/// request.GET["q"] → cursor.execute produces a taint finding with a diagram.
/// Returns (TempDir, repo_root, patch_path) — keep TempDir alive for the test.
fn write_small_fixture() -> (TempDir, String, String) {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().to_path_buf();
    std::fs::write(
        repo.join("views.py"),
        "def lookup_view(request):\n\
         \tq = request.GET[\"q\"]\n\
         \tcursor.execute(f\"SELECT * FROM users WHERE name = '{q}'\")\n",
    )
    .unwrap();
    let patch = repo.join("test.patch");
    std::fs::write(
        &patch,
        "diff --git a/views.py b/views.py\n\
         index 0000001..0000002 100644\n\
         --- a/views.py\n\
         +++ b/views.py\n\
         @@ -1,3 +1,3 @@\n\
          def lookup_view(request):\n\
         -\tq = request.GET[\"q\"]\n\
         +\tq = request.GET[\"query\"]\n\
          \tcursor.execute(f\"SELECT * FROM users WHERE name = '{q}'\")\n",
    )
    .unwrap();
    let repo_str = repo.to_str().unwrap().to_string();
    let patch_str = patch.to_str().unwrap().to_string();
    (tmp, repo_str, patch_str)
}

#[test]
fn cli_accepts_mermaid_format() {
    // Verify that --format mermaid is accepted by clap AND that the actual output
    // starts with the expected Prism report header.
    let (_tmp, repo, patch) = write_small_fixture();
    let output = prism_cmd()
        .args([
            "--repo",
            &repo,
            "--diff",
            &patch,
            "--algorithm",
            "taint",
            "--format",
            "mermaid",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "prism should exit 0 for --format mermaid"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("# Prism diagram report"),
        "--format mermaid should produce a '# Prism diagram report' header; got: {}",
        &stdout[..stdout.len().min(120)]
    );
}

#[test]
fn cli_diagram_node_cap_parses() {
    // Verify --diagram-node-cap is accepted for valid values (>= 4) and rejected
    // for values below the minimum.
    //
    // The minimum cap of 4 is the smallest that can accommodate head + ghost + tail
    // with at least one real node on each side.  Values < 4 are rejected by the
    // CLI parser (clap value_parser) with a clear error message.
    let (_tmp, repo, patch) = write_small_fixture();

    // (a) Values below the minimum must cause clap to reject the command (exit non-zero).
    let output = prism_cmd()
        .args([
            "--repo",
            &repo,
            "--diff",
            &patch,
            "--algorithm",
            "taint",
            "--diagram-node-cap",
            "3",
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--diagram-node-cap 3 should be rejected (exit non-zero); \
         stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("4") || stderr.contains("must be"),
        "--diagram-node-cap 3 rejection message should mention the minimum; got: {}",
        stderr
    );

    // (b) The minimum valid value (4) must be accepted (exit 0).
    let output = prism_cmd()
        .args([
            "--repo",
            &repo,
            "--diff",
            &patch,
            "--algorithm",
            "taint",
            "--diagram-node-cap",
            "4",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "--diagram-node-cap 4 should be accepted (exit 0); \
         stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_strict_diagrams_flag_parses() {
    // Verify --strict-diagrams is accepted by clap AND that a clean run exits 0
    // (the Django fixture has no bug-class diagram warnings at default cap).
    let (_tmp, repo, patch) = write_small_fixture();
    prism_cmd()
        .args([
            "--repo",
            &repo,
            "--diff",
            &patch,
            "--algorithm",
            "taint",
            "--strict-diagrams",
        ])
        .assert()
        .success();
}
