use crate::doc_admission::supplied_bytes;
use crate::doc_cases::{fixture, json_fixture, schema};
use prism_boundary::revision::AdmissionRequest;
use prism_boundary::{
    admit_inputs, admit_schema_input, diff_schemas, index_schema, ImpactDocument,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub(super) fn admission(input: Value) -> prism_boundary::revision::AdmissionReport {
    let request = serde_json::from_value(input).unwrap();
    admit_inputs(&request, &supplied_bytes(&Value::Null))
}

pub(super) fn live_pin(rev: &str) -> Value {
    json!({"kind":"live_ref","rev":rev,"branch":"main","observed_at":"2026-11-26T12:00:00Z"})
}

fn version_pin(rev: &str) -> Value {
    json!({"kind":"explicit_pin","rev":rev,"selection_time":"2026-11-26T12:00:00Z",
        "selected_by":"reviewer","selection_reason":"compare releases"})
}

pub(super) fn impact(input: Value) -> ImpactDocument {
    serde_json::from_value(input).unwrap()
}

#[test]
fn fold2_counterpart_admission_uses_recorded_position() {
    let mut input = json_fixture("admission.json");
    input["claims"].as_array_mut().unwrap().push(json!({
        "id":"held","input_id":"service-held","slot":"service-held","purpose":"metadata","receipt_ids":[]
    }));
    for (mode, date) in [
        ("live", "2026-11-26T12:00:00Z"),
        ("historical_as_of", "2026-11-25T12:00:00Z"),
    ] {
        let mut control = input.clone();
        control["mode"] = json!(mode);
        control["cutoff_at"] = json!(date);
        for (i, rev) in [(1, "client-before-rev"), (3, "service-held-rev")] {
            control["inputs"][i]["selection"] = if mode == "live" {
                live_pin(rev)
            } else {
                json!({"kind":"historical_pin","rev":rev,"branch":"main","available_at":date,"evidence":"archive"})
            };
        }
        assert!(admission(control.clone()).counts.is_empty(), "{mode}");
        for (index, claim, sibling) in [(1, "sibling", "held"), (3, "held", "sibling")] {
            let mut damaged = control.clone();
            damaged["inputs"][index]["selection"] = Value::Null;
            let report = admission(damaged);
            assert_eq!(report.claims["schema-claim"].status, "admitted");
            assert_eq!(report.claims[sibling].status, "admitted");
            assert_eq!(report.claims[claim].status, "unchecked", "{mode} {claim}");
            assert_eq!(report.counts.get("missing_cutoff"), Some(&1));
        }
    }
}

#[test]
fn fold2_version_admission_needs_proof_and_clock() {
    let mut input = json_fixture("admission.json");
    input["mode"] = json!("version_comparison");
    input["selection_time"] = json!("2026-11-26T12:00:00Z");
    input["inputs"][0]["selection"] = version_pin("service-before-rev");
    input["inputs"][1]["selection"] = version_pin("client-before-rev");
    assert!(admission(input.clone()).counts.is_empty());
    let mut missing = input.clone();
    missing["inputs"][0]["selection"] = Value::Null;
    let report = admission(missing);
    assert_eq!(report.claims["sibling"].status, "admitted");
    assert_eq!(report.claims["schema-claim"].status, "unchecked");
    assert_eq!(report.counts.get("missing_cutoff"), Some(&1));
    input["selection_time"] = Value::Null;
    assert_eq!(admission(input).counts.get("missing_cutoff"), Some(&2));
}

#[test]
fn fold2_removed_caller_selection_fields_are_rejected() {
    let mut base = json_fixture("admission.json");
    for slot in base["slots"].as_array_mut().unwrap() {
        slot.as_object_mut().unwrap().remove("selection_basis");
    }
    for input in base["inputs"].as_array_mut().unwrap() {
        input["binding"]
            .as_object_mut()
            .unwrap()
            .remove("selection_basis");
    }
    assert!(serde_json::from_value::<AdmissionRequest>(base.clone()).is_ok());
    for (path, field, value) in [
        ("/slots/1", "selection_basis", json!("snapshot")),
        ("/inputs/1/binding", "selection_basis", json!("snapshot")),
        ("/claims/0", "requires_selection", json!(true)),
    ] {
        let mut damaged = base.clone();
        damaged.pointer_mut(path).unwrap()[field] = value;
        let error = serde_json::from_value::<AdmissionRequest>(damaged)
            .err()
            .expect(field);
        assert!(error.to_string().contains(field), "{error}");
    }
}

#[test]
fn fold2_same_repo_held_document_needs_selection() {
    let mut input = json_fixture("client-only.json");
    input["inputs"][0]["repo"] = json!("pilot-consumer");
    input["inputs"][0]["origin"]["repo"] = json!("pilot-consumer");
    input["inputs"][0]["schema"]["source"]["repo"] = json!("pilot-consumer");
    assert!(impact(input.clone()).validate().counts.is_empty());
    input["change"]["selection"] = Value::Null;
    let report = impact(input).validate();
    assert_eq!(report.claims["input:client-after"].status, "admitted");
    assert_eq!(
        report.claims["report-cutoff"]
            .reasons
            .first()
            .and_then(|r| r.record_id.as_deref()),
        Some("service-held")
    );
    assert!(report.counts.contains_key("missing_cutoff"));
}

#[test]
fn fold2_version_document_checks_each_selected_side() {
    let mut input = json_fixture("operations-only.json");
    input["mode"] = json!("version_comparison");
    input["change"]["selection"] = json!({
        "selection_time":"2026-11-26T12:00:00Z",
        "receipts":{
            "service-before":version_pin("service-before-rev"),
            "service-after":version_pin("service-after-rev")
        }
    });
    assert!(impact(input.clone()).validate().counts.is_empty());
    let bytes = serde_json::to_vec(&input).unwrap();
    let decoded = ImpactDocument::from_json(&bytes);
    for (missing, sibling) in [
        ("service-before", "service-after"),
        ("service-after", "service-before"),
    ] {
        let mut damaged = input.clone();
        damaged["change"]["selection"]["receipts"]
            .as_object_mut()
            .unwrap()
            .remove(missing);
        let report = impact(damaged).validate();
        assert_eq!(
            report.claims[&format!("input:{sibling}")].status,
            "admitted"
        );
        assert_eq!(
            report.claims["report-cutoff"]
                .reasons
                .first()
                .and_then(|r| r.record_id.as_deref()),
            Some(missing)
        );
        assert_eq!(report.counts.get("missing_cutoff"), Some(&1));
    }
    assert!(decoded.is_ok(), "{decoded:?}");
}

pub(super) fn media_request() -> (Value, std::collections::BTreeMap<String, Vec<u8>>) {
    let mut input = json_fixture("admission.json");
    input["inputs"][1]["selection"] = live_pin("client-before-rev");
    let mut bytes = supplied_bytes(&Value::Null);
    let schema = br#"{"swagger":"2.0","paths":{"/kiln":{"get":{"responses":{"200":{"schema":{"type":"object","properties":{"x":{"type":"string"}}}}}}},"/neighbor":{"get":{"produces":["application/json"],"responses":{"200":{"schema":{"type":"object","properties":{"y":{"type":"string"}}}}}}}}}"#;
    bytes.insert("schema".into(), schema.to_vec());
    input["receipts"][0]["digest"]["value"] = json!(format!("{:x}", Sha256::digest(schema)));
    let catalog = fixture("media.json");
    bytes.insert("media-proof".into(), catalog.clone());
    let mut media = input["receipts"][0].clone();
    media["id"] = json!("media-proof");
    media["kind"] = json!("media");
    media["origin"]["path"] = json!("catalog.json");
    media["dependencies"] = json!(["handler"]);
    media["digest"]["value"] = json!(format!("{:x}", Sha256::digest(catalog)));
    let mut handler = input["receipts"][7].clone();
    handler["id"] = json!("handler");
    handler["origin"] = input["receipts"][0]["origin"].clone();
    handler["origin"]["path"] = json!("handler.go");
    handler["origin"]["blob"] = json!("handler-blob");
    input["receipts"]
        .as_array_mut()
        .unwrap()
        .extend([media, handler]);
    input["receipts"][0]["dependencies"] = json!(["config", "media-proof"]);
    (input, bytes)
}

#[test]
fn fold2_media_source_blob_is_bound_by_reader_and_admission() {
    let (mut input, mut bytes) = media_request();
    let request = serde_json::from_value(input.clone()).unwrap();
    let control = admit_schema_input(&request, &bytes, "schema-claim");
    assert!(control.report.counts.is_empty());
    assert_eq!(
        index_schema(&control.input().unwrap())
            .unwrap()
            .fields
            .len(),
        2
    );

    let mut catalog: Value = serde_json::from_slice(&bytes["media-proof"]).unwrap();
    catalog[0]["schema_blob"] = json!("different-blob");
    let changed = serde_json::to_vec(&catalog).unwrap();
    input["receipts"][11]["digest"]["value"] = json!(format!("{:x}", Sha256::digest(&changed)));
    bytes.insert("media-proof".into(), changed);
    let request = serde_json::from_value(input).unwrap();
    let result = admit_schema_input(&request, &bytes, "schema-claim");
    assert_eq!(result.report.claims["sibling"].status, "admitted");
    let index = index_schema(&result.input().unwrap()).unwrap();
    assert_eq!(
        index.fields[0].id.to_string(),
        "field:resp:GET /neighbor:200#/y"
    );
    assert_eq!(index.fields.len(), 1);
    assert_eq!(result.report.counts.get("slot_mismatch"), Some(&1));
}

#[test]
fn fold2_media_cycle_crossing_required_dependency_refuses_entry() {
    let (mut input, bytes) = media_request();
    input["receipts"][0]["dependencies"] = json!(["config", "derived"]);
    input["receipts"].as_array_mut().unwrap().push(json!({
        "id":"derived","kind":"derived","origin":{"kind":"derived","version":"0.1"},"dependencies":["media-proof"]
    }));
    assert!(admission_with_bytes(input.clone(), &bytes)
        .counts
        .is_empty());
    input["receipts"][11]["dependencies"] = json!(["handler", "derived"]);
    let report = admission_with_bytes(input, &bytes);
    assert_eq!(report.claims["sibling"].status, "admitted");
    assert_eq!(report.claims["schema-claim"].status, "unchecked");
    assert_eq!(
        report.claims["schema-claim"].reasons[0].code,
        "dependency_cycle"
    );
    assert_eq!(report.counts.get("dependency_cycle"), Some(&1));
}

pub(super) fn admission_with_bytes(
    input: Value,
    bytes: &std::collections::BTreeMap<String, Vec<u8>>,
) -> prism_boundary::revision::AdmissionReport {
    admit_inputs(&serde_json::from_value(input).unwrap(), bytes)
}

#[test]
fn fold2_config_receipt_requires_an_owning_config_binding() {
    let mut input = json_fixture("admission.json");
    input["inputs"][1]["selection"] = live_pin("client-before-rev");
    input["claims"][0]["purpose"] = json!("metadata");
    input["claims"][0]["receipt_ids"] = json!(["config"]);
    assert!(admission(input.clone()).counts.is_empty());
    input["inputs"][0]["binding"]["config"] = Value::Null;
    input["slots"][0]["config"] = Value::Null;
    let report = admission(input);
    assert_eq!(report.claims["sibling"].status, "admitted");
    assert_eq!(report.claims["schema-claim"].status, "unchecked");
    assert_eq!(report.counts.get("slot_mismatch"), Some(&1));
}

#[test]
fn fold2_metric_counts_cannot_exceed_known_total() {
    let mut input = json_fixture("mixed.json");
    input["summary"]["operation_count"]["observed"] = json!(0);
    assert!(impact(input.clone()).validate().counts.is_empty());
    input["summary"]["operation_count"]["observed"] = json!(2);
    let report = impact(input).validate();
    assert_eq!(report.claims["actual_after"].status, "admitted");
    assert_eq!(report.counts.get("invalid_metric"), Some(&1));
}

#[test]
fn fold2_portable_count_boundary() {
    let control = json_fixture("operations-only.json");
    let mut input = control.clone();
    let own = crate::doc_schema::load("impact").unwrap();
    let opposite = crate::doc_schema::load("bindings").unwrap();
    for (value, accepted) in [
        (9_007_199_254_740_991u64, true),
        (9_007_199_254_740_992, false),
    ] {
        input["summary"]["unknown_target_count"]["observed"] = json!(value);
        let bytes = serde_json::to_vec(&input).unwrap();
        let schema_result =
            crate::doc_schema::validate(&own, &opposite, &control, &input, false, accepted);
        assert_eq!(
            (
                schema_result.is_ok(),
                ImpactDocument::from_json(&bytes).is_ok()
            ),
            (true, accepted),
            "{value}: {schema_result:?}"
        );
    }
}

#[test]
fn fold2_operation_addition_without_fields_does_not_inherit_media_limit() {
    let before = schema(&fixture("empty.json"), "service-before");
    for (produces, properties, reason, linked) in [
        (json!(["text/plain"]), json!({}), "non_json_media", false),
        (
            Value::Null,
            json!({"x":{"type":"string"}}),
            "media_unverified",
            true,
        ),
    ] {
        let mut after = json!({"swagger":"2.0","paths":{"/kiln":{"get":{
            "responses":{"200":{"schema":{"type":"object","properties":properties}}}
        }}}});
        if !produces.is_null() {
            after["paths"]["/kiln"]["get"]["produces"] = produces;
        }
        let after = schema(&serde_json::to_vec(&after).unwrap(), "service-after");
        let diff = diff_schemas(&before, &after).unwrap();
        let doc = impact(json_fixture("operations-only.json")).with_schema_diff(&diff);
        let row = &doc.change.elements[0];
        let limit = doc
            .coverage
            .iter()
            .find(|limit| limit.reason == reason)
            .expect("standing media limit");
        assert_eq!(row.after_fields.is_empty(), !linked);
        assert_eq!(row.coverage_limits.contains(&limit.limit_id), linked);
        assert_eq!(limit.affected_claims.contains(&row.row_id), linked);
    }
}

#[test]
fn fold2_direct_dto_still_needs_wire_closure() {
    let mut doc = impact(json_fixture("operations-only.json"));
    assert!(doc.validate().counts.is_empty());
    doc.change.subject_role = "unsupported".into();
    let report = doc.validate();
    assert_eq!(report.claims["input:service-before"].status, "admitted");
    assert_eq!(report.counts.get("invalid_document"), Some(&1));
}

#[test]
fn fold2_joint_document_uses_recorded_subject_anchor() {
    let mut wire = json_fixture("mixed.json");
    let mut client_after = live_pin("client-after-rev");
    client_after["branch"] = json!("topic");
    wire["change"]["selection"] = json!({
        "subject_head_input_id":"service-after",
        "receipts":{
            "client-before":live_pin("client-before-rev"),
            "client-after":client_after
        }
    });
    assert!(impact(wire.clone()).validate().counts.is_empty());
    wire["change"]["selection"]["receipts"]
        .as_object_mut()
        .unwrap()
        .remove("client-before");
    let report = impact(wire).validate();
    assert_eq!(report.claims["input:client-after"].status, "admitted");
    assert_eq!(report.claims["input:service-before"].status, "admitted");
    let cutoff = &report.claims["report-cutoff"];
    assert_eq!(cutoff.status, "unchecked");
    assert_eq!(
        cutoff.reasons[0].record_id.as_deref(),
        Some("client-before")
    );
}
