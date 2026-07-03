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

/// F3 (codex MAJOR): the CLI seed parser must normalize a `./`-prefixed
/// relative path exactly like the MCP loc-seed parser does
/// (`mcp::input::normalize_file_arg`) so `./app.py:2` resolves the same
/// function-relative variable as the bare `app.py:2` used in
/// `taint_reaches_reports_reachability_in_json` above, instead of missing the
/// `session.repo.files` lookup (keyed by normalized repo-relative paths) and
/// failing with an unsupported-file error.
#[test]
fn taint_reaches_normalizes_dot_slash_prefixed_source_path() {
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
            "--source",
            "./app.py:2",
            "--sink",
            "./app.py:4",
            "--format",
            "json",
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

/// F3: line `0` must be rejected at CLI seed-parse time (matching the MCP
/// loc-seed parser's eager `line < 1` check in `mcp::input::parse_seed`)
/// rather than being accepted at parse and only failing later, deep inside
/// `resolve_loc`, as `QueryError::LocationOutOfRange`. Asserting the message
/// does NOT mention "out of range" pins that this is now caught at the
/// earlier layer.
#[test]
fn taint_reaches_rejects_line_zero_at_parse() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("app.py"), "def f():\n    pass\n").unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "taint-reaches", "--repo"])
        .arg(dir.path())
        .args(["--source", "app.py:0", "--format", "json"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("at least 1"),
        "expected a clear min-line message; stderr: {stderr}"
    );
    assert!(
        !stderr.contains("out of range"),
        "line 0 must be rejected at parse time, not resolve_loc; stderr: {stderr}"
    );
}

/// F3: repeated `--source`/`--sink` flags must all be parsed and passed
/// through, not just the first (or last) occurrence. `reasoning.source_count`
/// directly reflects the number of resolved source seeds.
#[test]
fn taint_reaches_parses_repeated_source_and_sink_flags() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.py"),
        "def f():\n    user = input()\n    other = input()\n    sink(user)\n    sink(other)\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "taint-reaches", "--repo"])
        .arg(dir.path())
        .args([
            "--source", "app.py:2", "--source", "app.py:3", "--sink", "app.py:4", "--sink",
            "app.py:5", "--format", "json",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["reasoning"]["source_count"], 2);
    assert_eq!(v["reasoning"]["per_sink"].as_array().unwrap().len(), 2);
}
