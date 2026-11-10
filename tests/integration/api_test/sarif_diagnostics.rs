use super::*;
use prism::{data_flow::VarAccessKind, finding_confidence::MinConfidence};
use prism_review::algorithms::taint;
use std::collections::BTreeSet;

const CALLER: &str = "dir space/é\"#.js";

fn producer() -> (ReviewInputs, AlgorithmOutcome) {
    let source = format!(
        "import {{ relay }} from './transport';\nfunction h(input) {{\n const command = input;\n eval(command);\n{}\n}}",
        [" relay(command);"; 12].join("\n")
    );
    let lines: Vec<_> = std::iter::once(3).chain(5..=16).collect();
    let (_tmp, repo, diff) = write_repo(
        &[
            (CALLER, &source),
            ("dir space/transport.js", "export function relay(x) {}"),
        ],
        &[(CALLER, &lines)],
    );
    let opts = ReviewOptions::new(&repo);
    let inputs = load_review_inputs(&opts, &diff).unwrap();
    let built = build_context(&inputs, &opts).unwrap();
    let (_, trace) =
        taint::slice_with_trace(&built.ctx, &inputs.diff, &taint::TaintConfig::default()).unwrap();
    let sites: BTreeSet<_> = built
        .ctx
        .cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .filter(|s| s.callee_name == "relay")
        .map(|s| (s.line, s.start_byte, s.end_byte))
        .collect();
    let rows: Vec<_> = sites
        .iter()
        .map(|&(line, start, end)| {
            let call = inputs.files[CALLER]
                .tree
                .root_node()
                .descendant_for_byte_range(start, end);
            let argument = call
                .filter(|n| n.kind() == "call_expression" && n.byte_range() == (start..end))
                .and_then(|n| n.child_by_field_name("arguments"))
                .filter(|n| n.named_child_count() == 1)
                .and_then(|n| n.named_child(0));
            let exact_argument = argument.is_some_and(|n| &source[n.byte_range()] == "command");
            let retained = argument.is_some_and(|argument| {
                trace.roots.iter().any(|r| {
                    r.origin == taint::TaintRootOrigin::Diff
                        && r.location.file == CALLER
                        && r.location.line == line
                        && r.location.kind == VarAccessKind::Use
                        && r.location.start_byte == argument.start_byte()
                        && r.location.end_byte == argument.end_byte()
                })
            });
            (
                line,
                &source[start..end] == "relay(command)",
                exact_argument,
                retained,
            )
        })
        .collect();
    eprintln!("PRODUCER physical arguments={rows:?}");
    assert_eq!(rows.len(), 12, "producer needs twelve physical calls");
    assert!(
        rows.iter().all(|r| r.1 && r.2 && r.3),
        "producer requires every exact call/argument/Diff Use"
    );
    let outcome = run_algorithm_with_diagnostics(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        &repo,
    )
    .unwrap();
    assert!(
        outcome
            .result
            .findings
            .iter()
            .any(|f| f.category.as_deref() == Some("taint_sink")),
        "producer needs real sink"
    );
    assert!(
        outcome
            .result
            .evidence
            .iter()
            .flatten()
            .any(|e| !e.hops.is_empty()),
        "producer needs concrete evidence"
    );
    let d = outcome.diagnostics.as_ref().expect("producer diagnostics");
    assert_eq!(d.availability(), &ReviewBoundaryAvailability::Available);
    assert_eq!((d.located().len(), d.omitted_count()), (10, 2));
    assert!(d
        .located()
        .iter()
        .all(|n| n.file() == CALLER && &source[n.byte_range()] == "relay(command)"));
    drop(built);
    (inputs, outcome)
}

fn notifications(doc: &Value) -> Vec<Value> {
    doc["runs"][0]["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn without_notes(mut doc: Value) -> Value {
    let invocation = doc["runs"][0]["invocations"][0].as_object_mut().unwrap();
    if let Some(notes) = invocation.get_mut("toolExecutionNotifications") {
        notes
            .as_array_mut()
            .unwrap()
            .retain(|n| n["level"] != "note");
        if notes.as_array().unwrap().is_empty() {
            invocation.remove("toolExecutionNotifications");
        }
    }
    doc
}

#[test]
fn typed_notes_keep_caller_locations_and_exact_summary() {
    let (_, outcome) = producer();
    let d = outcome.diagnostics.as_ref().unwrap();
    let inputs = SarifInputs::new(&[]).boundary_diagnostics(d);
    let doc = to_sarif(&inputs);
    let mut expected: Vec<_> = d.located().iter().map(|note| serde_json::json!({
        "level": "note", "message": {"text": note.message()},
        "locations": [{"physicalLocation": {
            "artifactLocation": {"uri": "dir%20space/%C3%A9%22%23.js", "uriBaseId": "%SRCROOT%"},
            "region": {"startLine": note.line()}
        }}]
    })).collect();
    expected.push(serde_json::json!({"level": "note", "message": {"text": "taint-context: 2 more boundary calls not listed"}}));
    let observed = notifications(&doc);
    eprintln!(
        "LOCATED observed={} expected={} omitted={}",
        observed.len(),
        expected.len(),
        d.omitted_count()
    );
    assert_eq!(doc, to_sarif(&inputs));
    assert_eq!(without_notes(doc.clone()), to_sarif(&SarifInputs::new(&[])));
    assert_eq!(doc["runs"][0]["results"], serde_json::json!([]));
    assert_eq!(
        doc["runs"][0]["tool"]["driver"]["rules"],
        serde_json::json!([])
    );
    assert!(doc["runs"][0]["properties"].get("errors").is_none());
    assert_eq!(
        doc["runs"][0]["invocations"][0]["executionSuccessful"],
        true
    );
    assert_eq!(observed, expected);
}

#[test]
fn typed_notes_survive_finding_admission_and_builder_replacement() {
    let (loaded, outcome) = producer();
    let d = outcome.diagnostics.as_ref().unwrap();
    let mut findings = outcome.result.findings.clone();
    let mut evidence = outcome.result.evidence.clone();
    // Serializer-only unlabeled control; the real producer findings/evidence stay present.
    let mut unlabeled = findings[0].clone();
    unlabeled.description = "controlled unlabeled serializer row".into();
    unlabeled.severity = "info".into();
    findings.push(unlabeled);
    evidence.push(None);
    let before = (
        serde_json::to_value(&findings).unwrap(),
        format!("{evidence:?}"),
        d.clone(),
    );
    let empty = ReviewBoundaryDiagnostics::default();
    let mut rows = Vec::new();
    for floor in [MinConfidence::NameOnly, MinConfidence::Exact] {
        let plain = SarifInputs::new(&findings)
            .evidence(&evidence)
            .files(&loaded.files)
            .sources(&loaded.sources)
            .parse_quality(&loaded.parse_quality)
            .min_confidence(floor);
        let baseline = to_sarif(&plain);
        let controlled = baseline["runs"][0]["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["message"]["text"] == "controlled unlabeled serializer row");
        assert_eq!(
            controlled.is_some(),
            floor == MinConfidence::NameOnly,
            "admission precondition"
        );
        if let Some(row) = controlled {
            assert_eq!(row["properties"]["confidence"], "unlabeled");
        }
        let inputs = plain.boundary_diagnostics(d);
        let doc = to_sarif(&inputs);
        let notes: Vec<_> = notifications(&doc)
            .into_iter()
            .filter(|n| n["level"] == "note")
            .collect();
        let messages: Vec<_> = notes
            .iter()
            .map(|n| n["message"]["text"].as_str().unwrap().to_string())
            .collect();
        rows.push((
            floor,
            messages == d.messages(),
            notes.iter().all(|n| n["level"] == "note"),
        ));
        assert_eq!(without_notes(doc), baseline);
        assert_eq!(
            to_sarif(&inputs.boundary_diagnostics(&empty)),
            baseline,
            "builder replaces"
        );
    }
    eprintln!("ADMISSION rows={rows:?}");
    assert_eq!(
        before,
        (
            serde_json::to_value(&findings).unwrap(),
            format!("{evidence:?}"),
            d.clone()
        )
    );
    assert!(
        rows.iter().all(|r| r.1 && r.2),
        "notes independent of findings/floor"
    );
}
