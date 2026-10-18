use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

const SOURCE: &str = "function largest() {\n let a = 1;\n let b = 2;\n let c = 3;\n return a + b + c;\n}\nfunction small() { return 0; }\n";

fn nav(repo: &Path, profile: Option<&Path>, query: &str, threads: &str) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_prism"));
    cmd.env_remove("PRISM_PROFILE")
        .env("RAYON_NUM_THREADS", threads)
        .args(["nav", "--no-cache", query, "--repo"])
        .arg(repo);
    cmd.arg(if query == "call-stats" {
        "--dump-sites"
    } else {
        "--edges"
    });
    if let Some(path) = profile {
        cmd.env("PRISM_PROFILE", path);
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn records(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .expect("profile must exist")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}

fn assert_phases(rows: &[Value]) {
    for name in [
        "load_parse",
        "scope_graph",
        "call_graph",
        "dfg",
        "cpg_assembly",
        "cpg.step7",
    ] {
        assert!(
            rows.iter()
                .any(|r| r["record"] == "phase" && r["name"] == name && r["status"] == "measured"),
            "missing {name}"
        );
    }
    for row in rows
        .iter()
        .filter(|r| r["record"] == "phase" && r["status"] == "measured")
    {
        assert!(row["wall_ns"].is_u64());
        assert!(row["peak_rss_bytes"].is_u64());
        assert!(row["cpu_ns"].is_u64());
        assert!(matches!(
            row["time_kind"].as_str(),
            Some("serial_wall" | "parallel_work")
        ));
    }
}

#[test]
fn nav_profile_accounts_for_owners_and_searches() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.js"), SOURCE).unwrap();
    for threads in ["1", "4"] {
        let profile = dir.path().join(format!("profile-{threads}.jsonl"));
        nav(dir.path(), Some(&profile), "dfg-stats", threads);
        let rows = records(&profile);
        assert_phases(&rows);
        let owners: Vec<_> = rows.iter().filter(|r| r["record"] == "owner").collect();
        assert_eq!(owners.len(), 2);
        let largest = owners.iter().find(|r| r["owner"] == "largest").unwrap();
        assert_eq!(largest["file"], "a.js");
        assert_eq!(largest["language"], "JavaScript");
        assert_eq!(largest["start_line"], 1);
        assert_eq!(largest["start_byte"], 0);
        assert_eq!(largest["byte_size"], 77);
        assert_eq!(largest["ast_nodes"], 40);
        assert_eq!(largest["assignment_paths"], 3);
        assert_eq!(largest["definitions"], 3);
        assert_eq!(largest["counters"]["simple_searches"], 3);
        assert_eq!(largest["counters"]["member_searches"], 0);
        assert_eq!(
            largest["counters"]["searched_bytes"],
            58 * 3 // Each let binding searches the 58-byte body block.
        );
        assert!(largest["flow_search_ns"].as_u64().unwrap() > 0);
        assert!(largest["reaching_definitions_ns"].as_u64().unwrap() > 0);
        let small = owners.iter().find(|r| r["owner"] == "small").unwrap();
        assert_eq!(small["byte_size"], 30);
        assert_eq!(small["ast_nodes"], 13);
        assert_eq!(small["definitions"], 0);
        assert_eq!(small["assignment_paths"], 0);
        assert_eq!(small["counters"]["simple_searches"], 0);
        assert_eq!(small["flow_search_ns"], 0);
        let summary = rows.iter().find(|r| r["record"] == "summary").unwrap();
        assert_eq!(summary["owner_count"], owners.len());
        assert_eq!(
            summary["top_owners"].as_array().unwrap().len() as u64
                + summary["other_owners"]["count"].as_u64().unwrap(),
            owners.len() as u64
        );
        assert!(rows
            .iter()
            .any(|r| r["name"] == "dfg.file" && r["time_kind"] == "parallel_work"));
        assert!(rows
            .iter()
            .any(|r| r["name"] == "dfg.file_rd" && r["time_kind"] == "parallel_work"));
    }
}

#[test]
fn profile_does_not_change_outputs_and_warns_once() {
    let dir = tempfile::tempdir().unwrap();
    for (file, source) in [
        ("a.js", SOURCE),
        ("b.ts", "function typed(x: number) { return x; }"),
        ("c.py", "def py(x):\n    y = x + 1\n    return y\n"),
    ] {
        let caller = match file {
            "a.js" => "\nfunction jsCaller() { return largest(); }\n",
            "b.ts" => "\nfunction tsCaller() { return typed(1); }\n",
            _ => "\ndef pyCaller():\n    return py(1)\n",
        };
        std::fs::write(dir.path().join(file), format!("{source}{caller}")).unwrap();
    }
    for query in ["call-stats", "dfg-stats"] {
        let off = nav(dir.path(), None, query, "1");
        assert!(
            !off.stdout.is_empty(),
            "identity control must have observable output"
        );
        let path = dir.path().join("profile.jsonl");
        let on = nav(dir.path(), Some(&path), query, "4");
        assert_eq!(off.stdout, on.stdout);
        assert_eq!(off.stderr, on.stderr);
        let rows = records(&path);
        for language in ["JavaScript", "TypeScript", "Python"] {
            assert!(rows
                .iter()
                .any(|r| r["record"] == "owner" && r["language"] == language));
        }
        let py = rows
            .iter()
            .find(|r| r["record"] == "owner" && r["owner"] == "py")
            .unwrap();
        assert_eq!(py["counters"]["simple_searches"], 2);
        assert_eq!(
            py["counters"]["searched_bytes"],
            py["byte_size"].as_u64().unwrap() * 2
        );
        let bad = nav(dir.path(), Some(dir.path()), query, "4");
        assert_eq!(off.stdout, bad.stdout);
        assert_eq!(off.status.code(), bad.status.code());
        assert_eq!(
            String::from_utf8_lossy(&bad.stderr)
                .matches("PRISM_PROFILE:")
                .count(),
            1
        );
    }
}

#[test]
fn review_profile_has_build_phases() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.js"), SOURCE).unwrap();
    let diff = dir.path().join("change.patch");
    std::fs::write(&diff, "diff --git a/a.js b/a.js\n--- a/a.js\n+++ b/a.js\n@@ -2 +2 @@\n- let a = 0;\n+ let a = 1;\n").unwrap();
    let profile = dir.path().join("review.jsonl");
    let out = Command::new(env!("CARGO_BIN_EXE_prism"))
        .env("PRISM_PROFILE", &profile)
        .args(["--repo"])
        .arg(dir.path())
        .args(["--diff"])
        .arg(diff)
        .args(["--algorithm", "leftflow", "--no-cache"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_phases(&records(&profile));
}

#[test]
fn cache_profile_records_load_and_save_and_empty_owner_summary() {
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.js"), SOURCE).unwrap();
    let profile = dir.path().join("cached.jsonl");
    for pass in 0..2 {
        let out = Command::new(env!("CARGO_BIN_EXE_prism"))
            .env("PRISM_PROFILE", &profile)
            .args(["nav", "--cache-dir"])
            .arg(cache.path())
            .args(["dfg-stats", "--repo"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let rows = records(&profile);
        assert!(rows
            .iter()
            .any(|r| r["name"] == "cache.load" && r["status"] == "measured"));
        if pass == 0 {
            assert_eq!(rows.iter().filter(|r| r["record"] == "owner").count(), 2);
            assert!(rows
                .iter()
                .any(|r| r["name"] == "cache.save" && r["status"] == "measured"));
        } else {
            assert!(!rows.iter().any(|r| r["record"] == "owner"));
            let summary = rows.iter().find(|r| r["record"] == "summary").unwrap();
            assert_eq!(summary["owner_count"], 0);
            assert_eq!(summary["top_dfg_share"], 0.0);
        }
    }
}

#[test]
fn summary_conserves_normal_population_and_member_searches() {
    let dir = tempfile::tempdir().unwrap();
    let mut source =
        String::from("function member() {\n box.a = 1;\n box.a = 2;\n return box.a;\n}\n");
    for n in 0..12 {
        source.push_str(&format!("function normal{n}() {{ return {n}; }}\n"));
    }
    std::fs::write(dir.path().join("owners.js"), source).unwrap();
    let profile = dir.path().join("profile.jsonl");
    nav(dir.path(), Some(&profile), "call-stats", "4");
    let rows = records(&profile);
    let owners: Vec<_> = rows.iter().filter(|r| r["record"] == "owner").collect();
    assert_eq!(owners.len(), 13);
    let member = owners.iter().find(|r| r["owner"] == "member").unwrap();
    assert_eq!(member["counters"]["member_searches"], 2);
    assert_eq!(member["counters"]["simple_searches"], 0);
    let s = rows.iter().find(|r| r["record"] == "summary").unwrap();
    let top = s["top_owners"].as_array().unwrap();
    assert_eq!(top.len(), 10);
    assert_eq!(s["other_owners"]["count"], 3);
    let total: u64 = owners.iter().map(|o| o["total_ns"].as_u64().unwrap()).sum();
    let top_total: u64 = top.iter().map(|o| o["total_ns"].as_u64().unwrap()).sum();
    assert_eq!(s["total_owner_ns"], total);
    assert_eq!(s["other_owners"]["total_ns"], total - top_total);
    // serde_json without float_roundtrip can move a decimal by one ULP.
    assert!(
        (s["top_dfg_share"].as_f64().unwrap() - top_total as f64 / total as f64).abs()
            <= f64::EPSILON
    );
    let mut other: Vec<_> = owners
        .iter()
        .filter(|o| !top.iter().any(|t| t["owner"] == o["owner"]))
        .map(|o| o["total_ns"].as_u64().unwrap())
        .collect();
    other.sort_unstable();
    assert_eq!(s["other_owners"]["p50_ns"], other[1]);
    assert_eq!(s["other_owners"]["p90_ns"], other[2]);
    assert_eq!(s["other_owners"]["p99_ns"], other[2]);
    assert_eq!(s["other_owners"]["max_ns"], other[2]);
    let file_rd = rows.iter().find(|r| r["name"] == "dfg.file_rd").unwrap();
    assert_eq!(file_rd["aggregation"], "sum_owner_intervals");
    assert_eq!(
        file_rd["wall_ns"],
        owners
            .iter()
            .map(|o| o["reaching_definitions_ns"].as_u64().unwrap())
            .sum::<u64>()
    );
}

#[test]
fn empty_file_keeps_zero_owner_summary() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("empty.py"), "x = 1\n").unwrap();
    let profile = dir.path().join("profile.jsonl");
    nav(dir.path(), Some(&profile), "dfg-stats", "1");
    let rows = records(&profile);
    let s = rows.iter().find(|r| r["record"] == "summary").unwrap();
    assert_eq!(s["owner_count"], 0);
    assert_eq!(s["top_dfg_share"], 0.0);
    for key in ["count", "total_ns", "p50_ns", "p90_ns", "p99_ns", "max_ns"] {
        assert_eq!(s["other_owners"][key], 0);
    }
    assert!(s["top_owners"].as_array().unwrap().is_empty());
    assert!(rows
        .iter()
        .any(|r| r["name"] == "dfg.file_rd" && r["status"] == "not_run"));
}

#[test]
fn alternate_call_graph_paths_are_profiled() {
    for threads in ["1", "4"] {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        let cache = dir.path().join("cache");
        let profile = dir.path().join("profile.jsonl");
        let diff = dir.path().join("change.patch");
        let write = |value| {
            std::fs::write(
                repo.join("a.js"),
                format!("function f() {{\n let x = {value};\n return x;\n}}\n"),
            )
            .unwrap();
            std::fs::write(&diff, format!("diff --git a/a.js b/a.js\n--- a/a.js\n+++ b/a.js\n@@ -2 +2 @@\n- let x = 0;\n+ let x = {value};\n")).unwrap();
        };
        let invoke = |scoped| {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_prism"));
            cmd.env("PRISM_PROFILE", &profile)
                .env("RAYON_NUM_THREADS", threads)
                .arg("--repo")
                .arg(&repo)
                .arg("--diff")
                .arg(&diff)
                .args(["--algorithm", "leftflow"]);
            if scoped {
                cmd.args(["--no-cache", "--scoped-cpg"]);
            } else {
                cmd.arg("--cache-dir").arg(&cache);
            }
            let out = cmd.output().unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            (records(&profile), String::from_utf8(out.stderr).unwrap())
        };
        write(1);
        let (cold, _) = invoke(false);
        assert_phases(&cold);
        write(2);
        let (partial, stderr) = invoke(false);
        assert!(stderr.contains("partial hit"), "{stderr}");
        assert_phases(&partial);
        for name in [
            "call_graph.direct_subset.definitions",
            "call_graph.direct_subset.calls",
            "call_graph.incremental_refresh",
            "cpg.incremental",
        ] {
            assert!(
                partial
                    .iter()
                    .any(|r| r["name"] == name && r["status"] == "measured"),
                "missing {name}"
            );
        }
        let (hit, _) = invoke(false);
        assert!(hit
            .iter()
            .any(|r| r["record"] == "summary" && r["owner_count"] == 0));
        assert!(hit
            .iter()
            .any(|r| r["name"] == "call_graph" && r["status"] == "not_run"));
        std::fs::write(repo.join("b.js"), "function g() { return 1; }\n").unwrap();
        let mut patch = std::fs::read_to_string(&diff).unwrap();
        patch.push_str("diff --git a/b.js b/b.js\n--- a/b.js\n+++ b/b.js\n@@ -1 +1 @@\n-function g() { return 0; }\n+function g() { return 1; }\n");
        std::fs::write(&diff, patch).unwrap();
        let (scoped, _) = invoke(true);
        assert_phases(&scoped);
        for name in [
            "call_graph.skeleton.definitions",
            "call_graph.skeleton.calls",
        ] {
            assert!(
                scoped
                    .iter()
                    .any(|r| r["name"] == name && r["status"] == "measured"),
                "missing {name}"
            );
        }
    }
}
