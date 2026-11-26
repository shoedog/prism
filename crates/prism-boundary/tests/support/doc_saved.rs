use crate::doc_admission::supplied_bytes;
use crate::doc_cases::{apply_patches, fixture, json_fixture, schema, Case};
use prism_boundary::element::MessageRoot;
use prism_boundary::model::{
    unobserved_elements, ElementChangeRecord, MembershipRequest, RoleSlots,
};
use prism_boundary::revision::{
    saved_binding_status, AdmissionRequest, SavedLocation, SavedQuery, SavedScan, SavedScanEvidence,
};
use prism_boundary::{diff_schemas, signature, BindingsDocument, SchemaIndex};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn check(case: &Case<'_>) -> Result<(), String> {
    if case.scenario == "membership" {
        return membership(case);
    }

    let mut seed = case.input();
    if let Some(pinned) = case.pinned("scan.json") {
        assert_eq!(seed["schema_inventory_id"], pinned["schema_inventory_id"]);
        assert!(pinned["saved_matches"].as_array().unwrap().is_empty());
        assert_eq!(seed["client_input_id"], pinned["client_input"]);
        if !case.flag("omit_inventory") {
            let inv = case
                .pinned("inventory.json")
                .expect("pinned admitted inventory");
            assert_eq!(inv["operations"][0], "op:GET /kiln");
        }
    }
    let inventory_bytes = fixture(case.data["inventory"].as_str().unwrap_or("inventory.json"));
    let selected_bytes = fixture(case.data["selected"].as_str().unwrap_or("inventory.json"));
    let inventory = schema(&inventory_bytes, "service-before");
    let side = case.data["side"].as_str().unwrap();
    let service_id = format!("service-{side}");
    let plain_selected = schema(&selected_bytes, &service_id);
    let element = case.data["element"].as_str().unwrap();
    if seed["source_hash"] == "from_inventory" {
        seed["source_hash"] = json!(inventory.content_hash);
    }
    if let Some(segments) = case.data["inventory_scope"].as_array() {
        let tokens: Vec<_> = segments
            .iter()
            .map(|s| json!([{"kind":"literal","value":s}]))
            .collect();
        seed["scope"] = json!({"kind":"path_prefix","segments":tokens});
    }
    let mut scan: SavedScan = serde_json::from_value(seed).unwrap();
    if let Some(save) = case.data["save"].as_str() {
        let mut location = SavedLocation {
            location_id: "saved-site".into(),
            element_id: element.into(),
            grade: "bound".into(),
            signature: saved_signature(save, element, &inventory),
        };
        scan.matches.push(location.clone());
        if case.flag("conflict_location") {
            location.signature = json!([["unknown"]]);
            scan.matches.push(location);
        }
    }
    let mut admission = json_fixture("admission.json");
    let mut bytes = supplied_bytes(&Value::Null);
    admission["receipts"][0]["digest"]["value"] =
        json!(format!("{:x}", Sha256::digest(&inventory_bytes)));
    if !case.flag("omit_inventory") {
        bytes.insert("inventory-before".into(), inventory_bytes.clone());
    }
    bytes.insert("schema".into(), inventory_bytes);
    let source = plain_selected.source.as_ref().unwrap();
    admission["receipts"].as_array_mut().unwrap().push(json!({
        "id":"selected-schema", "kind":"blob", "format":"json",
        "origin":{
            "kind":"named_tree","repo":"pilot-producer","input_id":service_id,
            "path":source.path,"tree":source.tree,"blob":source.blob,"availability":"available"
        },
        "digest":{"algorithm":"sha256","framing":"raw_bytes","value":plain_selected.byte_sha256}
    }));
    admission["claims"].as_array_mut().unwrap().push(json!({
        "id":"selected-schema","input_id":service_id, "slot":service_id,
        "receipt_ids":["selected-schema"]
    }));
    if service_id == "service-before" {
        admission["receipts"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()["dependencies"] = json!(["config"]);
    }
    bytes.insert("selected-schema".into(), selected_bytes);
    apply_patches(&mut admission, &case.data["receipt_patch"]);
    let admission: AdmissionRequest = serde_json::from_value(admission).unwrap();
    let admitted = prism_boundary::admit_schema_input(&admission, &bytes, "selected-schema");
    let selected = admitted.index().unwrap().unwrap_or(plain_selected);
    let query: SavedQuery = serde_json::from_value(json!({
        "element_id":element,"selected_input_id":case.data["selected_input_id"].as_str().unwrap_or(&service_id),
        "side":side,"saved_location":case.data["location"],
    })).unwrap();
    let evidence = SavedScanEvidence {
        admission: &admission,
        bytes: &bytes,
        scan: (!case.flag("omit_scan")).then_some(&scan),
        inventory: (!case.flag("omit_inventory")).then_some(&inventory),
        selected: &selected,
    };
    if case.flag("paired") {
        let diff = diff_schemas(&inventory, &selected).unwrap();
        assert!(
            diff.response_root_pairs
                .iter()
                .any(|pair| pair.fields.iter().any(|field| {
                    field.before.to_string() == element && field.after.to_string().contains(":201#")
                })),
            "paired staleness fixture must have the real pair"
        );
    }

    let result = saved_binding_status(&query, &evidence);
    let mut expected = case.data["expect"].clone();
    for (key, path) in [
        ("dimension", "/binding_status/0/refusal/dimension"),
        ("record_id", "/binding_status/0/refusal/record_id"),
    ] {
        if let Some(value) = case.data.get(key) {
            expected[path] = value.clone();
        }
    }
    if case.flag("omit_scan") || case.flag("omit_inventory") || case.id == "C18" {
        expected["/admitted_raw_inputs"] = json!(["client-before"]);
    }
    if case.id == "C18" {
        assert_eq!(inventory.content_hash, selected.content_hash);
        expected["/admitted_saved_locations"] = json!([]);
    }
    if let Some(status @ ("binding_current" | "binding_stale")) =
        expected["/binding_status/0/status"].as_str()
    {
        let kind = if element.starts_with("field:") {
            "field"
        } else {
            "operation"
        };
        let saved_shape = saved_signature(kind, element, &inventory);
        let selected_shape = saved_signature(kind, element, &selected);
        assert_eq!(
            saved_shape == selected_shape,
            status == "binding_current",
            "fixture must discriminate comparable shape for {}",
            case.id
        );
        expected["/binding_status/0/saved_shape"] = saved_shape;
        expected["/binding_status/0/selected_shape"] = selected_shape;
    }
    if case.flag("mismatch") {
        assert_ne!(inventory.content_hash, selected.content_hash);
        expected["/standing_mismatches/0"] = json!({
            "kind":"schema_mismatch",
            "saved_content_hash":inventory.content_hash,"selected_content_hash":selected.content_hash,
            "saved_info_version":inventory.info_version,"selected_info_version":selected.info_version,
        });
    }
    let serialized = serde_json::to_value(result).unwrap();
    crate::doc_cases::expect_paths(&serialized, &expected)
        .map_err(|error| format!("saved:{}\t{error}", expected["/binding_status/0/status"]))
}

fn saved_signature(kind: &str, element: &str, index: &SchemaIndex) -> Value {
    if kind == "inconsistent_field" {
        return json!([
            ["known", "object"],
            ["missing"],
            ["known", false],
            ["missing"],
            ["missing"]
        ]);
    }
    if kind == "field" {
        let field = index
            .fields
            .iter()
            .chain(&index.observed_fields)
            .find(|field| field.id.to_string() == element)
            .expect("real saved field observation");
        return serde_json::from_str(&signature(&field.shape)).unwrap();
    }
    let mut records: Vec<Value> = index
        .fields
        .iter()
        .filter(|field| match &field.id.root {
            MessageRoot::Request(op) | MessageRoot::Response(op, _) => op.to_string() == element,
        })
        .map(|field| {
            let shape: Value = serde_json::from_str(&signature(&field.shape)).unwrap();
            json!([field.id.to_string(), shape])
        })
        .collect();
    if kind == "reordered_operation" {
        assert!(records.len() > 1);
        records.reverse();
    }
    json!(records)
}

fn membership(case: &Case<'_>) -> Result<(), String> {
    let input = case.input();
    let change: ElementChangeRecord = serde_json::from_value(input["change"].clone()).unwrap();
    let expected_inputs: Vec<String> =
        serde_json::from_value(input["expected_inputs"].clone()).unwrap();
    let evidence = case.data["evidence"].as_array().unwrap();
    let mut bindings = Vec::new();
    for id in &expected_inputs {
        if evidence.iter().all(|row| row["input_id"] != *id) {
            continue;
        }
        let is_client = id.starts_with("client");
        let mut doc = json_fixture(if is_client {
            "consumer.json"
        } else {
            "producer.json"
        });
        doc["input_id"] = json!(id);
        doc["rev"] = json!(format!("{id}-rev"));
        doc["tree"] = json!(format!("{id}-tree"));
        doc["origin"]["input_id"] = json!(id);
        doc["origin"]["tree"] = doc["tree"].clone();
        if !is_client {
            doc["schema"]["source"]["tree"] = doc["tree"].clone();
            doc["schema"]["source"]["rev"] = doc["rev"].clone();
        }
        let member = case.data["member"].as_str();
        if !is_client {
            if (member == Some("producer-before") && id == "service-before")
                || (member == Some("producer-after") && id == "service-after")
            {
                let key = if id == "service-before" {
                    "before_element_id"
                } else {
                    "after_element_id"
                };
                doc["operations"][0]["fields"] = json!([{"field_id":input["change"][key], "signature": input["change"][if id == "service-before" {"before_shape"} else {"after_shape"}]}]);
            } else {
                doc["operations"] = json!([]);
            }
        } else if id == "client-before" {
            let element = &input["change"]["before_element_id"];
            let observation = match member {
                Some("candidate") => json!({"grade":"candidate","target_ids":[element]}),
                Some("overlap") => {
                    json!({"grade":"unbound","target_ids":[],"overlaps":[{"element_id":element}]})
                }
                Some("operation-only") => json!({"grade":"bound","target_ids":["op:GET /kiln"]}),
                Some("bound-field") => json!({"grade":"bound","target_ids":[element],
                    "field_use":[{"field_id":element,"site_id":"call-before","kind":"read","evidence":["client-blob"]}]}),
                _ => {
                    doc["calls"] = json!([]);
                    Value::Null
                }
            };
            if !observation.is_null() {
                doc["calls"][0]["saved_match"] = observation;
                doc["calls"][0]["saved_match"]["service_input_id"] = json!("service-before");
                doc["calls"][0]["mapped_path"] = json!("/kiln");
                doc["calls"][0]["original_url_text"] = json!("/kiln");
                doc["calls"][0]["target_reason"] = Value::Null;
            }
        } else {
            doc["calls"] = json!([]);
        }
        bindings.push(serde_json::from_value::<BindingsDocument>(doc).unwrap());
    }
    let result = unobserved_elements(&MembershipRequest {
        change: &change,
        producer: &RoleSlots {
            before_input_id: Some("service-before".into()),
            after_input_id: Some("service-after".into()),
            reason: None,
        },
        consumer: Some(&RoleSlots {
            before_input_id: Some("client-before".into()),
            after_input_id: Some("client-after".into()),
            reason: None,
        }),
        expected_inputs: &expected_inputs,
        admitted_bindings: &bindings,
        membership_evidence: evidence,
    });
    case.expect(&result)
        .map_err(|error| format!("membership\t{error}"))
}
