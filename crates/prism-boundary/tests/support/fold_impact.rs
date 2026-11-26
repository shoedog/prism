use crate::doc_cases::{fixture, json_fixture, schema};
use prism_boundary::{diff_schemas, BindingsDocument, ImpactDocument};
use serde_json::{json, Value};

fn impact(name: &str) -> ImpactDocument {
    serde_json::from_value(json_fixture(name)).unwrap()
}

#[test]
fn fold_service_only_validates_role_slots_and_dependencies() {
    let control = impact("operations-only.json");
    assert!(control.validate().counts.is_empty());
    let mut missing = control.clone();
    missing.inputs[0].dependencies.push("missing".into());
    assert!(missing.validate().counts.contains_key("missing_dependency"));
    let mut swapped = control.clone();
    swapped.change.producer.as_mut().unwrap().after_input_id = Some("missing".into());
    assert!(swapped.validate().counts.contains_key("missing_slot"));
    let mut malformed = control;
    malformed.inputs[0].api = "other".into();
    assert!(malformed.validate().counts.contains_key("slot_mismatch"));
}

#[test]
fn fold_usage_links_are_required_for_present_sides() {
    let control = impact("mixed.json");
    assert_eq!(control.validate().claims["actual_after"].status, "admitted");
    for key in [
        "match_before",
        "match_after",
        "match_service_first",
        "match_client_first",
    ] {
        let mut wire = json_fixture("mixed.json");
        wire["usages"][0][key] = Value::Null;
        let changed: ImpactDocument = serde_json::from_value(wire).unwrap();
        let report = changed.validate();
        assert_eq!(report.claims["actual_after"].status, "admitted");
        assert!(report.counts.contains_key("missing_dependency"), "{key}");
    }
}

#[test]
fn fold_decision_ids_cannot_overwrite_comparisons() {
    let mut doc = impact("mixed.json");
    doc.comparisons[0].purpose = "wrong".into();
    let control = doc.validate();
    assert_eq!(control.claims["actual_before"].status, "unchecked");
    assert_eq!(control.claims["actual_after"].status, "admitted");
    doc.usages[0].usage_id = Some("actual_before".into());
    doc.usages[0].match_before = None;
    doc.usages[0].match_service_first = None;
    let report = doc.validate();
    assert_eq!(report.claims["actual_before"].status, "unchecked");
    assert_eq!(report.claims["actual_after"].status, "admitted");
}

#[test]
fn fold_binding_current_requires_present_selection() {
    let control = impact("operations-only.json");
    assert!(control.validate().counts.is_empty());
    let mut input = json_fixture("operations-only.json");
    input["binding_status"] = json!([{
        "scan_id":"unrecorded","element_id":"op:GET /kiln","selected_input_id":"missing",
        "side":"after","saved_location":"unknown-location","status":"binding_current",
        "reason":null,"selected_presence":"absent","saved_shape":[],"selected_shape":[]
    }]);
    let doc: ImpactDocument = serde_json::from_value(input.clone()).unwrap();
    assert!(!doc.validate().counts.is_empty());
    assert!(ImpactDocument::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
fn fold_diff_basis_mismatch_is_visible() {
    let bytes = fixture("inventory.json");
    let before = schema(&bytes, "service-before");
    let after = schema(&bytes, "service-after");
    let control =
        impact("operations-only.json").with_schema_diff(&diff_schemas(&before, &after).unwrap());
    assert!(control.validate().counts.is_empty());
    let wrong = schema(&bytes, "different-tree");
    let damaged = control.with_schema_diff(&diff_schemas(&before, &wrong).unwrap());
    assert!(!damaged.validate().counts.is_empty());
}

#[test]
fn fold_inventory_cannot_relabel_source() {
    let mut source = schema(&fixture("inventory.json"), "service-before");
    let doc: BindingsDocument = serde_json::from_value(json_fixture("producer.json")).unwrap();
    let control =
        serde_json::to_value(doc.clone().with_schema_inventory(&source).unwrap()).unwrap();
    assert_eq!(control["schema"]["source"]["path"], json!("openapi.json"));
    source.source.as_mut().unwrap().path = "other.json".into();
    assert!(doc.with_schema_inventory(&source).is_err());
}

#[test]
fn fold_integer_decimal_spelling_is_exact() {
    let mut input = json_fixture("producer.json");
    assert!(BindingsDocument::from_json(&serde_json::to_vec(&input).unwrap()).is_ok());
    input["operations"][0]["sites"][0]["line"] = serde_json::from_str("1.0").unwrap();
    assert!(BindingsDocument::from_json(&serde_json::to_vec(&input).unwrap()).is_ok());
    input["operations"][0]["sites"][0]["line"] =
        serde_json::from_str("1.0000000000000000001").unwrap();
    assert!(BindingsDocument::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
fn fold_root_limit_preserves_unknown_field_details() {
    let before = schema(&fixture("empty.json"), "service-before");
    let after_bytes = br#"{"swagger":"2.0","paths":{"/kiln":{"get":{"responses":{"200":{"schema":{"type":"object","properties":{"x":{"type":"string"}}}}}}}}}"#;
    let after = schema(after_bytes, "service-after");
    let diff = diff_schemas(&before, &after).unwrap();
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(
        diff.changes[0].after.fields[0].presence,
        prism_boundary::Presence::Unknown
    );
    let doc = impact("operations-only.json").with_schema_diff(&diff);
    let wire = serde_json::to_value(doc).unwrap();
    assert_eq!(
        wire["change"]["elements"][0]["after_fields"][0]["presence"],
        json!("unknown")
    );
    let links = wire["change"]["elements"][0]["coverage_limits"]
        .as_array()
        .unwrap();
    assert!(wire["coverage"].as_array().unwrap().iter().any(|limit| {
        limit["reason"] == "media_unverified" && links.contains(&limit["limit_id"])
    }));
}

#[test]
fn fold_missing_actual_side_keeps_metric_total_unknown() {
    let mut wire = json_fixture("mixed.json");
    let mut client = json_fixture("consumer.json");
    client["calls"][0]["site_id"] = json!("call-before");
    let client: BindingsDocument = serde_json::from_value(client).unwrap();
    wire["comparisons"].as_array_mut().unwrap().truncate(1);
    wire["summary"]["unknown_target_count"]["version"] = json!("before");
    let control: ImpactDocument = serde_json::from_value(wire.clone()).unwrap();
    let control = control.with_usage_counts(std::slice::from_ref(&client));
    assert!(control.summary["unknown_target_count"].total.is_some());
    wire["summary"]["unknown_target_count"]["version"] = json!("joint");
    let damaged: ImpactDocument = serde_json::from_value(wire).unwrap();
    let damaged = damaged.with_usage_counts(&[client]);
    assert!(damaged.summary["unknown_target_count"].total.is_none());
    assert_eq!(
        damaged.summary["unknown_target_count"].observed,
        control.summary["unknown_target_count"].observed
    );
}

#[test]
fn fold_nested_computed_source_cannot_borrow_commit_time() {
    let mut wire = json_fixture("operations-only.json");
    let producer = json_fixture("producer.json");
    let input = &mut wire["inputs"][0];
    input["snapshot_kind"] = json!("computed_tree");
    input["rev"] = Value::Null;
    input["computed_at"] = json!("2026-11-26T12:00:00Z");
    input["merge_receipt"] = json!("merge");
    input["schema"] = producer["schema"].clone();
    input["schema"]["source"]["rev"] = Value::Null;
    let control: ImpactDocument = serde_json::from_value(wire.clone()).unwrap();
    assert!(control.validate().counts.is_empty());
    wire["inputs"][0]["schema"]["source"]["committed_at"] = json!("2026-11-25T12:00:00Z");
    let doc: ImpactDocument = serde_json::from_value(wire).unwrap();
    assert!(doc.validate().counts.contains_key("slot_mismatch"));
}

#[test]
fn fold_each_map_needs_its_client_occurrence_inventory() {
    let mut wire = json_fixture("mixed.json");
    for input in wire["inputs"].as_array_mut().unwrap() {
        if input["role"] == "consumer" {
            let id = if input["input_id"] == "client-before" {
                "call-before"
            } else {
                "call-after"
            };
            input["occurrence_ids"] = json!([id]);
        }
    }
    let control: ImpactDocument = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        control.validate().claims["service_first"].status,
        "admitted"
    );
    wire["inputs"][2]["occurrence_ids"]
        .as_array_mut()
        .unwrap()
        .push(json!("unmatched-site"));
    let doc: ImpactDocument = serde_json::from_value(wire).unwrap();
    let report = doc.validate();
    assert_eq!(report.claims["actual_after"].status, "admitted");
    assert_eq!(report.claims["actual_before"].status, "unchecked");
    assert_eq!(report.claims["service_first"].status, "unchecked");
}

#[test]
fn fold_message_records_distinguish_media_basis() {
    let doc: BindingsDocument = serde_json::from_value(json_fixture("producer.json")).unwrap();
    for (media, location) in [
        (
            r#""produces":["application/json"],"paths":{"/kiln":{"get":{"responses":{"200":{"schema":{"type":"string"}}}}}}"#,
            "document_media",
        ),
        (
            r#""paths":{"/kiln":{"get":{"produces":["application/json"],"responses":{"200":{"schema":{"type":"string"}}}}}}"#,
            "operation_media",
        ),
    ] {
        let bytes = format!(r#"{{"swagger":"2.0",{media}}}"#);
        let index = schema(bytes.as_bytes(), "service-before");
        let wire =
            serde_json::to_value(doc.clone().with_schema_inventory(&index).unwrap()).unwrap();
        let message = &wire["operations"][0]["message_roots"][0];
        assert_eq!(message["effective_media"], "application/json");
        assert_eq!(message["media_source"], location);
        assert!(message["selected_schema"].is_object());
    }
}

#[test]
fn fold_before_gap_links_after_operation_refusal() {
    let before = schema(
        br#"{"openapi":"3.0.3","paths":{"/kiln":{"get":null}}}"#,
        "service-before",
    );
    let after = schema(&fixture("inventory.json"), "service-after");
    let diff = diff_schemas(&before, &after).unwrap();
    assert!(!diff
        .changes
        .iter()
        .any(|row| row.operation.to_string() == "op:GET /kiln"));
    let doc = impact("operations-only.json").with_schema_diff(&diff);
    let limit = doc
        .coverage
        .iter()
        .find(|limit| {
            limit.side == "before"
                && limit.claim == "operation_presence"
                && limit.source != "input_inventory"
        })
        .expect("before-side gap limits operation comparison");
    assert_eq!(limit.comparison_id, "service_diff");
    assert!(doc
        .warnings
        .iter()
        .any(|claim| claim.coverage_limits.contains(&limit.limit_id)));
}

#[test]
fn fold_diff_scope_and_same_tree_file_are_verified() {
    use prism_boundary::element::schema_path;
    use prism_boundary::{diff_boundary, ComparisonScope, DiffBasis, DiffSide};
    let before = schema(&fixture("empty.json"), "service-before");
    let bytes = br#"{"openapi":"3.0.3","paths":{"/kiln":{"get":{"responses":{}}},"/neighbor":{"get":{"responses":{}}}}}"#;
    let after = schema(bytes, "service-after");
    let path = schema_path("/kiln", "", &[]);
    let scope = json!({"kind":"path_prefix","segments": path.segments});
    let mut wire = json_fixture("operations-only.json");
    for input in wire["inputs"].as_array_mut().unwrap() {
        input["scope"] = scope.clone();
    }
    let doc: ImpactDocument = serde_json::from_value(wire).unwrap();
    let diff_for = |index, tree| DiffSide {
        schema: Some(index),
        basis: Some(DiffBasis {
            api: "pilot-api",
            authority: "schema",
            tree,
            scope: ComparisonScope::PathPrefix(path.clone()),
        }),
        ..DiffSide::default()
    };
    let left = diff_for(&before, "service-before-tree");
    let right = diff_for(&after, "service-after-tree");
    let narrowed = diff_boundary(&left, &right).unwrap();
    let control = doc.clone().with_schema_diff(&narrowed);
    assert!(control.validate().counts.is_empty());
    assert_eq!(control.change.elements.len(), 1);
    assert_eq!(control.change.elements[0].operation_id, "op:GET /kiln");
    let full = doc
        .clone()
        .with_schema_diff(&diff_schemas(&before, &after).unwrap());
    assert!(full.validate().counts.contains_key("slot_mismatch"));
    assert!(full.change.elements.is_empty());
    let mut doc = doc;
    if let prism_boundary::revision::ReceiptOrigin::NamedTree { path, .. } =
        &mut doc.inputs[1].origin
    {
        *path = "other-file.json".into();
    }
    let wrong_source = doc.with_schema_diff(&narrowed);
    assert!(wrong_source.validate().counts.contains_key("slot_mismatch"));
}

#[test]
fn fold_change_selection_resolves_counterpart_at_cutoff() {
    let mut doc = impact("client-only.json");
    assert!(doc.validate().counts.is_empty());
    doc.change.selection.as_mut().unwrap().input_id = Some("client-after".into());
    assert!(doc.validate().counts.contains_key("missing_cutoff"));
    assert_eq!(
        doc.validate().claims["input:client-after"].status,
        "admitted"
    );
    let mut doc = impact("client-only.json");
    doc.change.selection = None;
    assert!(doc.validate().counts.contains_key("missing_cutoff"));
}
