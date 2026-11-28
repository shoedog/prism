use serde_json::{json, Value};

use super::object_literal_arg_tests::build;
use super::trace::{Relation, ReturnFlowMode};
use super::trace_descent_tests::occurrence;
use super::trace_proofs::WalkProofs;
use super::{CodePropertyGraph, LossReason, VarAccess};
use crate::languages::Language;

const SWITCHES: &str = "PRISM_TAINT_WALK_PROOFS";
const FILE: &str = "PRISM_TAINT_WALK_PROOFS_FILE";
const CASE: &str = "PRISM_PROOFS_CASE";
const SUMMARY: &str = "taint-walk-proofs ";
const GF: &str = "function g(x) { sink(x); }\nfunction f(t) {\n  g(t);\n}\n";
const W05: &str = include_str!("../../tests/taint_parity/w05/a.js");
const RULES: [&str; 7] = ["i", "ii", "iii", "iv", "v", "return_flow", "return_input"];

pub(super) fn file_text(rules: Value) -> String {
    json!({"version": 1, "rules": rules}).to_string()
}

pub(super) fn entry(relation: &str, from: &str, to: &str, root: Option<&str>) -> Value {
    match root {
        Some(root) => json!({"relation": relation, "from": from, "to": to, "root": root}),
        None => json!({"relation": relation, "from": from, "to": to}),
    }
}

fn parse(rules: Value, switches: &str) -> Result<Option<WalkProofs>, String> {
    WalkProofs::from_parts(Some(&file_text(rules)), Some(switches))
}

// ---- parse and validate ----------------------------------------------------------------------

#[test]
fn a_valid_file_refuses_only_the_filed_hop_with_its_rule_reason() {
    let hop = entry("CallDescent", "F", "T", None);
    let proofs = parse(json!({"iv": [hop]}), "iv").unwrap().unwrap();
    assert_eq!(
        proofs.refusal_for_keys(Relation::CallDescent, "F", "T", "R"),
        Some(LossReason::UnprovenSlot)
    );
    assert_eq!(
        proofs.refusal_for_keys(Relation::CallDescent, "F", "OTHER", "R"),
        None
    );
}

#[test]
fn each_rule_refuses_with_its_documented_reason() {
    for (rule, relation, reason) in [
        ("i", "CallDescent", LossReason::NoMatchingSite),
        ("ii", "CallDescent", LossReason::AmbiguousSite),
        ("iii", "CallDescent", LossReason::NotExact),
        ("iv", "CallDescent", LossReason::UnprovenSlot),
        ("v", "CallDescent", LossReason::AmbiguousSite),
        (
            "return_flow",
            "ReturnFlow",
            LossReason::ReturnNotEnteredSite,
        ),
        (
            "return_input",
            "ReturnInput",
            LossReason::UnprovenReturnInput,
        ),
    ] {
        let rel = match relation {
            "CallDescent" => Relation::CallDescent,
            "ReturnFlow" => Relation::ReturnFlow,
            _ => Relation::ReturnInput,
        };
        let proofs = parse(json!({rule: [entry(relation, "F", "T", None)]}), rule)
            .unwrap()
            .unwrap();
        assert_eq!(
            proofs.refusal_for_keys(rel, "F", "T", "R"),
            Some(reason),
            "{rule}"
        );
    }
}

#[test]
fn a_root_restricted_exclusion_applies_only_under_that_root() {
    let proofs = parse(
        json!({"iii": [entry("CallDescent", "F", "T", Some("R1"))]}),
        "iii",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        proofs.refusal_for_keys(Relation::CallDescent, "F", "T", "R1"),
        Some(LossReason::NotExact)
    );
    assert_eq!(
        proofs.refusal_for_keys(Relation::CallDescent, "F", "T", "R2"),
        None
    );
}

#[test]
fn the_first_matching_rule_in_order_supplies_the_reason() {
    let hop = entry("CallDescent", "F", "T", None);
    let proofs = parse(
        json!({"v": [hop.clone()], "iii": [hop.clone()], "iv": [hop]}),
        "v,iv,iii",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        proofs.refusal_for_keys(Relation::CallDescent, "F", "T", "R"),
        Some(LossReason::NotExact)
    );
}

#[test]
fn a_rule_only_matches_its_own_hop_relation() {
    // Filed under iv with a ReturnFlow relation: matches neither hop kind.
    let proofs = parse(json!({"iv": [entry("ReturnFlow", "F", "T", None)]}), "iv")
        .unwrap()
        .unwrap();
    for relation in [Relation::CallDescent, Relation::ReturnFlow] {
        assert_eq!(proofs.refusal_for_keys(relation, "F", "T", "R"), None);
    }
    // A CallDescent entry under return_flow never matches a ReturnFlow hop.
    let proofs = parse(
        json!({"return_flow": [entry("CallDescent", "F", "T", None)]}),
        "return_flow",
    )
    .unwrap()
    .unwrap();
    for relation in [Relation::CallDescent, Relation::ReturnFlow] {
        assert_eq!(proofs.refusal_for_keys(relation, "F", "T", "R"), None);
    }
    // A switched-on crossing rule never refuses a non-crossing relation.
    let proofs = parse(json!({"iv": [entry("DataFlow", "F", "T", None)]}), "iv");
    assert!(proofs.is_err(), "DataFlow is not a filable relation");
}

#[test]
fn all_switches_every_rule_on() {
    let hop = entry("CallDescent", "F", "T", None);
    let empty = json!([]);
    let rules = json!({
        "i": empty, "ii": empty, "iii": empty, "iv": [hop], "v": empty,
        "return_flow": empty, "return_input": empty,
    });
    let proofs = parse(rules.clone(), "all").unwrap().unwrap();
    assert_eq!(
        proofs.refusal_for_keys(Relation::CallDescent, "F", "T", "R"),
        Some(LossReason::UnprovenSlot)
    );
    // `all` requires every rule in the file.
    let mut missing = rules;
    missing.as_object_mut().unwrap().remove("v");
    let error = parse(missing, "all").unwrap_err();
    assert!(error.contains("`v`") && error.contains(SWITCHES), "{error}");
}

#[test]
fn off_means_nothing_applies_and_nothing_is_read() {
    let text = file_text(json!({"iv": [entry("CallDescent", "F", "T", None)]}));
    for switches in [None, Some(""), Some("  "), Some(" , ")] {
        assert!(WalkProofs::from_parts(Some(&text), switches)
            .unwrap()
            .is_none());
        assert!(WalkProofs::from_parts(None, switches).unwrap().is_none());
        assert!(WalkProofs::from_parts(Some("{ not json"), switches)
            .unwrap()
            .is_none());
    }
}

#[test]
fn misconfiguration_is_an_error_naming_the_variable() {
    let good = json!({"iv": [entry("CallDescent", "F", "T", None)]});
    let cases: Vec<(Result<Option<WalkProofs>, String>, &str)> = vec![
        (WalkProofs::from_parts(None, Some("iv")), FILE),
        (WalkProofs::from_parts(Some("{ not json"), Some("iv")), FILE),
        (
            WalkProofs::from_parts(
                Some(&json!({"version": 2, "rules": good.clone()}).to_string()),
                Some("iv"),
            ),
            "version",
        ),
        (
            WalkProofs::from_parts(
                Some(&json!({"rules": good.clone()}).to_string()),
                Some("iv"),
            ),
            FILE,
        ),
        (parse(json!({"iv": [], "vi": []}), "iv"), "`vi`"),
        (parse(good.clone(), "iv,zz"), SWITCHES),
        (parse(good.clone(), "iii"), "`iii`"),
        (
            parse(
                json!({"iv": [{"relation": "CallDescent", "from": "F"}]}),
                "iv",
            ),
            FILE,
        ),
        (
            parse(json!({"iv": [entry("Sideways", "F", "T", None)]}), "iv"),
            FILE,
        ),
        (parse(json!({"iv": "x"}), "iv"), FILE),
    ];
    for (index, (result, needle)) in cases.into_iter().enumerate() {
        let error = result.expect_err(&format!("case {index} must fail"));
        assert!(error.contains(needle), "case {index}: {error}");
    }
}

// ---- walk ------------------------------------------------------------------------------------

pub(super) fn keys(cpg: &CodePropertyGraph, nodes: &[petgraph::graph::NodeIndex]) -> Vec<String> {
    nodes.iter().map(|&node| cpg.trace_key(node)).collect()
}

/// Child process: one walk under whatever the environment says, reported as JSON on stdout.
#[test]
fn walk_child() {
    let Ok(case) = std::env::var(CASE) else {
        return;
    };
    let report = walk_report(&case);
    println!("<<<REPORT{report}REPORT>>>");
}

/// One walk of a named case under the current configuration, described as JSON.
pub(super) fn walk_report(case: &str) -> Value {
    const LOCAL: &str = "function h(a) {\n  const b = a;\n  sink(b);\n}\n";
    let (source, roots_at, mode, ordered) = match case {
        "gf" => (
            GF,
            vec![(GF.find("g(t)").unwrap() + 2, VarAccess::Use)],
            ReturnFlowMode::Off,
            true,
        ),
        "w05" => (
            W05,
            vec![(W05.find("f(y)").unwrap() + 2, VarAccess::Use)],
            ReturnFlowMode::On,
            true,
        ),
        // The seed on `v = p` is the depth-zero CFG-pruned case of the return expression.
        "w05v" => (
            W05,
            vec![(W05.find("v = p").unwrap(), VarAccess::Def)],
            ReturnFlowMode::On,
            false,
        ),
        // No crossing anywhere in the walk.
        "local" => (
            LOCAL,
            vec![(LOCAL.find("h(a").unwrap() + 2, VarAccess::Def)],
            ReturnFlowMode::Off,
            true,
        ),
        // Two roots of one invocation reaching the same descent hop (a shared gate cache).
        "two" => (
            GF,
            vec![
                (GF.find("g(t)").unwrap() + 2, VarAccess::Use),
                (GF.find("f(t)").unwrap() + 2, VarAccess::Def),
            ],
            ReturnFlowMode::Off,
            true,
        ),
        other => panic!("unknown case {other}"),
    };
    let cpg = build(Language::JavaScript, &[("a", source)], "js");
    let roots: Vec<_> = roots_at
        .iter()
        .map(|&(at, access)| occurrence(&cpg, at, access))
        .collect();
    let root = roots[0];
    let parsed = crate::ast::ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let files = [("a.js".into(), parsed)].into();
    let order = crate::reasoning::order::AstOrderView::new(&cpg, &files);
    let trace = cpg.taint_trace_nodes_with_mode(&roots, ordered.then_some(&order), mode);
    let key = |node| cpg.trace_key(node);
    let hops: Vec<Value> = trace
        .parents_by_root
        .iter()
        .map(|(&(_, child), &(parent, relation))| {
            json!([key(parent), key(child), format!("{relation:?}")])
        })
        .collect();
    let boundary: Vec<Value> = trace
        .boundary
        .iter()
        .map(|edge| {
            json!({"root": key(edge.root), "from": key(edge.from), "to": key(edge.to),
                   "reason": edge.reason.to_string(), "relation": format!("{:?}", edge.relation)})
        })
        .collect();
    let cuts: Vec<Value> = trace
        .cuts
        .iter()
        .map(|cut| {
            json!({"root": key(cut.root), "from": key(cut.from), "to": key(cut.to),
                   "reason": cut.reason.to_string(), "relation": format!("{:?}", cut.relation)})
        })
        .collect();
    let frontier = keys(
        &cpg,
        &trace.frontier_by_root[&root]
            .iter()
            .copied()
            .collect::<Vec<_>>(),
    );
    let by_root: serde_json::Map<String, Value> = roots
        .iter()
        .map(|&r| {
            let nodes: Vec<_> = trace.frontier_by_root[&r].iter().copied().collect();
            (key(r), json!(keys(&cpg, &nodes)))
        })
        .collect();
    let report = json!({
        "root": key(root), "hops": hops, "boundary": boundary, "cuts": cuts,
        "frontier": frontier, "frontier_by_root": by_root, "warnings": trace.warnings,
    });
    report
}

struct Run {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

impl Run {
    fn report(&self) -> Value {
        assert!(self.status.success(), "{}\n{}", self.stdout, self.stderr);
        let start = self.stdout.find("<<<REPORT").expect("no report") + 9;
        let end = self.stdout.find("REPORT>>>").expect("no report end");
        serde_json::from_str(&self.stdout[start..end]).unwrap()
    }

    /// The child's whole observable output: report and ledger.
    fn output(&self) -> (String, String) {
        (self.report().to_string(), self.stderr.clone())
    }

    /// The report and the stderr lines that are not the walk proofs' own load summary.
    fn behaviour(&self) -> (String, Vec<String>) {
        let lines = self
            .stderr
            .lines()
            .filter(|line| !line.starts_with(SUMMARY))
            .map(str::to_owned)
            .collect();
        (self.report().to_string(), lines)
    }
}

fn run(case: &str, file: Option<&Value>, switches: Option<&str>, ledger: bool) -> Run {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "cpg::trace_proofs_tests::walk_child",
            "--nocapture",
        ])
        .env(CASE, case)
        .env_remove(SWITCHES)
        .env_remove(FILE)
        .env_remove("PRISM_TAINT_CROSSING_TRACE");
    if let Some(file) = file {
        std::fs::write(temp.path(), file.to_string()).unwrap();
        command.env(FILE, temp.path());
    }
    if let Some(switches) = switches {
        command.env(SWITCHES, switches);
    }
    if ledger {
        command.env("PRISM_TAINT_CROSSING_TRACE", "1");
    }
    let out = command.output().unwrap();
    Run {
        status: out.status,
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// A child with exactly the given raw environment values for the two variables.
fn run_raw(case: &str, switches: Option<&std::ffi::OsStr>, file: Option<&std::ffi::OsStr>) -> Run {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "cpg::trace_proofs_tests::walk_child",
            "--nocapture",
        ])
        .env(CASE, case)
        .env_remove(SWITCHES)
        .env_remove(FILE)
        .env_remove("PRISM_TAINT_CROSSING_TRACE");
    if let Some(switches) = switches {
        command.env(SWITCHES, switches);
    }
    if let Some(file) = file {
        command.env(FILE, file);
    }
    let out = command.output().unwrap();
    Run {
        status: out.status,
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

pub(super) fn file(rules: Value) -> Value {
    json!({"version": 1, "rules": rules})
}

pub(super) fn hop_of(report: &Value, relation: &str) -> (String, String) {
    let hop = report["hops"]
        .as_array()
        .unwrap()
        .iter()
        .find(|hop| hop[2] == relation)
        .unwrap_or_else(|| panic!("no {relation} hop in {report}"));
    (
        hop[0].as_str().unwrap().to_string(),
        hop[1].as_str().unwrap().to_string(),
    )
}

pub(super) fn reached(report: &Value, key: &str) -> bool {
    report["frontier"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node == key)
}

pub(super) fn empty_rules() -> Value {
    RULES
        .iter()
        .map(|rule| (rule.to_string(), json!([])))
        .collect()
}

pub(super) fn with_rule(rule: &str, entries: Vec<Value>) -> Value {
    let mut rules = empty_rules();
    rules[rule] = Value::Array(entries);
    file(rules)
}

#[test]
fn a_switched_on_slot_exclusion_refuses_the_descent() {
    let control = run("gf", None, None, false).report();
    assert!(control["boundary"].as_array().unwrap().is_empty());
    let (from, to) = hop_of(&control, "CallDescent");
    assert!(reached(&control, &to));
    let root = control["root"].as_str().unwrap();
    assert_eq!(from, root);

    let filed = with_rule("iv", vec![entry("CallDescent", &from, &to, None)]);
    let refused = run("gf", Some(&filed), Some("iv"), false).report();
    let boundary = refused["boundary"].as_array().unwrap();
    assert_eq!(boundary.len(), 1, "{refused}");
    assert_eq!(boundary[0]["reason"], "unproven_slot");
    assert_eq!(boundary[0]["relation"], "CallDescent");
    assert_eq!(boundary[0]["from"], from.as_str());
    assert_eq!(boundary[0]["to"], to.as_str());
    assert!(!reached(&refused, &to), "{refused}");
}

#[test]
fn every_crossing_rule_refuses_with_its_reason() {
    let control = run("gf", None, None, false).report();
    let (from, to) = hop_of(&control, "CallDescent");
    for (rule, reason) in [
        ("i", "no_matching_site"),
        ("ii", "ambiguous_site"),
        ("iii", "not_exact"),
        ("iv", "unproven_slot"),
        ("v", "ambiguous_site"),
    ] {
        let filed = with_rule(rule, vec![entry("CallDescent", &from, &to, None)]);
        let refused = run("gf", Some(&filed), Some(rule), false).report();
        assert_eq!(refused["boundary"][0]["reason"], reason, "{rule}");
        assert!(!reached(&refused, &to), "{rule}");
    }
}

#[test]
fn the_ledger_key_equals_the_hop_key_once_the_node_index_is_dropped() {
    let control = run("gf", None, None, false).report();
    let (from, to) = hop_of(&control, "CallDescent");
    let filed = with_rule("iv", vec![entry("CallDescent", &from, &to, None)]);
    let refused = run("gf", Some(&filed), Some("iv"), true);
    let line = refused
        .stderr
        .lines()
        .find(|line| line.starts_with("taint-refusal "))
        .unwrap_or_else(|| panic!("no refusal line: {}", refused.stderr));
    assert!(line.contains("reason=unproven_slot"), "{line}");
    let strip = |text: &str| {
        let mut out = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '#' && chars.peek().is_some_and(char::is_ascii_digit) {
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    chars.next();
                }
            } else {
                out.push(c);
            }
        }
        out
    };
    // The ledger prints each identity with `{:?}`, so compare in that spelling.
    for (field, key) in [("from=", &from), ("to=", &to)] {
        let printed = format!("{key:?}");
        let at = line.find(&format!(" {field}")).unwrap() + field.len() + 1;
        let text = &line[at..];
        let end = text
            .find(" to=")
            .or_else(|| text.find(" relation="))
            .unwrap();
        assert_eq!(strip(&text[..end]), printed, "{field}");
    }
}

#[test]
fn a_rule_that_is_off_changes_nothing() {
    let control = run("gf", None, None, false);
    let (from, to) = hop_of(&control.report(), "CallDescent");
    let mut rules = empty_rules();
    rules["iv"] = json!([entry("CallDescent", &from, &to, None)]);
    let filed = file(rules);
    // (c) the filed rule is not switched on, another is.
    assert_eq!(
        run("gf", Some(&filed), Some("iii"), false).behaviour(),
        control.behaviour()
    );
    // (d) file set, switches unset or empty.
    assert_eq!(
        run("gf", Some(&filed), None, false).output(),
        control.output()
    );
    assert_eq!(
        run("gf", Some(&filed), Some(""), false).output(),
        control.output()
    );
}

#[test]
fn off_is_byte_identical_including_the_ledger() {
    for case in ["gf", "w05"] {
        let control = run(case, None, None, true).output();
        let report = run(case, None, None, true).report();
        let mut rules = empty_rules();
        for relation in ["CallDescent", "ReturnFlow", "ReturnInput"] {
            if let Some(hop) = report["hops"]
                .as_array()
                .unwrap()
                .iter()
                .find(|hop| hop[2] == relation)
            {
                for rule in RULES {
                    rules[rule].as_array_mut().unwrap().push(entry(
                        relation,
                        hop[0].as_str().unwrap(),
                        hop[1].as_str().unwrap(),
                        None,
                    ));
                }
            }
        }
        let filed = file(rules);
        for switches in [None, Some(""), Some(" ")] {
            assert_eq!(
                run(case, Some(&filed), switches, true).output(),
                control,
                "{case} {switches:?}"
            );
        }
        assert!(!control.1.is_empty(), "{case}: the ledger saw the walk");
    }
}

#[test]
fn a_root_restriction_must_name_the_walk_root() {
    let control = run("gf", None, None, false).report();
    let (from, to) = hop_of(&control, "CallDescent");
    let root = control["root"].as_str().unwrap().to_string();
    let unchanged = run("gf", None, None, false).behaviour();
    let other = with_rule(
        "iv",
        vec![entry(
            "CallDescent",
            &from,
            &to,
            Some("a.js:1:0..1@None:q:Use"),
        )],
    );
    assert_eq!(
        run("gf", Some(&other), Some("iv"), false).behaviour(),
        unchanged
    );
    let right = with_rule("iv", vec![entry("CallDescent", &from, &to, Some(&root))]);
    let refused = run("gf", Some(&right), Some("iv"), false).report();
    assert_eq!(refused["boundary"][0]["reason"], "unproven_slot");
}

#[test]
fn a_relation_mismatch_never_matches() {
    let control = run("gf", None, None, false).report();
    let (from, to) = hop_of(&control, "CallDescent");
    let unchanged = run("gf", None, None, false).behaviour();
    let wrong = with_rule("iv", vec![entry("ReturnFlow", &from, &to, None)]);
    assert_eq!(
        run("gf", Some(&wrong), Some("iv"), false).behaviour(),
        unchanged
    );
}

#[test]
fn a_hop_that_is_not_a_crossing_never_matches_whatever_the_rule() {
    let control = run("gf", None, None, false).report();
    let (from, to) = hop_of(&control, "DataFlow");
    let unchanged = run("gf", None, None, false).behaviour();
    let mut rules = empty_rules();
    for rule in RULES {
        for relation in ["CallDescent", "ReturnFlow", "ReturnInput"] {
            rules[rule]
                .as_array_mut()
                .unwrap()
                .push(entry(relation, &from, &to, None));
        }
    }
    let filed = file(rules);
    assert_eq!(
        run("gf", Some(&filed), Some("all"), false).behaviour(),
        unchanged
    );
}

#[test]
fn all_with_a_missing_rule_fails_loudly() {
    let filed = file(json!({}));
    let failed = run("gf", Some(&filed), Some("all"), false);
    assert!(!failed.status.success(), "{}", failed.stdout);
    assert!(failed.stderr.contains(SWITCHES), "{}", failed.stderr);
    // Switches without a file, and a malformed file, are as loud.
    let failed = run("gf", None, Some("iv"), false);
    assert!(!failed.status.success());
    assert!(failed.stderr.contains(FILE), "{}", failed.stderr);
}

#[test]
fn return_input_exclusion_cuts_the_return_expression() {
    let control = run("w05", None, None, false).report();
    let (from, to) = hop_of(&control, "ReturnInput");
    let (_, flow_to) = hop_of(&control, "ReturnFlow");
    assert!(reached(&control, &flow_to));

    let filed = with_rule("return_input", vec![entry("ReturnInput", &from, &to, None)]);
    let cut = run("w05", Some(&filed), Some("return_input"), false).report();
    let cuts = cut["cuts"].as_array().unwrap();
    assert!(
        cuts.iter().any(|c| c["reason"] == "unproven_return_input"
            && c["relation"] == "ReturnInput"
            && c["from"] == from.as_str()
            && c["to"] == to.as_str()),
        "{cut}"
    );
    assert!(!reached(&cut, &to), "{cut}");
    assert!(
        cut["hops"]
            .as_array()
            .unwrap()
            .iter()
            .all(|hop| hop[2] != "ReturnFlow"),
        "{cut}"
    );
    // Switched off: unchanged.
    let unchanged = run("w05", None, None, false).behaviour();
    let off = run(
        "w05",
        Some(&filed),
        Some("i,ii,iii,iv,v,return_flow"),
        false,
    );
    assert_eq!(off.behaviour(), unchanged);
}

#[test]
fn return_flow_exclusion_refuses_the_ascent() {
    let control = run("w05", None, None, false).report();
    let (from, to) = hop_of(&control, "ReturnFlow");
    let filed = with_rule("return_flow", vec![entry("ReturnFlow", &from, &to, None)]);
    let refused = run("w05", Some(&filed), Some("return_flow"), false).report();
    let boundary = refused["boundary"].as_array().unwrap();
    assert!(
        boundary
            .iter()
            .any(|b| b["reason"] == "return_not_entered_site"
                && b["relation"] == "ReturnFlow"
                && b["from"] == from.as_str()
                && b["to"] == to.as_str()),
        "{refused}"
    );
    assert!(!reached(&refused, &to), "{refused}");
    // The return expression itself is still traversed.
    assert!(refused["hops"]
        .as_array()
        .unwrap()
        .iter()
        .any(|hop| hop[2] == "ReturnInput"));
}

#[test]
fn a_misconfiguration_is_detected_even_when_no_hop_reaches_the_hook() {
    // `local` has no crossing at all, so only a check at walk entry can see the error.
    let failed = run("local", None, Some("iv"), false);
    assert!(!failed.status.success(), "{}", failed.stdout);
    assert!(failed.stderr.contains(FILE), "{}", failed.stderr);
    let missing = file(json!({"i": []}));
    let failed = run("local", Some(&missing), Some("iv"), false);
    assert!(!failed.status.success(), "{}", failed.stdout);
    assert!(failed.stderr.contains("`iv`"), "{}", failed.stderr);
    // A correct configuration walks the same fixture without refusing anything.
    let good = file(json!({"iv": []}));
    let ok = run("local", Some(&good), Some("iv"), false);
    assert!(ok.report()["boundary"].as_array().unwrap().is_empty());
}

#[test]
fn the_env_loader_reads_the_switches_as_from_parts_does() {
    use std::ffi::OsStr;
    let control = run("gf", None, None, false).behaviour();
    // Only separators: off, so the (missing) file is never read.
    for switches in [",,", " , ", ","] {
        let run = run_raw(
            "gf",
            Some(OsStr::new(switches)),
            Some(OsStr::new("/nonexistent/walk-proofs.json")),
        );
        assert_eq!(run.behaviour(), control, "{switches:?}");
    }
    // A switch that is on needs a readable file.
    let failed = run_raw(
        "gf",
        Some(OsStr::new(" i ")),
        Some(OsStr::new("/nonexistent/walk-proofs.json")),
    );
    assert!(!failed.status.success());
    assert!(failed.stderr.contains(FILE), "{}", failed.stderr);
}

#[test]
fn a_value_that_is_not_utf8_is_an_error_naming_its_variable() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let bad = OsStr::from_bytes(&[b'i', 0xff]);
    let failed = run_raw("gf", Some(bad), None);
    assert!(!failed.status.success(), "{}", failed.stdout);
    assert!(
        failed.stderr.contains(SWITCHES) && failed.stderr.contains("UTF-8"),
        "{}",
        failed.stderr
    );
    let failed = run_raw("gf", Some(OsStr::new("iv")), Some(bad));
    assert!(!failed.status.success(), "{}", failed.stdout);
    assert!(
        failed.stderr.contains(FILE) && failed.stderr.contains("UTF-8"),
        "{}",
        failed.stderr
    );
}

#[test]
fn a_filed_return_input_does_not_hide_the_depth_zero_cfg_cut() {
    let control = run("w05v", None, None, false).report();
    let pruned = control["cuts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cut| cut["relation"] == "ReturnInput")
        .unwrap_or_else(|| panic!("no ReturnInput cut in {control}"));
    assert_eq!(pruned["reason"], "cfg_pruned", "{control}");
    let (from, to) = (
        pruned["from"].as_str().unwrap(),
        pruned["to"].as_str().unwrap(),
    );
    let filed = with_rule("return_input", vec![entry("ReturnInput", from, to, None)]);
    let report = run("w05v", Some(&filed), Some("return_input"), false).report();
    let cuts = report["cuts"].as_array().unwrap();
    assert!(
        cuts.iter().any(|cut| cut["relation"] == "ReturnInput"
            && cut["from"] == from
            && cut["to"] == to
            && cut["reason"] == "cfg_pruned"),
        "{report}"
    );
    assert!(
        cuts.iter()
            .all(|cut| cut["reason"] != "unproven_return_input"),
        "{report}"
    );
}

#[test]
fn an_exclusion_for_one_root_leaves_the_other_root_crossing_in_one_invocation() {
    let control = run("two", None, None, false).report();
    let (from, to) = hop_of(&control, "CallDescent");
    let by_root = control["frontier_by_root"].as_object().unwrap();
    let roots: Vec<&String> = by_root.keys().collect();
    assert_eq!(roots.len(), 2, "{control}");
    assert!(
        roots.iter().all(|root| by_root[*root]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node == to.as_str())),
        "both roots reach the formal in the control: {control}"
    );
    for filed_root in &roots {
        let filed = with_rule(
            "iv",
            vec![entry("CallDescent", &from, &to, Some(filed_root))],
        );
        let report = run("two", Some(&filed), Some("iv"), false).report();
        let boundary = report["boundary"].as_array().unwrap();
        assert_eq!(boundary.len(), 1, "{report}");
        assert_eq!(boundary[0]["root"], filed_root.as_str(), "{report}");
        for root in &roots {
            let crossed = report["frontier_by_root"][root.as_str()]
                .as_array()
                .unwrap()
                .iter()
                .any(|node| node == to.as_str());
            assert_eq!(
                crossed,
                root != filed_root,
                "{root} filed={filed_root}: {report}"
            );
        }
    }
}

#[test]
fn switched_on_rules_report_their_kept_and_dropped_entries_at_load() {
    let rules = json!({
        "iv": [
            entry("CallDescent", "F", "T", None),
            entry("CallDescent", "F2", "T2", None),
            entry("ReturnFlow", "F", "T", None),
        ],
        "return_flow": [entry("ReturnFlow", "F", "T", None)],
        "iii": [entry("CallDescent", "F", "T", None)],
    });
    let report = run("local", Some(&file(rules)), Some("iv,return_flow"), false);
    let lines: Vec<_> = report
        .stderr
        .lines()
        .filter(|line| line.starts_with(SUMMARY))
        .collect();
    assert_eq!(
        lines,
        [
            "taint-walk-proofs rule=iv kept=2 dropped_relation_mismatch=1",
            "taint-walk-proofs rule=return_flow kept=1 dropped_relation_mismatch=0",
        ],
        "{}",
        report.stderr
    );
    // Off: no line at all.
    let off = run("local", None, None, true);
    assert!(!off.stderr.contains(SUMMARY), "{}", off.stderr);
}
