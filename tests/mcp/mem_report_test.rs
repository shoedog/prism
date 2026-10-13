#![cfg(feature = "mcp")]

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp repo");
    std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n").expect("util.py");
    std::fs::write(
        dir.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .expect("main.py");
    dir
}

fn report_command(repo: &tempfile::TempDir) -> Command {
    let mut command = Command::cargo_bin("prism-mcp").expect("prism-mcp binary");
    command
        .arg("--repo")
        .arg(repo.path())
        .arg("--no-cache")
        .arg("--eager")
        .arg("--mem-settle-secs")
        .arg("0")
        .env("PRISM_MCP_MEM_REPORT", "1");
    command
}

fn report_line(stderr: &[u8]) -> Value {
    let text = String::from_utf8(stderr.to_vec()).expect("stderr utf8");
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("{\"prism_mem_report\":"))
        .collect();
    assert_eq!(lines.len(), 1, "exactly one report line in: {text}");
    serde_json::from_str(lines[0]).expect("report is JSON")
}

#[test]
fn report_mode_prints_one_report_and_does_not_serve() {
    let repo = repo();
    let output = report_command(&repo)
        .env("PRISM_TEST_SETTING", "on")
        // A real client would send this; report mode must not answer it.
        .write_stdin(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"t\",\"version\":\"0\"}}}\n",
        )
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(
        output.stdout.is_empty(),
        "report mode writes nothing to stdout"
    );
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(
        stderr.starts_with("prism-mcp: PRISM_MCP_MEM_REPORT=1"),
        "report mode announces itself first: {stderr}"
    );
    assert!(stderr.contains("not serving MCP"), "{stderr}");
    let report = report_line(&output.stderr);
    assert_eq!(report["stages"].as_array().unwrap().len(), 5);
    assert_eq!(report["attribution"]["files"], 2);
    assert_eq!(report["prism_mem_report"], 2);
    // The child ran with PRISM_MCP_MEM_REPORT=1 and PRISM_TEST_SETTING=on; only the second is a
    // setting that could change what is measured.
    assert_eq!(report["env"]["PRISM_TEST_SETTING"], "on");
    let env = report["env"].as_object().unwrap();
    assert!(!env.contains_key("PRISM_MCP_MEM_REPORT"), "{env:?}");
}

#[test]
fn a_generated_workload_feeds_a_workload_and_refresh_run() {
    let repo = repo();
    let emitted = report_command(&repo)
        .arg("--mem-emit-workload")
        .arg("2")
        .assert()
        .success()
        .get_output()
        .clone();
    let workload = repo.path().join("..").join(format!(
        "{}-workload.jsonl",
        repo.path().file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&workload, &emitted.stdout).expect("write workload");
    assert_eq!(String::from_utf8_lossy(&emitted.stdout).lines().count(), 8);

    let output = report_command(&repo)
        .arg("--mem-workload")
        .arg(&workload)
        .arg("--mem-refresh")
        .assert()
        .success()
        .get_output()
        .clone();
    let digest = format!("{:x}", Sha256::digest(&emitted.stdout));
    std::fs::remove_file(&workload).ok();
    let report = report_line(&output.stderr);
    assert_eq!(report["workload"]["sha256"], digest.as_str());
    assert_eq!(report["workload"]["requests"], 8);
    assert_eq!(report["workload"]["errors"], 0);
    assert_eq!(report["refresh"]["generation"], 1);
    assert_eq!(report["stages"].as_array().unwrap().len(), 7);
}

#[test]
fn report_mode_without_eager_fails_naming_the_variable() {
    let repo = repo();
    Command::cargo_bin("prism-mcp")
        .expect("prism-mcp binary")
        .arg("--repo")
        .arg(repo.path())
        .env("PRISM_MCP_MEM_REPORT", "1")
        .write_stdin("")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("PRISM_MCP_MEM_REPORT"))
        .stderr(predicate::str::contains("--eager"))
        .stderr(predicate::str::contains("unset"));
}

#[test]
fn mem_flags_without_the_variable_are_rejected() {
    let repo = repo();
    for flag in [vec!["--mem-refresh"], vec!["--mem-emit-workload", "2"]] {
        Command::cargo_bin("prism-mcp")
            .expect("prism-mcp binary")
            .arg("--repo")
            .arg(repo.path())
            .arg("--eager")
            .arg("--no-cache")
            .args(&flag)
            .env_remove("PRISM_MCP_MEM_REPORT")
            .write_stdin("")
            .assert()
            .failure()
            .stderr(predicate::str::contains("PRISM_MCP_MEM_REPORT=1"));
    }
}

#[test]
fn any_other_value_of_the_variable_leaves_the_server_alone() {
    let repo = repo();
    let output = Command::cargo_bin("prism-mcp")
        .expect("prism-mcp binary")
        .arg("--repo")
        .arg(repo.path())
        .arg("--no-cache")
        .env("PRISM_MCP_MEM_REPORT", "0")
        .write_stdin("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n")
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "the server answered the ping: {stdout}"
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("prism_mem_report"));
}

#[test]
fn a_rejected_workload_file_fails_with_its_line_number() {
    let repo = repo();
    let workload = repo.path().join("..").join(format!(
        "{}-bad.jsonl",
        repo.path().file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(
        &workload,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n",
    )
    .unwrap();
    report_command(&repo)
        .arg("--mem-workload")
        .arg(&workload)
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"));
    std::fs::remove_file(&workload).ok();
}

/// The tree-sitter heap figure for a Go file of `package main` plus 100 `var x int` lines.
///
/// 64_368 bytes is an independent census of that tree's subtree storage before allocator
/// rounding, taken by a reviewer with a C probe against tree-sitter 0.25.10. The reported
/// figure counts usable allocation sizes, so it cannot be below that; four times it would mean
/// something other than the one tree is being counted.
const GO_TREE_SUBTREE_BYTES: u64 = 64_368;

#[test]
fn the_tree_sitter_heap_figure_covers_the_retained_tree() {
    let dir = tempfile::tempdir().expect("temp repo");
    let source = format!("package main\n{}", "var x int\n".repeat(100));
    std::fs::write(dir.path().join("main.go"), source).expect("main.go");
    let output = report_command(&dir).assert().success().get_output().clone();
    let report = report_line(&output.stderr);
    assert_eq!(report["attribution"]["files"], 1, "{report}");

    let stages = report["stages"].as_array().unwrap();
    let stage = |name: &str| {
        stages
            .iter()
            .find(|stage| stage["name"] == name)
            .unwrap_or_else(|| panic!("no {name} stage in {report}"))
    };
    assert_eq!(
        stage("start")["tree_sitter_heap_bytes"],
        0,
        "nothing has parsed at the start"
    );
    for name in ["after_load_repo", "after_settle"] {
        let stage = stage(name);
        let tree_bytes = stage["tree_sitter_heap_bytes"]
            .as_u64()
            .unwrap_or_else(|| panic!("{name}: tree_sitter_heap_bytes is not a number: {stage}"));
        assert!(
            tree_bytes >= GO_TREE_SUBTREE_BYTES,
            "{name}: {tree_bytes} is below the {GO_TREE_SUBTREE_BYTES} bytes the tree holds"
        );
        assert!(
            tree_bytes < 4 * GO_TREE_SUBTREE_BYTES,
            "{name}: {tree_bytes} is four times the tree's size or more"
        );
        if let Some(heap) = stage["heap_in_use_bytes"].as_u64() {
            assert!(
                tree_bytes <= heap,
                "{name}: tree-sitter holds {tree_bytes}, more than the whole heap in use {heap}"
            );
        }
    }
}
