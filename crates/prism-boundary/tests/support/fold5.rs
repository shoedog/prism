use crate::fold2::{admission, impact};
use crate::fold3::historical_computed;
use prism_boundary::revision::{AdmissionReport, ClaimDecision};
use serde_json::{json, Value};

fn refused<'a>(report: &'a AdmissionReport, id: &str, code: &str) -> &'a ClaimDecision {
    let claim = &report.claims[id];
    assert_eq!(claim.status, "unchecked", "{id}: {report:?}");
    assert!(
        claim.reasons.iter().any(|reason| reason.code == code),
        "{claim:?}"
    );
    claim
}

fn pin(rev: &str, cutoff: &Value) -> Value {
    json!({"kind":"historical_pin","branch":"main","rev":rev,
        "available_at":cutoff,"evidence":"archive"})
}

#[test]
fn admission_depends_on_the_topic_heads_own_selection() {
    let (mut request, _) = historical_computed();
    let cutoff = request["cutoff_at"].clone();
    let topic = request["inputs"].as_array().unwrap().len() - 1;
    request["inputs"][topic]["selection"] = pin("topic-head-rev", &cutoff);
    let control = admission(request.clone());
    assert_eq!(
        control.claims["effective"].status, "admitted",
        "{control:?}"
    );

    request["inputs"][topic]["selection"] = pin("wrong-rev", &cutoff);
    let report = admission(request);
    assert_eq!(report.claims["effective"].status, "unchecked", "{report:?}");
    refused(&report, "effective", "missing_dependency");
}

fn frozen_control() -> Value {
    let (_, mut wire) = historical_computed();
    wire["change"]["producer"]["after_input_id"] = json!("topic-head");
    wire
}

#[test]
fn frozen_cutoff_depends_on_the_input_that_holds_the_merge_receipt() {
    let mut wire = frozen_control();
    let control = impact(wire.clone()).validate();
    assert_eq!(
        control.claims["report-cutoff"].status, "admitted",
        "{control:?}"
    );

    let after = wire["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|input| input["input_id"] == "service-after")
        .unwrap();
    after["dependencies"] = json!(["missing"]);
    let report = impact(wire).validate();
    refused(&report, "input:service-after", "missing_dependency");
    refused(&report, "report-cutoff", "missing_dependency");
}

#[test]
fn frozen_head_candidates_are_role_checked() {
    let (_, mut wire) = historical_computed();
    assert_eq!(
        impact(wire.clone()).validate().claims["report-cutoff"].status,
        "admitted"
    );
    wire["change"]["subject_role"] = json!("both");
    wire["change"]["consumer"] =
        json!({"before_input_id":"topic-head","after_input_id":"topic-head"});
    wire["change"]["selection"]["proposed_head"] = json!("different-topic-rev");
    let report = impact(wire).validate();
    refused(&report, "slots:consumer", "slot_mismatch");
    refused(&report, "report-cutoff", "missing_cutoff");
}
