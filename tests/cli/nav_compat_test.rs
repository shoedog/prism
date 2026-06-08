use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("prism").unwrap()
}

const REPO: &str = "tests/fixtures/nav_compat";
const DIFF: &str = "tests/fixtures/nav_compat/d.json";
const CG: &str = "tests/fixtures/nav_callgraph";

fn golden(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/nav_compat/golden/{name}")).unwrap()
}

// The `review` preset aggregates all 26 algorithms; at least one (Taint, on this
// fixture) is environment-sensitive / not byte-stable, so the aggregate cannot be a
// byte-for-byte golden. We instead lock byte-for-byte on DETERMINISTIC algorithms —
// which proves the ReviewArgs CLI refactor preserved output, since the refactor is
// algorithm-agnostic — and smoke-test that `review` itself still runs.
// (Taint's nondeterminism is a pre-existing issue tracked in the roadmap follow-ups.)
#[test]
fn review_still_runs_nonempty() {
    let out = bin()
        .args(["--repo", REPO, "--diff", DIFF, "--algorithm", "review"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!out.stdout.is_empty(), "review must still produce output");
}

#[test]
fn leftflow_text_byte_identical() {
    let out = bin()
        .args(["--repo", REPO, "--diff", DIFF, "--algorithm", "leftflow"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("leftflow.txt"));
}

#[test]
fn thin_byte_identical() {
    let out = bin()
        .args(["--repo", REPO, "--diff", DIFF, "--algorithm", "thin"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("thin.txt"));
}

#[test]
fn parentfunction_byte_identical() {
    let out = bin()
        .args([
            "--repo",
            REPO,
            "--diff",
            DIFF,
            "--algorithm",
            "parentfunction",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        golden("parentfunction.txt")
    );
}

#[test]
fn leftflow_json_byte_identical() {
    let out = bin()
        .args([
            "--repo",
            REPO,
            "--diff",
            DIFF,
            "--algorithm",
            "leftflow",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        golden("leftflow.json")
    );
}

#[test]
fn list_algorithms_byte_identical() {
    let out = bin().args(["--list-algorithms"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("list.txt"));
}

#[test]
fn nav_with_review_flag_is_parse_error() {
    let out = bin()
        .args(["nav", "nodes-at", "--repo", REPO, "--diff", DIFF])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unexpected argument"),
        "stderr was:\n{stderr}"
    );
    assert!(
        !stderr.contains("not yet implemented"),
        "parse errors must happen before the nav stub runs; stderr was:\n{stderr}"
    );
}

#[test]
fn top_level_review_flag_before_nav_subcommand_is_parse_error() {
    let out = bin()
        .args([
            "--algorithm",
            "taint",
            "nav",
            "nodes-at",
            "--repo",
            ".",
            "--location",
            "x:1",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("unexpected argument"),
        "stderr was:\n{stderr}"
    );
    assert!(
        !stderr.contains("not yet implemented"),
        "parse errors must happen before the nav stub runs; stderr was:\n{stderr}"
    );
}

#[test]
fn nav_nodes_at_json_on_self() {
    // Dogfood: run against this repo (cargo test cwd = crate root).
    let out = bin()
        .args([
            "nav",
            "nodes-at",
            "--repo",
            ".",
            "--location",
            "src/main.rs:300",
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
    assert!(v["query"]
        .as_str()
        .unwrap()
        .starts_with("nodes-at:src/main.rs:300"));
    assert!(v["items"].is_array());
}

#[test]
fn nav_nodes_at_rejects_unknown_format() {
    let out = bin()
        .args([
            "nav",
            "nodes-at",
            "--repo",
            REPO,
            "--location",
            "a.py:1",
            "--format",
            "jsn",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn nav_callers_json_on_self() {
    let out = bin()
        .args([
            "nav",
            "callers",
            "--repo",
            ".",
            "--symbol",
            "build_scoped",
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
    assert!(v["query"]
        .as_str()
        .unwrap()
        .starts_with("callers:build_scoped"));
    assert!(v["items"].is_array());
}

#[test]
fn nav_subcommands_reject_review_diff_flag() {
    for sub in ["callers", "callees", "ego"] {
        let out = bin()
            .args([
                "nav", sub, "--repo", ".", "--symbol", "x", "--diff", "d.json",
            ])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "`nav {sub} --diff` must be a clap parse error"
        );
    }
}

#[test]
fn nav_subcommands_reject_unknown_format() {
    for sub in ["callers", "callees", "ego"] {
        let out = bin()
            .args([
                "nav", sub, "--repo", ".", "--symbol", "x", "--format", "jsn",
            ])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "`nav {sub} --format jsn` must be a clap parse error"
        );
    }
}

#[test]
fn nav_ambiguous_seed_exits_3() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def dup():\n    return 1\n").unwrap();
    std::fs::write(dir.path().join("b.py"), "def dup():\n    return 2\n").unwrap();
    let out = bin()
        .args([
            "nav",
            "callers",
            "--repo",
            dir.path().to_str().unwrap(),
            "--symbol",
            "dup",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["error"]["AmbiguousSymbol"].is_object());
}

#[test]
fn nav_error_text_format_is_short_text() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), "def dup():\n    return 1\n").unwrap();
    std::fs::write(dir.path().join("b.py"), "def dup():\n    return 2\n").unwrap();
    let out = bin()
        .args([
            "nav",
            "callers",
            "--repo",
            dir.path().to_str().unwrap(),
            "--symbol",
            "dup",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("error: ambiguous symbol"));
    assert!(serde_json::from_slice::<serde_json::Value>(&out.stdout).is_err());
}

#[test]
fn callees_golden_qualified() {
    let out = bin()
        .args([
            "nav", "callees", "--repo", CG, "--symbol", "run", "--format", "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        golden("callees_run.json")
    );
}

#[test]
fn ego_golden() {
    let out = bin()
        .args([
            "nav", "ego", "--repo", CG, "--symbol", "run", "--hops", "1", "--edges", "Call",
            "--format", "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), golden("ego_run.json"));
}

#[test]
fn callees_empty_for_leaf() {
    let out = bin()
        .args([
            "nav", "callees", "--repo", CG, "--symbol", "lonely", "--format", "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["items"].as_array().unwrap().is_empty());
    assert_eq!(v["query"], "callees:lonely@main.py");
}
