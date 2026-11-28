//! Fold-1 regressions for the real walk proofs that need a process of their own (the proof
//! configuration is read once per process): the compare line of a return crossing and the
//! `+unknown` misconfiguration.

use std::process::{Command, Output};

use super::object_literal_arg_tests::parse_files;
use super::trace_descent_tests::occurrence;
use super::trace_proofs::WalkProofs;
use super::{CodePropertyGraph, ReturnFlowMode, VarAccess};
use crate::languages::Language;
use crate::reasoning::order::AstOrderView;

const REAL: &str = "PRISM_TAINT_WALK_PROOFS_REAL";
const COMPARE: &str = "PRISM_TAINT_WALK_PROOFS_COMPARE";
const CASE: &str = "PRISM_IWALK_FOLD1_TEST_CASE";
const FLOW: &str =
    "function g(p) { return p; }\nfunction f(t) {\n  const out = g(t);\n  sink(out);\n}\n";

fn child(env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args([
        "--exact",
        "cpg::walk_proofs_fold1_cfg_tests::fold1_case",
        "--nocapture",
    ]);
    for name in [
        "PRISM_TAINT_WALK_PROOFS_FILE",
        "PRISM_TAINT_WALK_PROOFS",
        REAL,
        COMPARE,
        "PRISM_TAINT_CROSSING_TRACE",
    ] {
        command.env_remove(name);
    }
    command
        .env(CASE, "walk")
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

/// Walks the return flow of `FLOW` (the `return p` operand into `out`) under the process's
/// proof configuration.
#[test]
fn fold1_case() {
    if std::env::var(CASE).is_err() {
        return;
    }
    let files = parse_files(Language::JavaScript, &[("a", FLOW)], "js");
    let cpg = CodePropertyGraph::build(&files);
    let returned = occurrence(&cpg, FLOW.find("return p").unwrap() + 7, VarAccess::Use);
    let out = occurrence(&cpg, FLOW.find("out =").unwrap(), VarAccess::Def);
    let view = AstOrderView::new(&cpg, &files);
    let trace = cpg.taint_trace_nodes_with_mode(&[returned], Some(&view), ReturnFlowMode::On);
    assert!(trace.frontier_by_root[&returned].contains(&out));
    println!(
        "RETURN from={:?} to={:?} root={:?}",
        cpg.trace_key(returned),
        cpg.trace_key(out),
        cpg.trace_key(returned)
    );
}

#[test]
fn compare_prints_an_unsupported_line_per_rule_for_an_accepted_return_crossing() {
    let output = child(&[(COMPARE, "1")]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stdout}\n{stderr}");
    let keys = stdout
        .lines()
        .find_map(|line| line.strip_prefix("RETURN "))
        .unwrap();
    for rule in ["i", "ii"] {
        let line = format!(
            "taint-walk-proof-compare rule={rule} relation=ReturnFlow {keys} verdict=unsupported reason=- family=return_proofs_pending"
        );
        assert_eq!(
            stderr.lines().filter(|seen| *seen == line).count(),
            1,
            "missing `{line}` in\n{stderr}"
        );
    }
    // Compare stays observational: the same walk without it reaches the same frontier (the
    // child asserts the crossing itself).
    let off = child(&[]);
    assert!(off.status.success());
    assert!(!String::from_utf8(off.stderr)
        .unwrap()
        .contains("taint-walk-proof-compare"));
}

#[test]
fn real_plus_unknown_without_a_rule_is_a_configuration_error() {
    for bad in ["+unknown", " +unknown", ",+unknown"] {
        let error = WalkProofs::from_parts_real(None, None, Some(bad), false).unwrap_err();
        assert!(
            error.starts_with(REAL) && error.contains("+unknown"),
            "`{bad}`: {error}"
        );
    }
    // Controls: the suffix after a rule, and an empty value, are fine.
    assert!(
        WalkProofs::from_parts_real(None, None, Some("i+unknown"), false)
            .unwrap()
            .is_some()
    );
    assert!(WalkProofs::from_parts_real(None, None, Some(""), false)
        .unwrap()
        .is_none());
    let output = child(&[(REAL, "+unknown")]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(REAL) && stderr.contains("+unknown"),
        "{stderr}"
    );
}
