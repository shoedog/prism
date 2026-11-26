use crate::doc_admission::supplied_bytes;
use crate::doc_cases::{fixture, json_fixture, schema};
use prism_boundary::model::{unobserved_elements, MembershipRequest, RoleSlots};
use prism_boundary::revision::{
    saved_binding_status, AdmissionRequest, SavedQuery, SavedScanEvidence,
};
use prism_boundary::BindingsDocument;
use serde_json::{json, Value};

fn prefix(path: &str) -> Value {
    json!({"kind":"path_prefix","segments":[[{"kind":"literal","value":path}]]})
}

fn saved(
    mutate: impl FnOnce(&mut Value, &mut Value),
    inventory_bytes: &[u8],
    selected_bytes: &[u8],
    element: &str,
) -> Value {
    let inventory = schema(inventory_bytes, "service-before");
    let selected = schema(selected_bytes, "service-after");
    let mut scan = json_fixture("scan.json");
    scan["source_hash"] = json!(inventory.content_hash);
    let mut admission = json_fixture("admission.json");
    admission["receipts"][0]["digest"]["value"] = json!(inventory.byte_sha256);
    admission["receipts"].as_array_mut().unwrap().push(json!({
        "id":"selected","kind":"blob","format":"json",
        "origin":{"kind":"named_tree","input_id":"service-after","repo":"pilot-producer",
            "tree":"service-after-tree","path":"openapi.json","blob":"schema-blob","availability":"available"},
        "digest":{"algorithm":"sha256","framing":"raw_bytes","value":selected.byte_sha256}
    }));
    admission["claims"].as_array_mut().unwrap().push(json!({
        "id":"selected","input_id":"service-after","slot":"service-after","receipt_ids":["selected"]
    }));
    mutate(&mut scan, &mut admission);
    let scan = serde_json::from_value(scan).unwrap();
    let admission: AdmissionRequest = serde_json::from_value(admission).unwrap();
    let mut bytes = supplied_bytes(&Value::Null);
    bytes.insert("schema".into(), inventory_bytes.to_vec());
    bytes.insert("inventory-before".into(), inventory_bytes.to_vec());
    bytes.insert("selected".into(), selected_bytes.to_vec());
    let query: SavedQuery = serde_json::from_value(json!({
        "element_id":element,"selected_input_id":"service-after","side":"after","saved_location":null
    })).unwrap();
    let report = saved_binding_status(
        &query,
        &SavedScanEvidence {
            admission: &admission,
            bytes: &bytes,
            scan: Some(&scan),
            inventory: Some(&inventory),
            selected: &selected,
        },
    );
    assert_eq!(report.admitted_raw_inputs, ["client-before"]);
    serde_json::to_value(report).unwrap()["binding_status"][0].clone()
}

#[test]
fn fold_saved_segments_evaluate_only_relevant_scopes() {
    let bytes = fixture("inventory.json");
    let element = "op:GET /kiln";
    let control = saved(|_, _| {}, &bytes, &bytes, element);
    assert_eq!(control["status"], "no_saved_match");
    let case = |path| {
        saved(
            |scan, admission| {
                admission["receipts"][8]["scope"] = prefix("kiln");
                scan["expected_segments"] = json!(["sender.js", "missing.js"]);
                scan["segments"] = json!([
                    {"path":"sender.js","scope":prefix("kiln"),"source_receipt_id":"scan-origin"},
                    {"path":"missing.js","scope":prefix(path),"source_receipt_id":null}
                ]);
            },
            &bytes,
            &bytes,
            element,
        )
    };
    assert_eq!(case("neighbor")["status"], "no_saved_match");
    assert_eq!(case("kiln")["status"], "not_checked");
}

#[test]
fn fold_empty_saved_segment_lists_do_not_establish_completeness() {
    let bytes = fixture("inventory.json");
    let element = "op:GET /kiln";
    let control = saved(|_, _| {}, &bytes, &bytes, element);
    assert_eq!(control["status"], "no_saved_match");
    let unknown = saved(
        |scan, _| {
            scan["expected_segments"] = json!([]);
            scan["admitted_segments"] = json!([]);
        },
        &bytes,
        &bytes,
        element,
    );
    assert_eq!(unknown["status"], "not_checked");
}

#[test]
fn fold_selected_slot_does_not_authorize_off_scope_presence() {
    let bytes = fixture("inventory.json");
    let element = "op:GET /kiln";
    let control = saved(|_, _| {}, &bytes, &bytes, element);
    assert_eq!(control["selected_presence"], "present");
    let result = saved(
        |_, admission| {
            admission["slots"][2]["scope"] = prefix("neighbor");
            admission["inputs"][2]["binding"]["scope"] = prefix("neighbor");
        },
        &bytes,
        &bytes,
        element,
    );
    assert_eq!(result["selected_presence"], "unknown");
    assert_eq!(result["status"], "not_checked");
}

#[test]
fn fold_observed_only_field_does_not_prove_saved_presence() {
    let observed = br#"{"swagger":"2.0","paths":{"/kiln":{"get":{"responses":{"200":{"schema":{"type":"object","properties":{"stamp":{"type":"string"}}}}}}}}}"#;
    let bytes = fixture("inventory.json");
    let element = "field:resp:GET /kiln:200#/stamp";
    let control = saved(|_, _| {}, &bytes, &bytes, element);
    assert_eq!(control["selected_presence"], "present");
    let result = saved(|_, _| {}, observed, observed, element);
    assert_eq!(result["selected_presence"], "unknown");
    assert_eq!(result["status"], "not_checked");
}

#[test]
fn fold_bound_call_overlap_withholds_membership_negative() {
    let seed = json_fixture("membership.json");
    let mut raw_change = seed["change"].clone();
    raw_change["after_presence"] = json!("absent");
    raw_change["after_element_id"] = Value::Null;
    let change = serde_json::from_value(raw_change).unwrap();
    let mut client = json_fixture("consumer.json");
    client["calls"][0]["saved_match"] = json!({
        "grade":"bound","target_ids":["op:GET /elsewhere"],"service_input_id":"service-before"
    });
    let mut producer = json_fixture("producer.json");
    producer["operations"] = json!([]);
    let ids = ["client-before".into(), "service-before".into()];
    let evidence = [
        json!({"input_id":"client-before","side":"before","complete":true}),
        json!({"input_id":"service-before","side":"before","complete":true}),
    ];
    let run = |client: Value| {
        let bindings: [BindingsDocument; 2] = [
            serde_json::from_value(client).unwrap(),
            serde_json::from_value(producer.clone()).unwrap(),
        ];
        unobserved_elements(&MembershipRequest {
            change: &change,
            producer: &RoleSlots {
                before_input_id: Some("service-before".into()),
                after_input_id: None,
                reason: None,
            },
            consumer: Some(&RoleSlots {
                before_input_id: Some("client-before".into()),
                after_input_id: None,
                reason: None,
            }),
            expected_inputs: &ids,
            admitted_bindings: &bindings,
            membership_evidence: &evidence,
        })
    };
    let control = run(client.clone());
    assert_eq!(control["unobserved"].as_array().unwrap().len(), 1);
    client["calls"][0]["saved_match"]["overlaps"] = json!([{"element_id":"op:GET /kiln"}]);
    assert_eq!(run(client)["coverage"][0]["reason"], "membership_possible");
}
