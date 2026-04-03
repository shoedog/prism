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
fn test_list_algorithms_shows_all_categories() {
    prism_cmd()
        .arg("--list-algorithms")
        .assert()
        .success()
        .stdout(predicate::str::contains("Paper (arXiv:2505.17928)"))
        .stdout(predicate::str::contains("Established taxonomy"))
        .stdout(predicate::str::contains("Theoretical extensions"))
        .stdout(predicate::str::contains("Novel extensions"))
        .stdout(predicate::str::contains("leftflow"))
        .stdout(predicate::str::contains("taint"))
        .stdout(predicate::str::contains("spiral"))
        .stdout(predicate::str::contains("absence"));
}

#[test]
fn test_list_algorithms_does_not_require_repo() {
    // --list-algorithms should work without --repo or --diff
    prism_cmd().arg("--list-algorithms").assert().success();
}

#[test]
fn test_explicit_leftflow_text() {
    prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--algorithm",
            "leftflow",
        ])
        .assert()
        .success();
}

#[test]
fn test_leftflow_python_fixture() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "leftflow",
        ])
        .assert()
        .success();
}

#[test]
fn test_originaldiff_python() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "originaldiff",
        ])
        .assert()
        .success();
}

#[test]
fn test_parentfunction_python() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "parentfunction",
        ])
        .assert()
        .success();
}

#[test]
fn test_fullflow_python() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "fullflow",
        ])
        .assert()
        .success();
}

#[test]
fn test_thin_python() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "thin",
        ])
        .assert()
        .success();
}

#[test]
fn test_comma_separated_algorithms() {
    let output = prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "leftflow,originaldiff,thin",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Multi-algo JSON should be valid");
    // Multi-run JSON has algorithms_run array
    let algos = json
        .get("algorithms_run")
        .expect("Should have algorithms_run");
    assert_eq!(algos.as_array().unwrap().len(), 3);
}

#[test]
fn test_review_suite() {
    let output = prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "review",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Review suite JSON");
    let algos = json
        .get("algorithms_run")
        .expect("Should have algorithms_run");
    // Review suite should run multiple algorithms
    assert!(
        algos.as_array().unwrap().len() > 1,
        "Review suite should run multiple algorithms"
    );
}

#[test]
fn test_barrier_slice_with_depth() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "barrier",
            "--barrier-depth",
            "3",
        ])
        .assert()
        .success();
}

#[test]
fn test_barrier_slice_with_symbols() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "barrier",
            "--barrier-symbols",
            "print,log",
        ])
        .assert()
        .success();
}

#[test]
fn test_chop_with_source_sink() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "chop",
            "--chop-source",
            "calc.py:5",
            "--chop-sink",
            "calc.py:7",
        ])
        .assert()
        .success();
}

#[test]
fn test_chop_missing_source_fails() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "chop",
            "--chop-sink",
            "calc.py:7",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("chop-source"));
}

#[test]
fn test_taint_with_explicit_source() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "taint",
            "--taint-source",
            "calc.py:5",
        ])
        .assert()
        .success();
}

#[test]
fn test_taint_auto_from_diff() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "taint",
        ])
        .assert()
        .success();
}

#[test]
fn test_conditioned_slice_with_condition() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "conditioned",
            "--condition",
            "x==5",
        ])
        .assert()
        .success();
}

#[test]
fn test_conditioned_missing_condition_fails() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "conditioned",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("condition"));
}

#[test]
fn test_spiral_with_max_ring() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "spiral",
            "--spiral-max-ring",
            "3",
        ])
        .assert()
        .success();
}

#[test]
fn test_horizontal_auto_pattern() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "horizontal",
        ])
        .assert()
        .success();
}

#[test]
fn test_horizontal_name_pattern() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "horizontal",
            "--peer-pattern",
            "name:test_*",
        ])
        .assert()
        .success();
}

#[test]
fn test_horizontal_decorator_pattern() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "horizontal",
            "--peer-pattern",
            "decorator:@route",
        ])
        .assert()
        .success();
}

#[test]
fn test_horizontal_class_pattern() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "horizontal",
            "--peer-pattern",
            "class:Calculator",
        ])
        .assert()
        .success();
}

#[test]
fn test_vertical_with_explicit_layers() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "vertical",
            "--layers",
            "handler,service,repository",
        ])
        .assert()
        .success();
}

#[test]
fn test_vertical_auto_layers() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "vertical",
        ])
        .assert()
        .success();
}

#[test]
fn test_angle_slice_error_handling() {
    prism_cmd()
        .args([
            "--repo",
            &fixture_path("c"),
            "--diff",
            &fixture_path("c/timer_uaf.diff"),
            "--algorithm",
            "angle",
            "--concern",
            "error_handling",
        ])
        .assert()
        .success();
}

#[test]
fn test_angle_slice_logging() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "angle",
            "--concern",
            "logging",
        ])
        .assert()
        .success();
}

#[test]
fn test_quantum_slice_with_var() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "quantum",
            "--quantum-var",
            "product",
        ])
        .assert()
        .success();
}

#[test]
fn test_quantum_slice_auto() {
    prism_cmd()
        .args([
            "--repo",
            "tests/fixtures/python",
            "--diff",
            &fixture_path("python/calc.diff"),
            "--algorithm",
            "quantum",
        ])
        .assert()
        .success();
}
