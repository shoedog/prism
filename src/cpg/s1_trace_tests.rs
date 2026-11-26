//! The opt-in S1 trace: events, key fields and the suppression baseline.
//!
//! Capture is process-wide and other tests may deposit records meanwhile, so every
//! assertion selects records by this module's own fixture file names (`s1t_*`).

use super::object_literal_arg_tests::build;
use super::s1_trace::capture::{self, Capture};
use super::s1_trace::{read_baseline_for_test, suppression_for_test};
use super::CodePropertyGraph;
use crate::call_graph::{CallSite, FunctionId};
use crate::languages::Language;
use serde_json::Value;

const RETURN_USED: &str = "function C({v}) {\n  observe(v);\n  return v + 1;\n}\n\nfunction caller() {\n  const out = C({v: 7});\n  return out;\n}\n";
const RETURN_CONSTANT: &str = "function C({v}) {\n  observe(v);\n  return 0;\n}\n\nfunction caller() {\n  const out = C({v: 7});\n  return out;\n}\n";
const ARG_PATTERN: &str = "function sink(value) {\n  return value;\n}\nfunction C({v}) {\n  observe(v);\n  return sink(v);\n}\n";
const ARG_LITERAL: &str = "function sink(value) {\n  return value;\n}\nfunction C({v}) {\n  observe(v);\n  return sink(0);\n}\n";
const BARE_AND_MEMBER: &str = "function C({o}) {\n  use(o);\n  return o.k;\n}\n";
const MEMBER_ONLY: &str = "function C({o}) {\n  return o.k;\n}\n";

fn run(cap: &Capture, stem: &str, source: &str) -> (CodePropertyGraph, Vec<Value>) {
    let graph = build(Language::TypeScript, &[(stem, source)], "ts");
    let file = format!("{stem}.ts");
    let events = cap
        .events()
        .into_iter()
        .filter(|event| mentions(event, &file))
        .collect();
    (graph, events)
}

fn mentions(event: &Value, file: &str) -> bool {
    event.to_string().contains(&format!("\"{file}\""))
}

fn named<'a>(events: &'a [Value], name: &str) -> Vec<&'a Value> {
    events.iter().filter(|e| e["event"] == name).collect()
}

#[test]
fn return_input_and_suppression_state_report_the_callee_pattern_flow() {
    let cap = capture::start();
    let (_, events) = run(&cap, "s1t_ret_used", RETURN_USED);
    let inputs = named(&events, "return_input");
    assert_eq!(inputs.len(), 1, "{events:?}");
    let input = inputs[0];
    assert_eq!(input["counter"], "return_input_edges");
    assert_eq!(input["use"]["Variable"]["start_byte"], 41);
    assert_eq!(input["use"]["Variable"]["access"], "Use");
    assert_eq!(input["return_value"]["ReturnValue"]["start_byte"], 41);
    assert_eq!(input["return_value"]["ReturnValue"]["end_byte"], 46);
    let defs = input["reaching_defs"].as_array().unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0]["def"]["Variable"]["start_byte"], 12);
    assert_eq!(defs[0]["label"], "Exact");

    let states = named(&events, "suppression_state");
    assert_eq!(states.len(), 1, "{events:?}");
    assert_eq!(states[0]["call"]["callee"]["name"], "C");
    assert_eq!(states[0]["call"]["caller"]["name"], "caller");
    assert_eq!(states[0]["call"]["bytes"], serde_json::json!([85, 94]));
    assert_eq!(
        states[0]["reasons"],
        serde_json::json!(["return_flow_suppression_void_unbound_uses"])
    );
}

#[test]
fn a_constant_return_emits_suppression_state_but_no_return_input() {
    let cap = capture::start();
    let (_, events) = run(&cap, "s1t_ret_const", RETURN_CONSTANT);
    assert!(named(&events, "return_input").is_empty(), "{events:?}");
    assert_eq!(named(&events, "suppression_state").len(), 1, "{events:?}");
}

#[test]
fn final_stats_are_reported_with_the_build_identity() {
    let cap = capture::start();
    let (graph, _) = run(&cap, "s1t_final", RETURN_USED);
    super::s1_trace::report_final(&graph);
    let all = cap.events();
    let finals = named(&all, "final_return_flow_stats");
    let ours = finals
        .iter()
        .find(|e| e["stats"]["return_input_edges"] == 1 && e["stats"]["return_flow_edges"] == 1)
        .unwrap_or_else(|| panic!("{finals:?}"));
    assert_eq!(
        ours["build"],
        crate::cpg_cache::current_cache_build_identity()
    );
    assert_eq!(
        ours["stats"]["return_flow_suppression_void_unbound_uses"],
        1
    );
}

#[test]
fn an_ordinary_argument_from_a_pattern_local_is_reported() {
    let cap = capture::start();
    let (_, events) = run(&cap, "s1t_arg", ARG_PATTERN);
    let args = named(&events, "ordinary_argument");
    assert_eq!(args.len(), 1, "{events:?}");
    let arg = args[0];
    assert_eq!(arg["caller"]["name"], "C");
    assert_eq!(arg["slot"], 0);
    assert_eq!(arg["call_bytes"], serde_json::json!([82, 89]));
    assert_eq!(arg["argument_bytes"], serde_json::json!([87, 88]));
    assert_eq!(arg["use"]["Variable"]["function"], "C");
    assert_eq!(arg["callee_def"]["Variable"]["function"], "sink");
    assert_eq!(arg["callee_def"]["Variable"]["access"], "Def");
    let defs = arg["reaching_defs"].as_array().unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0]["def"]["Variable"]["start_byte"], 53);
}

#[test]
fn a_literal_argument_reports_no_ordinary_argument() {
    let cap = capture::start();
    let (_, events) = run(&cap, "s1t_lit", ARG_LITERAL);
    assert!(named(&events, "ordinary_argument").is_empty(), "{events:?}");
}

#[test]
fn a_bare_occurrence_demotes_the_projection_and_is_reported() {
    let cap = capture::start();
    let (_, events) = run(&cap, "s1t_bare", BARE_AND_MEMBER);
    let demotions = named(&events, "projection_demotion");
    assert_eq!(demotions.len(), 1, "{events:?}");
    let demotion = demotions[0];
    assert_eq!(demotion["file"], "s1t_bare.ts");
    assert_eq!(demotion["grammar"], "TypeScript");
    assert_eq!(demotion["source_digest"].as_str().unwrap().len(), 64);
    assert_eq!(
        demotion["attempt"]["projection_causes"],
        serde_json::json!(["bare_occurrence"])
    );
    assert_eq!(demotion["attempt"]["disposition"], "emitted");
    assert_eq!(demotion["attempt"]["key"]["path"]["fields"][0], "k");
}

#[test]
fn a_member_only_formal_reports_no_projection_demotion() {
    let cap = capture::start();
    let (_, events) = run(&cap, "s1t_member", MEMBER_ONLY);
    assert!(
        named(&events, "projection_demotion").is_empty(),
        "{events:?}"
    );
}

/// One real call site (and the callee id it resolves to) from `RETURN_USED`.
fn site_and_callee(cap: &Capture) -> (CallSite, FunctionId) {
    let (graph, _) = run(cap, "s1t_base", RETURN_USED);
    let site = graph
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "C")
        .unwrap()
        .clone();
    let callee = FunctionId {
        file: "s1t_base.ts".into(),
        name: "C".into(),
        start_line: 1,
        end_line: 4,
    };
    (site, callee)
}

fn baseline_file(
    site: &CallSite,
    callee: &FunctionId,
    reasons: &[&str],
) -> tempfile::NamedTempFile {
    let row = serde_json::json!({
        "event": "suppression_state",
        "call": {
            "caller": site.caller,
            "bytes": [site.start_byte, site.end_byte],
            "callee": callee,
        },
        "reasons": reasons,
    });
    let mut file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(
        &mut file,
        format!("noise line\nPRISM_S1 {{\"event\":\"other\"}}\nPRISM_S1 {row}\n").as_bytes(),
    )
    .unwrap();
    file
}

fn transitions(cap: &Capture) -> Vec<Value> {
    let all = cap.events();
    named(&all, "suppression_transition")
        .into_iter()
        .filter(|e| e["call"]["callee"]["file"] == "s1t_base.ts")
        .cloned()
        .collect()
}

#[test]
fn a_baseline_flag_that_turned_on_is_a_positive_transition() {
    let cap = capture::start();
    let (site, callee) = site_and_callee(&cap);
    let file = baseline_file(&site, &callee, &[]);
    let base = read_baseline_for_test(file.path()).unwrap();
    // Now void by unbound uses only: complete returns, unbound RHS uses.
    suppression_for_test(Some(&base), &site, &callee, (false, true, false));
    let found = transitions(&cap);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0]["counter"],
        "return_flow_suppression_void_unbound_uses"
    );
    assert_eq!(found[0]["delta"], 1);
    assert_eq!(found[0]["old_reason"], serde_json::json!([]));
    assert_eq!(
        found[0]["new_reason"],
        serde_json::json!(["return_flow_suppression_void_unbound_uses"])
    );
}

#[test]
fn a_baseline_flag_that_turned_off_is_a_negative_transition_for_each_flag() {
    let cap = capture::start();
    let (site, callee) = site_and_callee(&cap);
    let file = baseline_file(
        &site,
        &callee,
        &["return_flow_suppression_void_incomplete_returns"],
    );
    let base = read_baseline_for_test(file.path()).unwrap();
    // Now certified: certified turns on (+1), incomplete_returns turns off (-1).
    suppression_for_test(Some(&base), &site, &callee, (true, true, true));
    let found = transitions(&cap);
    let by_counter: std::collections::BTreeMap<_, _> = found
        .iter()
        .map(|e| {
            (
                e["counter"].as_str().unwrap().to_owned(),
                e["delta"].clone(),
            )
        })
        .collect();
    assert_eq!(
        by_counter,
        [
            ("return_flow_suppression_certified".to_owned(), 1.into()),
            (
                "return_flow_suppression_void_incomplete_returns".to_owned(),
                (-1).into()
            ),
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn an_unchanged_or_missing_baseline_row_gives_the_right_transitions() {
    let cap = capture::start();
    let (site, callee) = site_and_callee(&cap);
    let file = baseline_file(
        &site,
        &callee,
        &["return_flow_suppression_void_unbound_uses"],
    );
    let base = read_baseline_for_test(file.path()).unwrap();
    suppression_for_test(Some(&base), &site, &callee, (false, true, false));
    assert!(
        transitions(&cap).is_empty(),
        "same state must not transition"
    );

    // A call absent from the baseline reads as "no reasons before".
    let other = FunctionId {
        name: "D".into(),
        ..callee.clone()
    };
    suppression_for_test(Some(&base), &site, &other, (true, true, true));
    let found = transitions(&cap);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["counter"], "return_flow_suppression_certified");
    assert_eq!(found[0]["delta"], 1);

    // No baseline at all: only the state event. Select by this fixture's own call key,
    // since a concurrent build may deposit unrelated records.
    let ours = |events: &[Value]| -> Vec<Value> {
        events
            .iter()
            .filter(|e| {
                e["call"]["callee"] == serde_json::json!(callee)
                    && e["call"]["bytes"] == serde_json::json!([site.start_byte, site.end_byte])
            })
            .cloned()
            .collect()
    };
    let before = ours(&cap.events()).len();
    suppression_for_test(None, &site, &callee, (true, true, true));
    let after = ours(&cap.events());
    assert_eq!(after.len(), before + 1, "{after:?}");
    assert_eq!(after.last().unwrap()["event"], "suppression_state");
}

#[test]
fn baseline_reading_rejects_duplicates_bad_json_and_unreadable_files() {
    let cap = capture::start();
    let (site, callee) = site_and_callee(&cap);
    let dup = baseline_file(&site, &callee, &[]);
    let text = std::fs::read_to_string(dup.path()).unwrap();
    let row = text.lines().last().unwrap().to_owned();
    std::fs::write(dup.path(), format!("{text}{row}\n")).unwrap();
    let error = read_baseline_for_test(dup.path()).unwrap_err();
    assert!(error.contains("duplicate suppression state"), "{error}");

    std::fs::write(dup.path(), "PRISM_S1 {not json\n").unwrap();
    assert!(read_baseline_for_test(dup.path()).is_err());

    std::fs::write(dup.path(), "PRISM_S1 {\"event\":\"suppression_state\"}\n").unwrap();
    assert!(
        read_baseline_for_test(dup.path()).is_err(),
        "missing fields"
    );

    let missing = dup.path().with_extension("absent");
    assert!(read_baseline_for_test(&missing).is_err());

    std::fs::write(dup.path(), "no prefix here\n").unwrap();
    assert!(read_baseline_for_test(dup.path()).unwrap().is_empty());
}

const ROWS: &str = include_str!("../../tests/fixtures/props_matrix/rows.jsonl");

#[derive(serde::Deserialize)]
struct MatrixRow {
    langs: Vec<String>,
    files: std::collections::BTreeMap<String, String>,
}

/// Every matrix fixture, built per case with a JS-family grammar.
fn matrix_graphs() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (index, line) in ROWS.lines().enumerate() {
        let row: MatrixRow = serde_json::from_str(line).unwrap();
        for ext in row.langs.iter().filter(|ext| {
            Language::from_path(&format!("x.{ext}")).is_some_and(|l| l.is_js_family())
        }) {
            for (case, source) in &row.files {
                out.push((format!("s1t_m{index}/{case}.{ext}"), source.clone()));
            }
        }
    }
    out
}

fn snapshot(name: &str, source: &str) -> Option<Vec<String>> {
    let language = Language::from_path(name)?;
    let parsed = crate::ast::ParsedFile::parse(name, source, language).ok()?;
    let files = std::collections::BTreeMap::from([(name.to_owned(), parsed)]);
    let graph = CodePropertyGraph::build(&files);
    let mut rows: Vec<_> = graph
        .graph
        .edge_indices()
        .map(|edge| {
            let (from, to) = graph.graph.edge_endpoints(edge).unwrap();
            format!(
                "{:?} -> {:?} {:?}",
                graph.graph[from], graph.graph[to], graph.graph[edge]
            )
        })
        .collect();
    rows.sort();
    Some(rows)
}

/// Tracing observes the producer through the evidence seam; it must never change a
/// graph. Builds every matrix fixture untraced, then traced, and compares them; the
/// traced pass also has to have produced evidence (demotions and return inputs).
#[test]
fn tracing_every_matrix_fixture_leaves_the_graph_unchanged() {
    let fixtures = matrix_graphs();
    assert!(fixtures.len() > 500, "{}", fixtures.len());
    let off: Vec<_> = {
        let _quiet = capture::quiet();
        fixtures
            .iter()
            .map(|(name, source)| snapshot(name, source))
            .collect()
    };
    let cap = capture::start();
    let on: Vec<_> = fixtures
        .iter()
        .map(|(name, source)| snapshot(name, source))
        .collect();
    let events = cap.events();
    let mut diverged = Vec::new();
    for ((name, _), (a, b)) in fixtures.iter().zip(off.iter().zip(&on)) {
        if a != b {
            diverged.push(name.clone());
        }
    }
    assert!(diverged.is_empty(), "{diverged:?}");
    let ours = |event: &&Value| event.to_string().contains("s1t_m");
    let count = |name| {
        events
            .iter()
            .filter(|e| e["event"] == name)
            .filter(ours)
            .count()
    };
    assert!(count("projection_demotion") > 0);
    assert!(count("return_input") > 0);
    assert!(count("suppression_state") > 0);
    assert!(count("ordinary_argument") > 0);
}

/// A capture another test starts between fact collection and replay must not misalign
/// the replay: it used to index an empty site list (`trace` true, nothing collected).
#[test]
fn tracing_switched_on_between_collection_and_replay_does_not_panic() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let cap = capture::quiet();
    let fired = Arc::new(AtomicBool::new(false));
    let flag = fired.clone();
    cap.after_collect("s1t_flip.ts", move || {
        flag.store(true, Ordering::SeqCst);
        capture::switch_on();
    });
    let graph = build(Language::TypeScript, &[("s1t_flip", RETURN_USED)], "ts");
    // The seam must really have been hit, or the test proves nothing.
    assert!(fired.load(Ordering::SeqCst), "hook never fired");
    assert!(graph.return_flow_stats.return_flow_edges > 0);
    // Collection saw tracing off, so replay emits nothing for this build.
    let ours: Vec<_> = cap
        .events()
        .into_iter()
        .filter(|e| mentions(e, "s1t_flip.ts"))
        .filter(|e| e["event"] == "suppression_state")
        .collect();
    assert!(ours.is_empty(), "{ours:?}");
}

/// A hook bound to one fixture file is not taken by another build, and stays pending.
#[test]
fn a_hook_for_another_fixture_does_not_fire_in_this_build() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let cap = capture::quiet();
    let fired = Arc::new(AtomicBool::new(false));
    let flag = fired.clone();
    cap.after_collect("s1t_not_built.ts", move || {
        flag.store(true, Ordering::SeqCst);
        capture::switch_on();
    });
    build(
        Language::TypeScript,
        &[("s1t_flip_other", RETURN_USED)],
        "ts",
    );
    assert!(!fired.load(Ordering::SeqCst), "fired in an unrelated build");
    assert!(
        matches!(super::s1_trace::capture_mode_for_test(), capture::Mode::Off),
        "the hold's state was changed by an unrelated build"
    );
}

/// The selection rule is pure: a held capture overrides the environment either way.
#[test]
fn a_held_capture_overrides_the_environment() {
    use super::s1_trace::select;
    use capture::Mode::{Default, Off, On};
    assert!(!select(Off, true));
    assert!(!select(Off, false));
    assert!(select(On, true));
    assert!(select(On, false));
    assert!(select(Default, true), "no holder: the environment decides");
    assert!(!select(Default, false));
}

/// Through the real global, while the holder lock is held (so no sibling can change it).
#[test]
fn the_control_is_off_and_the_capture_is_on_while_held() {
    use super::s1_trace::enabled_from_for_test as enabled_from;
    {
        let _quiet = capture::quiet();
        assert!(!enabled_from(true));
        assert!(!enabled_from(false));
    }
    let _cap = capture::start();
    assert!(enabled_from(false));
    assert!(enabled_from(true));
}
