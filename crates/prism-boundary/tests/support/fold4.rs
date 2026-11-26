use crate::doc_cases::json_fixture;
use crate::fold2::{admission, admission_with_bytes, impact, media_request};
use crate::fold3::historical_computed;
use prism_boundary::admit_schema_input;
use prism_boundary::revision::{AdmissionReport, ClaimDecision};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn refused<'a>(report: &'a AdmissionReport, id: &str, code: &str) -> &'a ClaimDecision {
    let claim = &report.claims[id];
    assert_eq!(claim.status, "unchecked", "{id}: {claim:?}");
    assert!(
        claim.reasons.iter().any(|reason| reason.code == code),
        "{claim:?}"
    );
    assert!(report.counts.contains_key(code), "{report:?}");
    claim
}

#[test]
fn report_cutoff_requires_admitted_topic_head() {
    let (_, mut wire) = historical_computed();
    let mut evidence = wire["inputs"][0].clone();
    evidence["input_id"] = json!("head-evidence");
    evidence["origin"]["input_id"] = json!("head-evidence");
    wire["inputs"][2]["dependencies"] = json!(["head-evidence"]);
    wire["inputs"].as_array_mut().unwrap().push(evidence);
    let control = impact(wire.clone()).validate();
    assert_eq!(control.claims["input:topic-head"].status, "admitted");
    assert_eq!(control.claims["report-cutoff"].status, "admitted");

    wire["inputs"].as_array_mut().unwrap().pop();
    let result = impact(wire).validate();
    refused(&result, "input:topic-head", "missing_dependency");
    let cutoff = refused(&result, "report-cutoff", "missing_dependency");
    assert_eq!(
        cutoff.reasons[0].record_id.as_deref(),
        Some("input:topic-head")
    );
}

fn consumer_computed() -> (Value, Value) {
    let (mut request, mut wire) = historical_computed();
    for index in [2, 4] {
        let slot = &mut request["slots"][index];
        slot.as_object_mut().unwrap().remove("service_side");
        slot["role"] = json!("consumer");
        slot["authority"] = json!("calls");
        slot["repo"] = json!("pilot-consumer");
        slot["config"] = Value::Null;
        if index == 2 {
            slot["slot"] = json!("client-after");
            slot["input_id"] = json!("client-after");
        }
        request["inputs"][index]["binding"] = slot.clone();
    }
    request["merge"]["base"] = json!("client-before-rev");
    request["expected_merge"] = request["merge"].clone();
    let claim = request["claims"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    claim["input_id"] = json!("client-after");
    claim["slot"] = json!("client-after");

    wire["inputs"][0] = json_fixture("client-only.json")["inputs"][1].clone();
    for index in [1, 2] {
        let input = &mut wire["inputs"][index];
        input["role"] = json!("consumer");
        input["operation_authority"] = json!("calls");
        input["repo"] = json!("pilot-consumer");
    }
    wire["inputs"][1]["input_id"] = json!("client-after");
    wire["inputs"][2]["origin"]["repo"] = json!("pilot-consumer");
    wire["change"]["subject_role"] = json!("client");
    wire["change"]["producer"] = Value::Null;
    wire["change"]["consumer"] = json!({
        "before_input_id":"client-before","after_input_id":"client-after"
    });
    (request, wire)
}

#[test]
fn sideless_computed_client_checks_merge_head() {
    let (mut request, _) = consumer_computed();
    assert_eq!(
        admission(request.clone()).claims["effective"].status,
        "admitted"
    );
    request["merge"]["head"] = json!("different-topic-rev");
    request["expected_merge"] = request["merge"].clone();
    let report = admission(request);
    let decision = refused(&report, "effective", "invalid_merge_proof");
    assert_eq!(decision.reasons[0].dimension.as_deref(), Some("head"));
}

#[test]
fn frozen_computed_client_checks_merge_head() {
    let (_, mut wire) = consumer_computed();
    assert!(impact(wire.clone()).validate().counts.is_empty());
    wire["change"]["selection"]["proposed_head"] = json!("different-topic-rev");
    let report = impact(wire).validate();
    let decision = refused(&report, "report-cutoff", "invalid_merge_proof");
    assert_eq!(decision.reasons[0].dimension.as_deref(), Some("head"));
}

#[test]
fn explicit_service_after_checks_merge_head() {
    let (mut request, _) = historical_computed();
    assert_eq!(
        admission(request.clone()).claims["effective"].status,
        "admitted"
    );
    request["merge"]["head"] = json!("different-topic-rev");
    request["expected_merge"] = request["merge"].clone();
    let report = admission(request);
    let decision = refused(&report, "effective", "invalid_merge_proof");
    assert_eq!(decision.reasons[0].dimension.as_deref(), Some("head"));
}

#[test]
fn required_media_refusal_blocks_schema() {
    required_media_fault("status", "201");
}

#[test]
fn required_media_reasons_reach_schema() {
    required_media_fault("schema_blob", "different-blob");
}

#[test]
fn optional_media_still_requires_its_own_media_dependencies() {
    let (mut request, mut bytes) = media_request();
    request["receipts"][11]["dependencies"] = json!(["handler", "nested-proof"]);
    let mut nested = request["receipts"][11].clone();
    nested["id"] = json!("nested-proof");
    nested["origin"]["blob"] = json!("nested-blob");
    nested["dependencies"] = json!(["handler"]);
    bytes.insert("nested-proof".into(), bytes["media-proof"].clone());
    request["receipts"].as_array_mut().unwrap().push(nested);
    let media = "media:12:schema-claim:media-proof";
    let control = admission_with_bytes(request.clone(), &bytes);
    assert_eq!(control.claims["schema-claim"].status, "admitted");
    assert_eq!(control.claims[media].status, "admitted");

    let mut catalog: Value = serde_json::from_slice(&bytes["nested-proof"]).unwrap();
    catalog[0]["status"] = json!("201");
    let damaged = serde_json::to_vec(&catalog).unwrap();
    request["receipts"][13]["digest"]["value"] = json!(format!("{:x}", Sha256::digest(&damaged)));
    bytes.insert("nested-proof".into(), damaged);
    let report = admission_with_bytes(request, &bytes);
    assert_eq!(report.claims["schema-claim"].status, "admitted");
    let parent = refused(&report, media, "missing_dependency");
    assert!(parent
        .reasons
        .iter()
        .any(|reason| reason.evidence[0] == "nested-proof"));
    refused(
        &report,
        "media:12:schema-claim:nested-proof",
        "slot_mismatch",
    );
}

fn required_media_fault(key: &str, wrong: &str) {
    let (mut request, mut bytes) = media_request();
    let optional = request.clone();
    request["receipts"][0]["dependencies"] = json!(["config", "derived"]);
    request["receipts"].as_array_mut().unwrap().push(json!({
        "id":"derived","kind":"derived","origin":{"kind":"derived","version":"0.1"},
        "dependencies":["media-proof"]
    }));
    let media = "media:12:schema-claim:media-proof";
    let control = admission_with_bytes(request.clone(), &bytes);
    assert_eq!(control.claims["schema-claim"].status, "admitted");
    assert_eq!(control.claims[media].status, "admitted");

    let mut catalog: Value = serde_json::from_slice(&bytes["media-proof"]).unwrap();
    catalog[0][key] = json!(wrong);
    let damaged = serde_json::to_vec(&catalog).unwrap();
    let mut optional = optional;
    for value in [&mut request, &mut optional] {
        value["receipts"][11]["digest"]["value"] = json!(format!("{:x}", Sha256::digest(&damaged)));
    }
    bytes.insert("media-proof".into(), damaged);

    let attached = admission_with_bytes(optional, &bytes);
    assert_eq!(attached.claims["schema-claim"].status, "admitted");
    refused(&attached, media, "slot_mismatch");
    let request = serde_json::from_value(request).unwrap();
    let required = admit_schema_input(&request, &bytes, "schema-claim");
    assert_eq!(required.report.claims["sibling"].status, "admitted");
    let proof = refused(&required.report, media, "slot_mismatch");
    let schema = refused(&required.report, "schema-claim", "missing_dependency");
    assert!(schema.reasons.contains(&proof.reasons[0]), "{schema:?}");
    assert!(schema.reasons.iter().any(|reason| {
        reason.code == "missing_dependency" && reason.evidence[0] == "media-proof"
    }));
    assert!(required.input().is_none());
}

#[test]
fn frozen_computed_topic_head_must_be_separate() {
    let (_, mut wire) = historical_computed();
    assert_eq!(
        impact(wire.clone()).validate().claims["report-cutoff"].status,
        "admitted"
    );
    wire["change"]["producer"]["before_input_id"] = json!("topic-head");
    refused(&impact(wire).validate(), "report-cutoff", "missing_cutoff");
}

#[test]
fn frozen_computed_topic_head_cannot_fill_held_slots() {
    let (_, mut wire) = consumer_computed();
    wire["inputs"][2]["branch"] = json!("topic");
    wire["change"]["selection"]["receipts"] = json!({"topic-head":{
        "kind":"historical_pin","branch":"topic","rev":"topic-head-rev",
        "available_at":wire["cutoff_at"],"evidence":"archive"
    }});
    assert!(impact(wire.clone()).validate().counts.is_empty());

    wire["change"]["producer"] =
        json!({"before_input_id":"topic-head","after_input_id":"topic-head"});
    refused(&impact(wire).validate(), "report-cutoff", "missing_cutoff");
}

#[test]
fn frozen_singular_receipt_requires_input_branch() {
    let mut wire = json_fixture("client-only.json");
    wire["change"]["selection"]["branch"] = json!("main");
    assert!(impact(wire.clone()).validate().counts.is_empty());
    wire["inputs"][0].as_object_mut().unwrap().remove("branch");
    let report = impact(wire).validate();
    let decision = refused(&report, "report-cutoff", "missing_cutoff");
    assert_eq!(decision.reasons[0].dimension.as_deref(), Some("branch"));
}
