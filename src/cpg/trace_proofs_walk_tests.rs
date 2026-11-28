//! In-process tests of the walk-proof hooks and the loader: no subprocess, the configuration is
//! installed per thread through `with_walk_proofs`. The child-process tests in
//! `trace_proofs_tests` pin the real environment behaviour.

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

use serde_json::{json, Value};

use super::trace_proofs::{with_walk_proofs, WalkProofs};
use super::trace_proofs_tests::{
    empty_rules, entry, file, file_text, hop_of, reached, walk_report, with_rule,
};

const RULES: [&str; 7] = ["i", "ii", "iii", "iv", "v", "return_flow", "return_input"];

/// The report of one walk with `rule` switched on over `filed`, or off when `filed` is `None`.
fn walk(case: &str, filed: Option<(&str, &Value)>) -> Value {
    let proofs = filed.map(|(switches, filed)| {
        WalkProofs::from_parts(Some(&filed.to_string()), Some(switches))
            .unwrap()
            .unwrap()
    });
    with_walk_proofs(proofs, || walk_report(case))
}

fn off(case: &str) -> Value {
    walk(case, None)
}

#[test]
fn a_switched_on_rule_refuses_the_filed_descent_with_its_reason() {
    let control = off("gf");
    assert!(control["boundary"].as_array().unwrap().is_empty());
    let (from, to) = hop_of(&control, "CallDescent");
    for (rule, reason) in [
        ("i", "no_matching_site"),
        ("ii", "ambiguous_site"),
        ("iii", "not_exact"),
        ("iv", "unproven_slot"),
        ("v", "ambiguous_site"),
    ] {
        let filed = with_rule(rule, vec![entry("CallDescent", &from, &to, None)]);
        let report = walk("gf", Some((rule, &filed)));
        let boundary = report["boundary"].as_array().unwrap();
        assert_eq!(boundary.len(), 1, "{rule}: {report}");
        assert_eq!(boundary[0]["reason"], reason, "{rule}");
        assert_eq!(boundary[0]["relation"], "CallDescent");
        assert_eq!(boundary[0]["from"], from.as_str());
        assert_eq!(boundary[0]["to"], to.as_str());
        assert!(!reached(&report, &to), "{rule}: {report}");
    }
}

#[test]
fn a_rule_that_is_off_or_a_hop_that_is_not_filed_changes_nothing() {
    let control = off("gf");
    let (from, to) = hop_of(&control, "CallDescent");
    let filed = with_rule("iv", vec![entry("CallDescent", &from, &to, None)]);
    // Filed under iv, switched on iii.
    assert_eq!(walk("gf", Some(("iii", &filed))), control);
    // Another hop filed under the switched-on rule.
    let other = with_rule("iv", vec![entry("CallDescent", &from, "elsewhere", None)]);
    assert_eq!(walk("gf", Some(("iv", &other))), control);
}

#[test]
fn a_root_restriction_must_name_the_walk_root() {
    let control = off("gf");
    let (from, to) = hop_of(&control, "CallDescent");
    let root = control["root"].as_str().unwrap();
    let elsewhere = with_rule(
        "iv",
        vec![entry(
            "CallDescent",
            &from,
            &to,
            Some("a.js:1:0..1@None:q:Use"),
        )],
    );
    assert_eq!(walk("gf", Some(("iv", &elsewhere))), control);
    let here = with_rule("iv", vec![entry("CallDescent", &from, &to, Some(root))]);
    let report = walk("gf", Some(("iv", &here)));
    assert_eq!(report["boundary"][0]["reason"], "unproven_slot", "{report}");
}

#[test]
fn a_relation_mismatch_never_matches() {
    let control = off("gf");
    let (from, to) = hop_of(&control, "CallDescent");
    let wrong = with_rule("iv", vec![entry("ReturnFlow", &from, &to, None)]);
    assert_eq!(walk("gf", Some(("iv", &wrong))), control);
}

#[test]
fn a_hop_that_is_not_a_crossing_never_matches_whatever_the_rule() {
    let control = off("gf");
    let (from, to) = hop_of(&control, "DataFlow");
    let mut rules = empty_rules();
    for rule in RULES {
        for relation in ["CallDescent", "ReturnFlow", "ReturnInput"] {
            rules[rule]
                .as_array_mut()
                .unwrap()
                .push(entry(relation, &from, &to, None));
        }
    }
    assert_eq!(walk("gf", Some(("all", &file(rules)))), control);
}

#[test]
fn a_filed_return_input_is_a_cut_and_stops_the_ascent() {
    let control = off("w05");
    let (from, to) = hop_of(&control, "ReturnInput");
    let (_, flow_to) = hop_of(&control, "ReturnFlow");
    assert!(reached(&control, &flow_to));
    let filed = with_rule("return_input", vec![entry("ReturnInput", &from, &to, None)]);
    let report = walk("w05", Some(("return_input", &filed)));
    assert!(
        report["cuts"].as_array().unwrap().iter().any(|cut| {
            cut["reason"] == "unproven_return_input"
                && cut["relation"] == "ReturnInput"
                && cut["from"] == from.as_str()
                && cut["to"] == to.as_str()
        }),
        "{report}"
    );
    assert!(!reached(&report, &to) && !reached(&report, &flow_to));
    // Not switched on: the same file changes nothing.
    assert_eq!(walk("w05", Some(("return_flow", &filed))), control);
}

#[test]
fn a_filed_return_flow_is_refused_as_a_boundary() {
    let control = off("w05");
    let (from, to) = hop_of(&control, "ReturnFlow");
    let filed = with_rule("return_flow", vec![entry("ReturnFlow", &from, &to, None)]);
    let report = walk("w05", Some(("return_flow", &filed)));
    assert!(
        report["boundary"].as_array().unwrap().iter().any(|edge| {
            edge["reason"] == "return_not_entered_site"
                && edge["relation"] == "ReturnFlow"
                && edge["to"] == to.as_str()
        }),
        "{report}"
    );
    assert!(!reached(&report, &to));
}

#[test]
fn the_depth_zero_cfg_cut_wins_over_a_filed_return_input() {
    let control = off("w05v");
    let pruned = control["cuts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cut| cut["relation"] == "ReturnInput")
        .unwrap();
    assert_eq!(pruned["reason"], "cfg_pruned");
    let (from, to) = (
        pruned["from"].as_str().unwrap(),
        pruned["to"].as_str().unwrap(),
    );
    let filed = with_rule("return_input", vec![entry("ReturnInput", from, to, None)]);
    assert_eq!(walk("w05v", Some(("return_input", &filed))), control);
}

#[test]
fn an_exclusion_for_one_root_leaves_the_other_root_crossing() {
    let control = off("two");
    let (from, to) = hop_of(&control, "CallDescent");
    let by_root = control["frontier_by_root"].as_object().unwrap();
    let roots: Vec<&String> = by_root.keys().collect();
    assert_eq!(roots.len(), 2);
    let reaches = |report: &Value, root: &str| {
        report["frontier_by_root"][root]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node == to.as_str())
    };
    for filed_root in &roots {
        let filed = with_rule(
            "iv",
            vec![entry("CallDescent", &from, &to, Some(filed_root))],
        );
        let report = walk("two", Some(("iv", &filed)));
        assert_eq!(report["boundary"].as_array().unwrap().len(), 1, "{report}");
        for root in &roots {
            assert_eq!(reaches(&report, root), root != filed_root, "{report}");
        }
    }
}

#[test]
fn the_installed_configuration_is_scoped_to_the_closure() {
    let control = off("gf");
    let (from, to) = hop_of(&control, "CallDescent");
    let filed = with_rule("iv", vec![entry("CallDescent", &from, &to, None)]);
    let refused = walk("gf", Some(("iv", &filed)));
    assert!(!refused["boundary"].as_array().unwrap().is_empty());
    // Nothing leaks out of the closure.
    assert_eq!(walk_report("gf"), control);
}

// ---- loader ----------------------------------------------------------------------------------

fn os(text: &str) -> Option<OsString> {
    Some(OsString::from(text))
}

fn not_utf8() -> Option<OsString> {
    Some(OsString::from_vec(vec![b'i', 0xff]))
}

#[test]
fn no_rule_on_means_nothing_is_read_or_loaded() {
    let missing = os("/nonexistent/walk-proofs.json");
    for switches in [None, os(""), os("  "), os(",,"), os(" , "), os(",")] {
        for file in [None, missing.clone()] {
            assert!(WalkProofs::from_env_values(file, switches.clone())
                .unwrap()
                .is_none());
        }
    }
}

#[test]
fn a_value_that_is_not_utf8_names_its_variable() {
    let error = WalkProofs::from_env_values(None, not_utf8()).unwrap_err();
    assert!(
        error.contains("PRISM_TAINT_WALK_PROOFS:") && error.contains("UTF-8"),
        "{error}"
    );
    let error = WalkProofs::from_env_values(not_utf8(), None).unwrap_err();
    assert!(error.contains("PRISM_TAINT_WALK_PROOFS_FILE"), "{error}");
    let error = WalkProofs::from_env_values(not_utf8(), os("iv")).unwrap_err();
    assert!(
        error.contains("PRISM_TAINT_WALK_PROOFS_FILE") && error.contains("UTF-8"),
        "{error}"
    );
}

#[test]
fn a_rule_on_needs_a_readable_wellformed_file() {
    let error = WalkProofs::from_env_values(None, os("iv")).unwrap_err();
    assert!(error.contains("PRISM_TAINT_WALK_PROOFS_FILE"), "{error}");
    let error =
        WalkProofs::from_env_values(os("/nonexistent/walk-proofs.json"), os("iv")).unwrap_err();
    assert!(error.contains("cannot read"), "{error}");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("proofs.json");
    std::fs::write(&path, "{ not json").unwrap();
    let error = WalkProofs::from_env_values(Some(path.clone().into()), os("iv")).unwrap_err();
    assert!(error.contains("malformed"), "{error}");
    std::fs::write(
        &path,
        file_text(json!({"iv": [entry("CallDescent", "F", "T", None)]})),
    )
    .unwrap();
    let proofs = WalkProofs::from_env_values(Some(path.into()), os("iv"))
        .unwrap()
        .unwrap();
    assert_eq!(
        proofs.refusal_for_keys(super::Relation::CallDescent, "F", "T", "R"),
        Some(super::LossReason::UnprovenSlot)
    );
}

#[test]
fn the_summary_reports_kept_and_dropped_entries_per_rule() {
    let rules = json!({
        "iv": [
            entry("CallDescent", "F", "T", None),
            entry("CallDescent", "F2", "T2", None),
            entry("ReturnFlow", "F", "T", None),
        ],
        "return_flow": [entry("ReturnFlow", "F", "T", None)],
    });
    let proofs = WalkProofs::from_parts(Some(&file_text(rules)), Some("iv,return_flow"))
        .unwrap()
        .unwrap();
    assert_eq!(
        proofs.summary(),
        "taint-walk-proofs rule=iv kept=2 dropped_relation_mismatch=1\n\
         taint-walk-proofs rule=return_flow kept=1 dropped_relation_mismatch=0\n"
    );
}

#[test]
fn load_returns_the_proofs_and_panics_on_a_misconfiguration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("proofs.json");
    std::fs::write(&path, file_text(json!({"iv": []}))).unwrap();
    assert!(WalkProofs::load(Some(path.into()), os("iv")).is_some());
    assert!(WalkProofs::load(None, None).is_none());
    let panic = std::panic::catch_unwind(|| WalkProofs::load(None, os("iv"))).unwrap_err();
    let message = panic.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(
        message.contains("PRISM_TAINT_WALK_PROOFS_FILE"),
        "{message}"
    );
}
