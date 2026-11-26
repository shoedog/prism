use crate::doc_admission::supplied_bytes;
use crate::doc_cases::{fixture, json_fixture};
use prism_boundary::revision::AdmissionRequest;
use prism_boundary::{admit_inputs, admit_schema_input, index_schema};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn base() -> (Value, BTreeMap<String, Vec<u8>>) {
    (json_fixture("admission.json"), supplied_bytes(&Value::Null))
}

fn check(input: Value, bytes: &BTreeMap<String, Vec<u8>>, claim: &str, code: Option<&str>) {
    let input: AdmissionRequest = serde_json::from_value(input).expect("decode admission records");
    let report = admit_inputs(&input, bytes);
    assert_eq!(report.claims["sibling"].status, "admitted");
    assert_eq!(
        report.claims[claim]
            .reasons
            .first()
            .map(|r| r.code.as_str()),
        code
    );
    if let Some(code) = code {
        assert_eq!(report.counts.get(code), Some(&1));
    } else {
        assert!(report.counts.is_empty());
    }
}

#[test]
fn fold_selection_comes_from_counterpart_slot() {
    let (mut input, bytes) = base();
    check(input.clone(), &bytes, "schema-claim", None);
    input["slots"][0]["selection_basis"] = json!("counterpart");
    input["inputs"][0]["binding"]["selection_basis"] = json!("counterpart");
    let mut proved = input.clone();
    proved["inputs"][0]["selection"] = json!({"kind":"live_ref","branch":"main",
        "rev":"service-before-rev","observed_at":"2026-11-26T12:00:00Z"});
    check(proved, &bytes, "schema-claim", None);
    check(input, &bytes, "schema-claim", Some("missing_cutoff"));
}

#[test]
fn fold_historical_clock_comes_from_subject_snapshot() {
    let (mut input, bytes) = base();
    input["mode"] = json!("historical_as_of");
    input["subject_head_input_id"] = json!("service-after");
    input["cutoff_at"] = json!("2026-11-25T12:00:00Z");
    input["claims"][0]["requires_selection"] = json!(true);
    input["inputs"][0]["selection"] = json!({"kind":"historical_pin","branch":"main",
        "rev":"service-before-rev","available_at":"2026-11-25T12:00:00Z","evidence":"archive"});
    check(input.clone(), &bytes, "schema-claim", None);
    input["inputs"][2]["snapshot"]["committed_at"] = json!("2026-11-25T13:00:00Z");
    check(input, &bytes, "schema-claim", Some("missing_cutoff"));
}

#[test]
fn fold_derived_version_dispatch() {
    let (mut input, bytes) = base();
    input["receipts"].as_array_mut().unwrap().push(json!({
        "id":"derived","kind":"derived","origin":{"kind":"derived","version":"0.1"},
        "dependencies":["settings"]
    }));
    input["receipts"][0]["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!("derived"));
    check(input.clone(), &bytes, "schema-claim", None);
    input["receipts"][11]["origin"]["version"] = json!("future");
    check(input, &bytes, "schema-claim", Some("unsupported_kind"));
}

#[test]
fn fold_unknown_role_spares_sibling() {
    let (mut input, bytes) = base();
    check(input.clone(), &bytes, "schema-claim", None);
    input["inputs"][0]["binding"]["role"] = json!("future");
    check(input, &bytes, "schema-claim", Some("unsupported_kind"));
}

#[test]
fn fold_computed_metadata_spares_sibling() {
    let (mut input, bytes) = base();
    input["slots"][0]["rev"] = Value::Null;
    input["inputs"][0]["binding"]["rev"] = Value::Null;
    input["inputs"][0]["snapshot"] = json!({"snapshot_kind":"computed_tree",
        "tree":"service-before-tree","computed_at":"2026-11-26T11:00:00Z","merge_receipt":"merge"});
    let merge = json!({"id":"merge","base":"base","head":"head","target":"target",
        "procedure":"ort","result_tree":"service-before-tree","computed_at":"2026-11-26T11:00:00Z"});
    input["expected_merge"] = merge.clone();
    input["merge"] = merge;
    check(input.clone(), &bytes, "schema-claim", None);
    input["inputs"][0]["snapshot"]["committed_at"] = json!("2026-11-26T11:00:00Z");
    check(input, &bytes, "schema-claim", Some("slot_mismatch"));
}

#[test]
fn fold_admission_rejects_unknown_keys() {
    let (input, _) = base();
    assert!(serde_json::from_value::<AdmissionRequest>(input.clone()).is_ok());
    for path in ["/receipts/0", "/claims/0", "/slots/0", "/inputs/0/snapshot"] {
        let mut damaged = input.clone();
        damaged.pointer_mut(path).unwrap()["misspelled"] = json!(true);
        assert!(
            serde_json::from_value::<AdmissionRequest>(damaged).is_err(),
            "{path}"
        );
    }
}

#[test]
fn fold_schema_purpose_requires_one_resolved_entry() {
    let (mut input, bytes) = base();
    input["claims"][0]["purpose"] = json!("schema");
    check(input.clone(), &bytes, "schema-claim", None);
    input["claims"][0]["receipt_ids"] = json!(["config"]);
    let request: AdmissionRequest = serde_json::from_value(input).unwrap();
    let report = admit_inputs(&request, &bytes);
    let projected = admit_schema_input(&request, &bytes, "schema-claim");
    assert_eq!(report.claims["sibling"].status, "admitted");
    assert!(projected.input().is_none());
    assert_eq!(report.counts, projected.report.counts);
    assert_eq!(report.counts.get("missing_receipt"), Some(&1));
}

#[test]
fn fold_config_entry_identity() {
    let (mut input, bytes) = base();
    input["receipts"][1]["entry"] = json!(1);
    check(input.clone(), &bytes, "schema-claim", None);
    input["receipts"][1]["entry"] = json!(2);
    check(input, &bytes, "schema-claim", Some("slot_mismatch"));
}

#[test]
fn fold_config_requires_schema_dependency_edge() {
    let (mut input, bytes) = base();
    check(input.clone(), &bytes, "schema-claim", None);
    input["receipts"][0]["dependencies"] = json!([]);
    check(input, &bytes, "schema-claim", Some("missing_dependency"));
}

#[test]
fn fold_conflicting_logical_config_key_spares_sibling() {
    let (mut input, bytes) = base();
    let mut alias = input["receipts"][1].clone();
    alias["id"] = json!("alias");
    input["receipts"].as_array_mut().unwrap().push(alias);
    input["receipts"][0]["dependencies"] = json!(["config", "alias"]);
    check(input.clone(), &bytes, "schema-claim", None);
    input["receipts"][11]["dependencies"] = json!(["mapping", "settings"]);
    check(input, &bytes, "schema-claim", Some("receipt_conflict"));
}

#[test]
fn fold_config_cutoff_origin_is_accounted() {
    let (mut input, bytes) = base();
    check(input.clone(), &bytes, "schema-claim", None);
    input["receipts"][1]["origin"] = json!({"kind":"independent_cutoff","evidence":"archive",
        "available_at":"2026-11-25T12:00:00Z"});
    check(input, &bytes, "schema-claim", Some("unsupported_kind"));
}

#[test]
fn fold_missing_reference_uri_is_accounted() {
    let (mut input, mut bytes) = base();
    bytes.insert("parts".into(), fixture("parts.json"));
    let mut reference = input["receipts"][0].clone();
    reference["id"] = json!("parts");
    reference["origin"]["path"] = json!("parts.json");
    reference["origin"]["blob"] = json!("parts-blob");
    reference["digest"]["value"] = json!(format!("{:x}", Sha256::digest(&bytes["parts"])));
    reference["dependencies"] = json!([]);
    reference["retrieval_uri"] = json!("https://pilot.invalid/parts.json");
    input["receipts"].as_array_mut().unwrap().push(reference);
    input["receipts"][0]["dependencies"] = json!(["config", "parts"]);
    check(input.clone(), &bytes, "schema-claim", None);
    input["receipts"][11]["retrieval_uri"] = Value::Null;
    check(input, &bytes, "schema-claim", Some("missing_receipt"));
}

#[test]
fn fold_media_dependency_refusal_stays_at_root() {
    let (mut input, mut bytes) = base();
    let dialect = br#"{"swagger":"2.0","paths":{"/kiln":{"get":{"responses":{"200":{"schema":{"type":"object","properties":{"x":{"type":"string"}}}}}}},"/neighbor":{"get":{"produces":["application/json"],"responses":{"200":{"schema":{"type":"object","properties":{"y":{"type":"string"}}}}}}}}}"#;
    bytes.insert("schema".into(), dialect.to_vec());
    input["receipts"][0]["digest"]["value"] = json!(format!("{:x}", Sha256::digest(dialect)));
    let catalog = fixture("media.json");
    let mut media = input["receipts"][0].clone();
    media["id"] = json!("media-proof");
    media["kind"] = json!("media");
    media["origin"]["path"] = json!("catalog.json");
    media["dependencies"] = json!(["handler"]);
    media["digest"]["value"] = json!(format!("{:x}", Sha256::digest(&catalog)));
    bytes.insert("media-proof".into(), catalog);
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
    let report = |value| {
        let request: AdmissionRequest = serde_json::from_value(value).unwrap();
        let admitted = admit_schema_input(&request, &bytes, "schema-claim");
        assert_eq!(admitted.report.claims["sibling"].status, "admitted");
        let reader = admitted
            .input()
            .expect("entry survives a refused media proof");
        let index = index_schema(&reader).unwrap();
        assert!(index
            .fields
            .iter()
            .any(|f| f.id.to_string().contains("/neighbor")));
        let projected = reader.media_receipts.len();
        (admitted.report, projected)
    };
    let (control, projected) = report(input.clone());
    assert!(control.counts.is_empty());
    assert_eq!(projected, 1);
    for (key, value, reason) in [
        ("id", "missing-handler", "missing_dependency"),
        ("path", "wrong.go", "missing_dependency"),
    ] {
        let mut damaged = input.clone();
        if key == "id" {
            damaged["receipts"][12]["id"] = json!(value);
        } else {
            damaged["receipts"][12]["origin"][key] = json!(value);
        }
        let (report, projected) = report(damaged);
        assert_eq!(projected, 0);
        assert_eq!(report.counts.get(reason), Some(&1));
        let decision = report
            .claims
            .values()
            .find(|claim| claim.status == "unchecked")
            .unwrap();
        let root = decision.media_root.as_ref().unwrap();
        assert_eq!(root.operation_id, "op:GET /kiln");
        assert_eq!(root.status.as_deref(), Some("200"));
    }
}

#[test]
fn fold_multiple_slot_mismatches_keep_all_dimensions() {
    let (mut input, bytes) = base();
    check(input.clone(), &bytes, "schema-claim", None);
    input["inputs"][0]["binding"]["repo"] = json!("other-repo");
    input["inputs"][0]["binding"]["tree"] = json!("other-tree");
    let request: AdmissionRequest = serde_json::from_value(input).unwrap();
    let report = admit_inputs(&request, &bytes);
    assert_eq!(report.claims["sibling"].status, "admitted");
    assert_eq!(report.counts.get("slot_mismatch"), Some(&1));
    let wire = serde_json::to_value(&report).unwrap();
    let reason = &wire["claims"]["schema-claim"]["reasons"][0];
    assert_eq!(reason["dimension"], "repo");
    let details = reason["additional_mismatches"]
        .as_array()
        .expect("second failed dimension");
    assert!(details.iter().any(|detail| detail["dimension"] == "tree"));
}

#[test]
fn fold_config_receipt_must_record_numbered_entry() {
    let (mut input, bytes) = base();
    input["receipts"][1]["entry"] = json!(1);
    check(input.clone(), &bytes, "schema-claim", None);
    input["receipts"][1]
        .as_object_mut()
        .unwrap()
        .remove("entry");
    check(input, &bytes, "schema-claim", Some("missing_receipt"));
}
