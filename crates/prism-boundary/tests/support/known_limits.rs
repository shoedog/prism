// These tests record observable behavior outside the P1d accept-list.
use crate::doc_admission::supplied_bytes;
use crate::doc_cases::{fixture, json_fixture, schema, API};
use prism_boundary::model::{unobserved_elements, MembershipRequest, RoleSlots};
use prism_boundary::revision::{
    saved_binding_status, AdmissionRequest, AdmittedSchemaInput, Blob, Format, ReferenceBlob,
    SavedScanEvidence, Snapshot,
};
use prism_boundary::{
    admit_inputs, admit_schema_input, index_schema, signature, BindingsDocument, ImpactDocument,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn known(id: &str) -> Value {
    json_fixture("known-limits.json")[&format!("known-limit-{id}")].clone()
}

fn admitted_index(bytes_in: &[u8]) -> prism_boundary::SchemaIndex {
    let mut input = json_fixture("admission.json");
    input["receipts"][0]["digest"]["value"] = json!(format!("{:x}", Sha256::digest(bytes_in)));
    let request = serde_json::from_value(input).unwrap();
    let mut bytes = supplied_bytes(&Value::Null);
    bytes.insert("schema".into(), bytes_in.to_vec());
    let result = admit_schema_input(&request, &bytes, "schema-claim");
    assert!(result.report.counts.is_empty());
    result.index().unwrap().unwrap()
}

fn name(value: &Value) -> &str {
    value.as_str().unwrap()
}

fn impact(value: Value) -> ImpactDocument {
    serde_json::from_value(value).unwrap()
}

#[test]
fn known_limit_01_non_json_declaration_origin() {
    let row = known("01");
    let method = name(&row["input"]["method"]);
    let path = name(&row["input"]["path"]);
    let project = |media: &str, document: bool| {
        let mut swagger = json_fixture(name(&row["fixture"]));
        let target = if document {
            &mut swagger
        } else {
            &mut swagger["paths"][path][method]
        };
        target["produces"] = json!([media]);
        let bytes = serde_json::to_vec(&swagger).unwrap();
        let index = admitted_index(&bytes);
        let mut binding = json_fixture("producer.json");
        binding["schema"]["source"]["sha256"] = json!(index.byte_sha256);
        let doc: BindingsDocument = serde_json::from_value(binding).unwrap();
        let wire = serde_json::to_value(doc.with_schema_inventory(&index).unwrap()).unwrap();
        wire["operations"][0]["message_roots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|root| root["status"] == row["input"]["status"])
            .unwrap()
            .clone()
    };
    let media = name(&row["input"]["media"]);
    let document = project(media, true);
    let operation = project(media, false);
    assert_eq!(document, operation);
    assert_eq!(
        document["declared_media"],
        row["observed"]["declared_media"]
    );
    assert_eq!(document["coverage"], row["observed"]["coverage"]);
    assert!(document["media_source"].is_null());
    assert_eq!(
        document["reader_limits"][0]["reason"],
        row["observed"]["reader_reason"]
    );
    assert_eq!(
        project("application/json", true)["media_source"],
        "document_media"
    );
    assert_eq!(
        project("application/json", false)["media_source"],
        "operation_media"
    );
}

#[test]
fn known_limit_02_seeded_link_rows_are_wire_controls() {
    let row = known("02");
    let cases = json_fixture(name(&row["fixture"]));
    for id in row["input"]["rows"].as_array().unwrap() {
        let case = &cases[name(id)]["variants"][0];
        assert!(case["diff"].is_null());
        let patch = |path| {
            case["patches"]
                .as_array()
                .unwrap()
                .iter()
                .find(|patch| patch["path"] == path)
                .unwrap()["value"]
                .clone()
        };
        assert!(!patch("/change/elements").as_array().unwrap().is_empty());
        assert!(patch("/coverage")[0]["affected_claims"].is_array());
    }
}

#[test]
fn known_limit_03_historical_effective_tree_and_separate_head() {
    let row = known("03");
    let mut input = json_fixture(name(&row["fixture"]));
    let facts = &row["input"];
    input["mode"] = facts["mode"].clone();
    input["cutoff_at"] = facts["cutoff_at"].clone();
    input["inputs"][1]["committed_at"] = facts["cutoff_at"].clone();
    input["change"]["selection"] = json!({"subject_head_input_id":"service-after"});
    assert!(impact(input.clone()).validate().counts.is_empty());
    let mut topic = input["inputs"][1].clone();
    topic
        .as_object_mut()
        .unwrap()
        .extend(facts["topic_head"].as_object().unwrap().clone());
    topic["origin"]["input_id"] = topic["input_id"].clone();
    topic["origin"]["tree"] = topic["tree"].clone();
    input["inputs"].as_array_mut().unwrap().push(topic);
    let after = input["inputs"][1].as_object_mut().unwrap();
    for key in ["rev", "committed_at", "subject"] {
        after.remove(key);
    }
    after.extend(facts["after_snapshot"].as_object().unwrap().clone());
    after.insert("origin".into(), facts["after_origin"].clone());
    input["change"]["selection"] = facts["selection"].clone();
    let report = impact(input).validate();
    assert_eq!(report.claims["input:service-after"].status, "admitted");
    assert_eq!(report.claims["input:topic-head"].status, "admitted");
    assert_eq!(report.claims["report-cutoff"].status, "unchecked");
    assert_eq!(json!(report.counts), row["observed"]["counts"]);
}

#[test]
fn known_limit_04_absent_comparison_and_usages() {
    let row = known("04");
    let mut control = impact(json_fixture(name(&row["fixture"])));
    let id = name(&row["input"]["remove_comparison"]);
    assert_eq!(control.validate().claims[id].status, "admitted");
    let mut dangling = control.clone();
    dangling.usages[0].match_service_first = None;
    assert!(dangling
        .validate()
        .counts
        .contains_key("missing_dependency"));
    control.comparisons.retain(|pair| pair.comparison_id != id);
    control.usages.clear();
    let report = control.validate();
    assert!(report.counts.is_empty());
    assert!(!report.claims.contains_key(id));
}

#[test]
fn known_limit_05_frozen_negative_does_not_recompute_matches() {
    let row = known("05");
    let mut wire = json_fixture(name(&row["fixture"]));
    let scenario = &row["input"];
    let mut scan = json_fixture(name(&scenario["scan_fixture"]));
    scan["source_hash"] = json!(schema(&fixture("inventory.json"), "service-before").content_hash);
    wire["saved_scans"] = json!([scan]);
    wire["binding_status"] = json!([scenario["status"]]);
    assert!(impact(wire.clone()).validate().counts.is_empty());
    wire["saved_scans"][0]["matches"] = json!([scenario["match"]]);
    let report = impact(wire).validate();
    assert_eq!(report.claims["binding-status:0"].status, "admitted");
    assert_eq!(json!(report.counts), row["observed"]["counts"]);
}

#[test]
fn known_limit_06_attachment_can_overwrite_digest() {
    let row = known("06");
    let seed = json_fixture(name(&row["fixture"]));
    let control = name(&row["control"]["fixture"]);
    let substituted = name(&row["input"]["index_fixture"]);
    for fixture_name in [control, substituted] {
        let bytes = fixture(fixture_name);
        let index = if fixture_name == control {
            admitted_index(&bytes)
        } else {
            schema(&bytes, "service-before")
        };
        let binding: BindingsDocument = serde_json::from_value(seed.clone()).unwrap();
        let doc = binding.with_schema_inventory(&index).unwrap();
        let projected = serde_json::to_value(doc).unwrap();
        let digest = format!("{:x}", Sha256::digest(bytes));
        assert_eq!(projected["schema"]["source"]["sha256"], digest);
        let operations = projected["operations"].as_array().unwrap();
        assert_eq!(
            operations
                .iter()
                .any(|operation| operation["operation_id"] == row["observed"]["added_operation"]),
            fixture_name == substituted,
        );
        assert_eq!(
            seed["schema"]["source"]["sha256"] == digest,
            fixture_name == control
        );
    }
}

#[test]
fn known_limit_07_unadmitted_saved_reference() {
    let row = known("07");
    let facts = &row["input"];
    let selected_bytes = fixture(name(&facts["selected_fixture"]));
    let reference_bytes = fixture(name(&facts["reference_fixture"]));
    let reference = ReferenceBlob {
        retrieval_uri: name(&facts["reference_uri"]),
        blob: Blob {
            path: "parts.json",
            tree: "service-before-tree",
            oid: name(&facts["reference_blob"]),
            bytes: &reference_bytes,
            format: Format::Json,
        },
    };
    let element = name(&facts["query_element"]);
    let external_fixture = name(&facts["entry_fixture"]);
    for inventory_fixture in [name(&row["control"]["fixture"]), external_fixture] {
        let entry = fixture(inventory_fixture);
        let external = inventory_fixture == external_fixture;
        let tree = "service-before-tree";
        let index = index_schema(&AdmittedSchemaInput {
            api: API,
            authority: "schema",
            snapshot: Snapshot::Commit {
                tree,
                commit: "service-before-rev",
                subject: None,
                committer_date: Some("2026-11-25T12:00:00Z"),
            },
            entry: Blob {
                path: "openapi.json",
                tree,
                oid: "schema-blob",
                bytes: &entry,
                format: Format::Json,
            },
            retrieval_uri: external.then(|| name(&facts["entry_uri"])),
            references: if external {
                std::slice::from_ref(&reference)
            } else {
                &[]
            },
            config_receipts: &[],
            media_receipts: &[],
        })
        .unwrap();
        let field = index
            .fields
            .iter()
            .find(|field| field.id.to_string() == element)
            .unwrap();
        let mut scan = json_fixture(name(&facts["scan_fixture"]));
        scan["source_hash"] = json!(index.content_hash);
        scan["matches"] = json!([{
            "location_id":"saved-site","element_id":element,"grade":"bound",
            "signature":serde_json::from_str::<Value>(&signature(&field.shape)).unwrap()
        }]);
        let scan = serde_json::from_value(scan).unwrap();
        let mut wire = json_fixture(name(&row["fixture"]));
        wire["receipts"][0]["digest"]["value"] = json!(index.byte_sha256);
        if external {
            wire["receipts"][0]["retrieval_uri"] = facts["entry_uri"].clone();
        }
        wire["receipts"].as_array_mut().unwrap().push(json!({
            "id":"selected","kind":"blob","format":"json",
            "origin":{"kind":"named_tree","input_id":"service-after","repo":"pilot-producer",
                "tree":"service-after-tree","path":"openapi.json","blob":"schema-blob","availability":"available"},
            "digest":{"algorithm":"sha256","framing":"raw_bytes","value":format!("{:x}",Sha256::digest(&selected_bytes))}
        }));
        wire["claims"].as_array_mut().unwrap().push(json!({
            "id":"selected","input_id":"service-after","slot":"service-after","receipt_ids":["selected"]
        }));
        let request = serde_json::from_value(wire).unwrap();
        let mut bytes = supplied_bytes(&Value::Null);
        bytes.insert("schema".into(), entry.clone());
        bytes.insert("inventory-before".into(), entry);
        bytes.insert("selected".into(), selected_bytes.clone());
        let admitted = admit_schema_input(&request, &bytes, "schema-claim");
        assert!(admitted.input().unwrap().references.is_empty());
        let admitted_inventory = admitted.index().unwrap().unwrap();
        assert_eq!(admitted_inventory.fields.is_empty(), external);
        let saved_inventory = if external {
            &index
        } else {
            &admitted_inventory
        };
        let selected = admit_schema_input(&request, &bytes, "selected")
            .index()
            .unwrap()
            .unwrap();
        let query = serde_json::from_value(json!({
            "element_id":element,"selected_input_id":"service-after","side":"after","saved_location":"saved-site"
        })).unwrap();
        let result = saved_binding_status(
            &query,
            &SavedScanEvidence {
                admission: &request,
                bytes: &bytes,
                scan: Some(&scan),
                inventory: Some(saved_inventory),
                selected: &selected,
            },
        );
        assert_eq!(result.binding_status[0].status, "binding_current");
        assert_eq!(
            json!(result.admitted_saved_locations),
            row["observed"]["admitted_saved_locations"]
        );
    }
}

#[test]
fn known_limit_08_negative_uses_neighboring_file_scope() {
    let row = known("08");
    let mut change = json_fixture(name(&row["fixture"]))["change"].clone();
    change["after_presence"] = row["input"]["query_after_presence"].clone();
    change["after_element_id"] = Value::Null;
    let change = serde_json::from_value(change).unwrap();
    let mut wire = json_fixture(name(&row["input"]["binding_fixture"]));
    wire["operations"] = row["input"]["operations"].clone();
    for scope in [
        row["input"]["input_scope"].clone(),
        row["input"]["file_status_scope"].clone(),
    ] {
        wire["file_status"][0]["scope"] = scope;
        let binding = serde_json::from_value(wire.clone()).unwrap();
        let result = unobserved_elements(&MembershipRequest {
            change: &change,
            producer: &RoleSlots {
                before_input_id: Some("service-before".into()),
                after_input_id: None,
                reason: None,
            },
            consumer: None,
            expected_inputs: &["service-before".into()],
            admitted_bindings: &[binding],
            membership_evidence: &[
                json!({"input_id":"service-before","side":"before","complete":true}),
            ],
        });
        assert_eq!(
            json!(result["unobserved"].as_array().unwrap().len()),
            row["observed"]["unobserved_count"]
        );
        assert_eq!(result["coverage"], row["observed"]["coverage"]);
    }
}

#[test]
fn known_limit_09_draft_schema_accepts_inverted_metric() {
    let row = known("09");
    let control = json_fixture(name(&row["fixture"]));
    let facts = &row["input"];
    let mut mutated = control.clone();
    let metric = mutated.pointer_mut(name(&facts["path"])).unwrap();
    for key in ["observed", "total", "reason"] {
        metric[key] = facts[key].clone();
    }
    let own = crate::doc_schema::load("impact").unwrap();
    let opposite = crate::doc_schema::load("bindings").unwrap();
    crate::doc_schema::validate(&own, &opposite, &control, &mutated, false, true).unwrap();
    assert!(impact(control).validate().counts.is_empty());
    assert_eq!(
        json!(impact(mutated).validate().counts),
        row["observed"]["typed_counts"]
    );
}

#[test]
fn known_limit_10_unknown_key_refuses_envelope() {
    let row = known("10");
    let base = json_fixture(name(&row["fixture"]));
    let mut damaged = base.clone();
    let parent = name(&row["input"]["path"]).rsplit_once('/').unwrap();
    damaged.pointer_mut(parent.0).unwrap()[parent.1] = row["input"]["value"].clone();
    let error = serde_json::from_value::<AdmissionRequest>(damaged).unwrap_err();
    assert!(error
        .to_string()
        .contains(name(&row["observed"]["decode_error_field"])));
    let mut control = base;
    control["receipts"][0]["kind"] = json!("future");
    let report = admit_inputs(
        &serde_json::from_value(control).unwrap(),
        &supplied_bytes(&Value::Null),
    );
    assert_eq!(report.claims["sibling"].status, "admitted");
    assert_eq!(report.counts.get("unsupported_kind"), Some(&1));
}
