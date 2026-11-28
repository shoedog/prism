// Admission binds named-tree bytes and typed receipts to the selected slots.
// Admission preserves independently supported claims and counts scoped refusals.
// Bindings preserve snapshot identity, raw observations, and extraction coverage.
// Impact retains both revision-local sides and each comparison's own occurrences.
// Impact stores each standing limit once and links supported affected claims.
// Saved status compares the same admitted element on the selected service side.
// Membership negatives require complete evidence across all applicable inputs.
// Both 0.1 schemas validate their own DTOs and enforce closed records.
//
// P4 or deferred companions from the binding design answers:
// A24 A26 A30 A37 A39 A40 A42 A43 A45 A46 A47
// B14 B16 B18 B25 B26 B34 B40 B43 B44 B45
// C19 C20 C23
// D15 (full ordering and replay perturbation)

#[path = "support/doc_admission.rs"]
mod doc_admission;
#[path = "support/doc_cases.rs"]
mod doc_cases;
#[path = "support/doc_saved.rs"]
mod doc_saved;
#[path = "support/doc_schema.rs"]
mod doc_schema;
#[path = "support/doc_wire.rs"]
mod doc_wire;
#[allow(dead_code)]
#[path = "support/matrix.rs"]
mod matrix;

use doc_cases::run;

#[test]
fn admission_receipt_and_slot_rows() {
    run(&["admission"], doc_admission::check);
}

#[test]
fn admission_document_decode_and_reference_rows() {
    run(&["decode", "semantic"], doc_wire::check);
}

#[test]
fn documents_bindings_and_impact_rows() {
    run(&["bindings", "impact", "round_trip"], doc_wire::check);
}

#[test]
fn documents_saved_scan_rows() {
    run(&["saved"], doc_saved::check);
}

#[test]
fn documents_membership_rows() {
    run(&["membership"], doc_saved::check);
}

#[test]
fn documents_schema_rows() {
    run(&["schema", "hashes"], doc_schema::check);
}

#[test]
fn cli_help_and_version() {
    use std::process::Command;
    let binary = env!("CARGO_BIN_EXE_prism-boundary");
    let help = Command::new(binary).arg("--help").output().unwrap();
    assert!(help.status.success(), "{:?}", help.stderr);
    let stdout = String::from_utf8(help.stdout).unwrap();
    assert!(stdout.contains("Usage: prism-boundary"));
    assert!(stdout.contains("--help") && stdout.contains("--version"));

    let version = Command::new(binary).arg("--version").output().unwrap();
    assert!(version.status.success(), "{:?}", version.stderr);
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim_end(),
        concat!("prism-boundary ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn schema_projection_and_report_share_receipt_refusals() {
    use prism_boundary::revision::AdmissionRequest;
    use prism_boundary::{admit_inputs, admit_schema_input};
    use serde_json::json;
    use sha2::{Digest, Sha256};

    for (key, replacement, file, code) in [
        (
            "/receipts/0/origin/kind",
            json!("new_origin"),
            "inventory.json",
            "unsupported_kind",
        ),
        (
            "/receipts/0/origin/availability",
            json!("new_state"),
            "inventory.json",
            "unsupported_kind",
        ),
        (
            "/receipts/0/format",
            json!("other"),
            "inventory.json",
            "unsupported_format",
        ),
        (
            "/receipts/0/format",
            json!("json"),
            "conflicting.txt",
            "unsupported_schema_dialect",
        ),
    ] {
        let mut input = doc_cases::json_fixture("admission.json");
        *input.pointer_mut(key).unwrap() = replacement;
        let mut bytes = doc_admission::supplied_bytes(&serde_json::Value::Null);
        bytes.insert("schema".into(), doc_cases::fixture(file));
        input["receipts"][0]["digest"]["value"] =
            json!(format!("{:x}", Sha256::digest(&bytes["schema"])));
        let request: AdmissionRequest = serde_json::from_value(input).unwrap();
        let report = admit_inputs(&request, &bytes);
        let projection = admit_schema_input(&request, &bytes, "schema-claim");
        assert_eq!(report.counts.get(code), Some(&1), "{key} {file}");
        assert_eq!(report.claims["sibling"].status, "admitted");
        assert_eq!(
            serde_json::to_value(report).unwrap(),
            serde_json::to_value(&projection.report).unwrap()
        );
        assert!(projection.input().is_none());
    }
}

#[test]
fn coverage_deduplicates_with_recorded_origin_and_remaps_links() {
    use prism_boundary::model::{CoverageLimit, ImpactDocument, LimitRegion};
    use serde_json::json;

    let mut doc: ImpactDocument =
        serde_json::from_value(doc_cases::json_fixture("mixed.json")).unwrap();
    let row = &mut doc.change.elements[0];
    row.coverage_limits.push("second".into());
    let mut limit = CoverageLimit {
        limit_id: "first".into(),
        comparison_id: "actual_before".into(),
        input_id: "service-before".into(),
        side: "before".into(),
        authority: "schema".into(),
        scope: LimitRegion::Schema {
            path_scope: Some(prism_boundary::revision::Scope::Api),
        },
        pointer: vec![],
        claim: "coverage".into(),
        reason: "operation_unreadable".into(),
        source: "input_inventory".into(),
        evidence: vec![json!("schema-blob")],
        affected_claims: vec![],
        presence: None,
    };
    let original = limit.clone();
    limit.limit_id = "second".into();
    limit.source = "comparison".into();
    limit.evidence.push(json!("comparison-observation"));
    let doc = doc.with_coverage_limits(&[original, limit]);
    assert_eq!(doc.coverage.len(), 1);
    assert_eq!(doc.coverage[0].source, "both");
    assert_eq!(
        doc.coverage[0].evidence,
        [json!("schema-blob"), json!("comparison-observation")]
    );
    assert_eq!(doc.coverage[0].affected_claims, ["retained-stamp"]);
    assert_eq!(doc.change.elements[0].coverage_limits, ["first"]);
    assert!(doc.validate().counts.is_empty());
}

#[test]
fn cutoff_reference_requires_its_edge_and_availability() {
    use prism_boundary::revision::AdmissionRequest;
    use prism_boundary::{admit_schema_input, index_schema};
    use serde_json::json;
    use sha2::{Digest, Sha256};

    let mut input = doc_cases::json_fixture("admission.json");
    let mut bytes = doc_admission::supplied_bytes(&serde_json::Value::Null);
    for (receipt, file) in [("schema", "external.json"), ("parts", "parts.json")] {
        bytes.insert(receipt.into(), doc_cases::fixture(file));
    }
    input["receipts"][0]["digest"]["value"] =
        json!(format!("{:x}", Sha256::digest(&bytes["schema"])));
    input["receipts"][0]["retrieval_uri"] = json!("https://pilot.invalid/schemas/openapi.json");
    input["receipts"].as_array_mut().unwrap().push(json!({
        "id":"parts","kind":"blob","format":"json",
        "retrieval_uri":"https://pilot.invalid/schemas/parts.json",
        "origin":{
            "kind":"independent_cutoff","evidence":"archived",
            "available_at":"2026-11-26T11:00:00Z","repo":"pilot-dependency",
            "covered_tree":"parts-tree","path":"parts.json","blob":"parts-blob"
        },
        "digest":{"algorithm":"sha256","framing":"raw_bytes","value":format!("{:x}", Sha256::digest(&bytes["parts"]))}
    }));

    let request: AdmissionRequest = serde_json::from_value(input.clone()).unwrap();
    let absent = admit_schema_input(&request, &bytes, "schema-claim");
    assert!(index_schema(&absent.input().unwrap())
        .unwrap()
        .fields
        .is_empty());

    input["receipts"][0]["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!("parts"));
    let request: AdmissionRequest = serde_json::from_value(input.clone()).unwrap();
    let admitted = admit_schema_input(&request, &bytes, "schema-claim");
    let inventory = index_schema(&admitted.input().unwrap()).unwrap();
    assert_eq!(
        inventory.fields[0].id.to_string(),
        "field:resp:GET /kiln:200#/stamp"
    );
    assert_eq!(inventory.fields[0].sites[0].tree, "parts-tree");

    input["receipts"][11]["origin"]["available_at"] = json!("2026-11-27T11:00:00Z");
    let request: AdmissionRequest = serde_json::from_value(input).unwrap();
    let refused = admit_schema_input(&request, &bytes, "schema-claim");
    assert!(refused.input().is_none());
    assert_eq!(refused.report.counts.get("missing_cutoff"), Some(&1));
    assert_eq!(refused.report.claims["sibling"].status, "admitted");
}

#[test]
fn historical_selection_requires_evidence_covering_the_cutoff() {
    use prism_boundary::admit_inputs;
    use prism_boundary::revision::AdmissionRequest;
    use serde_json::json;

    let mut input = doc_cases::json_fixture("admission.json");
    input["mode"] = json!("historical_as_of");
    input["cutoff_at"] = json!("2026-11-25T12:00:00Z");
    input["inputs"][1]["selection"] = json!({"kind": "historical_pin", "branch": "main", "rev": "client-before-rev", "available_at": "2026-11-25T12:00:00Z", "evidence": "fixture-ref-archive"});
    let bytes = doc_admission::supplied_bytes(&serde_json::Value::Null);
    for (receipt, expected) in [
        (
            json!({"kind":"historical_snapshot","branch":"main","rev":"service-before-rev",
                "captured_at":"2026-11-25T05:00:00-07:00","evidence":"archived"}),
            "admitted",
        ),
        (
            json!({"kind":"historical_ref_history","branch":"main","rev":"service-before-rev",
                "valid_from":"2026-11-25T11:00:00Z","valid_until":"2026-11-25T13:00:00Z",
                "evidence":"ref-log"}),
            "admitted",
        ),
        (
            json!({"kind":"historical_ref_history","branch":"main","rev":"service-before-rev",
                "valid_from":"2026-11-25T11:00:00Z","valid_until":"2026-11-25T12:00:00Z",
                "evidence":"ref-log"}),
            "unchecked",
        ),
    ] {
        input["inputs"][0]["selection"] = receipt;
        let request: AdmissionRequest = serde_json::from_value(input.clone()).unwrap();
        let report = admit_inputs(&request, &bytes);
        assert_eq!(report.claims["schema-claim"].status, expected);
        assert_eq!(report.claims["sibling"].status, "admitted");
        assert_eq!(
            report.counts.get("missing_cutoff"),
            (expected == "unchecked").then_some(&1)
        );
    }
}

#[path = "support/fold_admission.rs"]
mod fold_admission;
#[path = "support/fold_impact.rs"]
mod fold_impact;
#[path = "support/fold_saved.rs"]
mod fold_saved;

#[path = "support/fold2.rs"]
mod fold2;

#[path = "support/known_limits.rs"]
mod known_limits;

#[path = "support/fold3.rs"]
mod fold3;

#[path = "support/fold4.rs"]
mod fold4;
#[path = "support/fold5.rs"]
mod fold5;
