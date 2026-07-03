use assert_cmd::Command;

/// Trivial same-function source->sink: `user` (line 2) flows into `value` and
/// reaches `sink(value)` (line 4) within one function, so `reasoning.reachability`
/// must be `"Reached"` in `--format json` output. Mirrors the library-level
/// fixture in tests/reasoning/taint_reaches_test.rs.
#[test]
fn taint_reaches_reports_reachability_in_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.py"),
        "def f():\n    user = input()\n    value = user\n    sink(value)\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "taint-reaches", "--repo"])
        .arg(dir.path())
        .args([
            "--source", "app.py:2", "--sink", "app.py:4", "--format", "json",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["reasoning"]["reachability"], "Reached");
}

/// Frontier mode (no --sink): the query must still run and the top-level
/// aggregate reachability stays `null` (frontier mode has no sink to aggregate
/// against) -- this is the CLI's analogue of the frontier_mode() reasoning path.
#[test]
fn taint_reaches_frontier_mode_omits_reachability() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.py"),
        "def f():\n    user = input()\n    sink(user)\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "taint-reaches", "--repo"])
        .arg(dir.path())
        .args(["--source", "app.py:2", "--format", "json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["reasoning"]["reachability"].is_null());
}

/// Malformed `--source` spec (missing `:line`) must be a clean CLI error, not a
/// panic.
#[test]
fn taint_reaches_rejects_malformed_source_spec() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("app.py"), "def f():\n    pass\n").unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "taint-reaches", "--repo"])
        .arg(dir.path())
        .args(["--source", "not-a-location", "--format", "json"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("file:line")
            || String::from_utf8_lossy(&out.stdout).contains("file:line"),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
