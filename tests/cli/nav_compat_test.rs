use assert_cmd::Command;
use prism::navigation::cache::nav_cache_subdir;
use prism::repo_loader::load_repo;

fn bin() -> Command {
    Command::cargo_bin("prism").unwrap()
}

const REPO: &str = "tests/fixtures/nav_compat";
const DIFF: &str = "tests/fixtures/nav_compat/d.json";
const CG: &str = "tests/fixtures/nav_callgraph";
const RUST_SCOPED: &str = "tests/fixtures/nav_scoped_rust";

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
fn nav_nodes_at_json_on_fixture() {
    let out = bin()
        .args([
            "nav",
            "nodes-at",
            "--repo",
            REPO,
            "--location",
            "a.py:1",
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
    assert!(v["query"].as_str().unwrap().starts_with("nodes-at:a.py:1"));
    assert!(!v["items"].as_array().unwrap().is_empty());
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
fn nav_symbol_spans_json_and_text_on_fixture_without_source_echo() {
    let json_out = bin()
        .args([
            "nav",
            "symbol-spans",
            "--repo",
            REPO,
            "--symbol",
            "f",
            "--file",
            "a.py",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        json_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&json_out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(value["schema_version"], "1.0");
    assert_eq!(value["query"], "symbol-spans:f@a.py");
    assert_eq!(value["symbol_span"]["file"], "a.py");
    assert!(value["name_span"]["start_byte"].is_number());
    assert!(value["body_span"]["end_byte"].is_number());
    assert!(!String::from_utf8_lossy(&json_out.stdout).contains("y = x + 1"));

    let text_out = bin()
        .args([
            "nav",
            "symbol-spans",
            "--repo",
            REPO,
            "--location",
            "a.py:2",
            "--format",
            "text",
        ])
        .output()
        .unwrap();
    assert!(
        text_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&text_out.stderr)
    );
    let text = String::from_utf8_lossy(&text_out.stdout);
    assert!(text.contains("symbol_span"));
    assert!(text.contains("name_span"));
    assert!(text.contains("body_span"));
    assert!(!text.contains("y = x + 1"));
}

#[test]
fn nav_symbol_spans_requires_one_valid_seed_shape() {
    let cases = [
        vec!["nav", "symbol-spans", "--repo", REPO],
        vec![
            "nav",
            "symbol-spans",
            "--repo",
            REPO,
            "--symbol",
            "f",
            "--location",
            "a.py:1",
        ],
        vec!["nav", "symbol-spans", "--repo", REPO, "--file", "a.py"],
    ];

    for args in cases {
        let out = bin().args(args).output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "invalid seed shape must fail in clap; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn nav_callers_json_on_fixture() {
    let out = bin()
        .args([
            "nav", "callers", "--repo", CG, "--symbol", "helper", "--file", "util.py", "--format",
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
        .starts_with("callers:helper@util.py"));
    assert!(v["items"].as_array().unwrap().iter().any(|it| {
        it["symbol"]["Function"]["file"] == "main.py" && it["symbol"]["Function"]["name"] == "run"
    }));
}

#[test]
fn nav_cache_writes_under_repo_subdir_and_no_cache_matches_output() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(
        repo.path().join("a.py"),
        "def helper():\n    return 1\n\ndef run():\n    return helper()\n",
    )
    .unwrap();

    let cached_cache = tempfile::tempdir().unwrap();
    let cached = bin()
        .args([
            "nav",
            "--cache-dir",
            cached_cache.path().to_str().unwrap(),
            "callers",
            "--repo",
            repo.path().to_str().unwrap(),
            "--symbol",
            "helper",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        cached.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&cached.stderr)
    );
    assert!(
        nav_cache_subdir(cached_cache.path(), &load_repo(repo.path()).unwrap())
            .join("cpg-cache.bin")
            .exists(),
        "cached nav run must write cpg-cache.bin under the per-repo cache subdir"
    );

    let uncached = bin()
        .args([
            "nav",
            "--no-cache",
            "callers",
            "--repo",
            repo.path().to_str().unwrap(),
            "--symbol",
            "helper",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        uncached.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&uncached.stderr)
    );
    assert_eq!(cached.stdout, uncached.stdout);
}

#[test]
fn nav_no_cache_conflicts_with_cache_dir() {
    let out = bin()
        .args([
            "nav",
            "--no-cache",
            "--cache-dir",
            "/tmp/prism-nav-cache",
            "callers",
            "--repo",
            ".",
            "--symbol",
            "helper",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflicts with"),
        "stderr was:\n{stderr}"
    );
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

#[test]
fn module_deps_golden() {
    // CG/main.py: util.helper() resolves cross-file to util.py (PrismCpg) AND
    // `import util` is labeled UnresolvedImport (HeuristicImport) + a warning.
    let out = bin()
        .args([
            "nav",
            "module-deps",
            "--repo",
            CG,
            "--file",
            "main.py",
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
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        golden("module_deps_run.json")
    );
}

#[test]
fn repo_map_golden() {
    let out = bin()
        .args(["nav", "repo-map", "--repo", CG, "--format", "json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        golden("repo_map_run.json")
    );
}

#[test]
fn module_deps_repo_map_live_smoke() {
    // Rust scoped-call fixture: the path must run through the real CLI and emit
    // valid JSON. Dedicated CPG tests cover whole-repo/deep-fixture parse safety.
    let md = bin()
        .args([
            "nav",
            "module-deps",
            "--repo",
            RUST_SCOPED,
            "--file",
            "src/lib.rs",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        md.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&md.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&md.stdout).unwrap();
    assert_eq!(v["query"], "module-deps:src/lib.rs");
    assert!(v["items"].is_array());
    assert!(v.get("graph").is_none(), "module-deps is a flat item list");
    assert!(v["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|it| it["source"] == "PrismCpg" && it["location"]["file"] == "src/algo.rs"));

    let rm = bin()
        .args(["nav", "repo-map", "--repo", RUST_SCOPED, "--format", "json"])
        .output()
        .unwrap();
    assert!(
        rm.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&rm.stderr)
    );
    let r: serde_json::Value = serde_json::from_slice(&rm.stdout).unwrap();
    assert_eq!(r["query"], "repo-map");
    let nodes = r["graph"]["nodes"].as_array().unwrap();
    assert!(nodes.iter().any(|n| n["location"]["file"] == "src/lib.rs"));
    assert!(nodes.iter().any(|n| n["location"]["file"] == "src/algo.rs"));
}

#[test]
fn callees_resolves_scoped_dispatch_dogfood() {
    // The fixture dispatches via a scoped Rust module call; before 3b.5 this
    // resolved 0 cross-file callees.
    let out = bin()
        .args([
            "nav",
            "callees",
            "--repo",
            RUST_SCOPED,
            "--symbol",
            "dispatch",
            "--file",
            "src/lib.rs",
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
    let resolved_cross_file = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|it| it["symbol"].is_object() && it["symbol"]["Function"]["file"] == "src/algo.rs")
        .count();
    assert!(
        resolved_cross_file > 0,
        "dispatch should resolve scoped algorithm callees cross-file; got {resolved_cross_file}"
    );
}

#[test]
fn onboard_emits_markdown_by_default_and_typed_json_on_request() {
    let markdown = bin()
        .args(["nav", "--no-cache", "onboard", "--repo", CG])
        .output()
        .unwrap();
    assert!(
        markdown.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&markdown.stderr)
    );
    let text = String::from_utf8(markdown.stdout).unwrap();
    assert!(text.starts_with("# Prism project overview\n"));
    assert!(text.contains("## Inventory\n"));
    assert!(text.contains("## Module architecture\n"));
    assert!(text.contains("## Call resolution\n"));
    assert!(text.ends_with('\n'));

    let json = bin()
        .args([
            "nav",
            "--no-cache",
            "onboard",
            "--repo",
            CG,
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        json.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&json.stderr)
    );
    assert!(json.stdout.ends_with(b"\n"));
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(value["schema_version"], "1.0");
    assert_eq!(value["inventory"]["indexed_files"], 2);
    assert_eq!(value["modules"]["edges"], 1);
    assert!(value["modules"]["connected"].as_array().unwrap().len() <= 12);
}

#[test]
fn onboard_repo_dot_reports_the_canonical_repository_basename() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("app.py"), "def run():\n    return 1\n").unwrap();
    let expected = repo.path().file_name().unwrap().to_string_lossy();
    let out = bin()
        .current_dir(repo.path())
        .args([
            "nav",
            "--no-cache",
            "onboard",
            "--repo",
            ".",
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
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["project"], expected.as_ref());
}

#[test]
fn onboard_out_is_create_new_and_preserves_existing_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("overview.json");
    let out_arg = out_path.to_str().unwrap();
    let first = bin()
        .args([
            "nav",
            "--no-cache",
            "onboard",
            "--repo",
            CG,
            "--format",
            "json",
            "--out",
            out_arg,
        ])
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.stdout.is_empty());
    let original = std::fs::read(&out_path).unwrap();
    let _: serde_json::Value = serde_json::from_slice(&original).unwrap();

    let second = bin()
        .args([
            "nav",
            "--no-cache",
            "onboard",
            "--repo",
            CG,
            "--format",
            "json",
            "--out",
            out_arg,
        ])
        .output()
        .unwrap();
    assert!(!second.status.success());
    assert!(second.stdout.is_empty());
    assert_eq!(std::fs::read(&out_path).unwrap(), original);
    assert!(String::from_utf8_lossy(&second.stderr).contains("already exists"));
}

#[test]
fn onboard_out_requires_an_existing_parent_directory() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("missing").join("overview.md");
    let out = bin()
        .args([
            "nav",
            "--no-cache",
            "onboard",
            "--repo",
            CG,
            "--out",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(!out_path.exists());
    assert!(String::from_utf8_lossy(&out.stderr).contains("failed to create onboarding report"));
}

#[test]
fn onboard_rejects_unknown_format_before_loading_repo() {
    let out = bin()
        .args([
            "nav",
            "onboard",
            "--repo",
            "/path/that/does/not/exist",
            "--format",
            "text",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid value"));
}

#[test]
fn onboard_warms_cpg_and_resolved_call_edge_caches() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(
        repo.path().join("main.py"),
        "import util\n\ndef run():\n    return util.helper()\n",
    )
    .unwrap();
    std::fs::write(repo.path().join("util.py"), "def helper():\n    return 1\n").unwrap();
    let cache = tempfile::tempdir().unwrap();

    let out = bin()
        .args([
            "nav",
            "--cache-dir",
            cache.path().to_str().unwrap(),
            "onboard",
            "--repo",
            repo.path().to_str().unwrap(),
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

    let loaded = load_repo(repo.path()).unwrap();
    let cache_dir = nav_cache_subdir(cache.path(), &loaded);
    assert!(cache_dir.join("cpg-cache.bin").exists());
    assert!(cache_dir.join("resolved-call-edge-index.bin").exists());
}
