use crate::doc_cases::{fixture, json_fixture, schema};
use crate::fold2::{admission, admission_with_bytes, impact, live_pin, media_request};
use crate::known_limits::admitted_index;
use prism_boundary::model::{unobserved_elements, MembershipRequest, RoleSlots};
use prism_boundary::{signature, BindingsDocument};
use serde_json::{json, Value};

const CUTOFF: &str = "2026-11-25T12:00:00Z";

#[test]
fn fold3_non_json_declaration_keeps_origin() {
    let project = |media: &str, document: bool| {
        let mut swagger = json_fixture("swagger.json");
        let target = if document {
            &mut swagger
        } else {
            &mut swagger["paths"]["/kiln"]["get"]
        };
        target["produces"] = json!([media]);
        let index = admitted_index(&serde_json::to_vec(&swagger).unwrap());
        let mut binding = json_fixture("producer.json");
        binding["schema"]["source"]["sha256"] = json!(index.byte_sha256);
        let doc: BindingsDocument = serde_json::from_value(binding).unwrap();
        let wire = serde_json::to_value(doc.with_schema_inventory(&index).unwrap()).unwrap();
        wire["operations"][0]["message_roots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|root| root["status"] == "200")
            .unwrap()
            .clone()
    };
    for (document, source) in [(true, "document_media"), (false, "operation_media")] {
        assert_eq!(
            project("application/json", document)["media_source"],
            source
        );
    }
    for (document, source) in [(true, "document_media"), (false, "operation_media")] {
        let root = project("text/plain", document);
        assert_eq!(root["declared_media"], json!(["text/plain"]));
        assert_eq!(root["coverage"], "withheld");
        assert!(root["selected_schema"].is_null());
        assert_eq!(root["reader_limits"][0]["reason"], "non_json_media");
        assert_eq!(root["media_source"], source);
    }
}

#[test]
fn fold3_historical_computed_after_and_topic_head() {
    let mut wire = json_fixture("operations-only.json");
    wire["mode"] = json!("historical_as_of");
    wire["cutoff_at"] = json!(CUTOFF);
    wire["inputs"][1]["committed_at"] = json!(CUTOFF);
    wire["change"]["selection"] = json!({"subject_head_input_id":"service-after"});
    assert!(impact(wire.clone()).validate().counts.is_empty());

    let tree = "service-after-tree";
    let computed_at = "2026-11-26T11:00:00Z";
    let snapshot = json!({"snapshot_kind":"computed_tree","tree":tree,
        "computed_at":computed_at,"merge_receipt":"merge-result"});
    let origin = json!({"kind":"independent_cutoff","evidence":"exact result at cutoff",
        "available_at":CUTOFF,"covered_tree":tree});
    let mut topic = wire["inputs"][1].clone();
    for (key, value) in [
        ("input_id", "topic-head"),
        ("rev", "topic-head-rev"),
        ("tree", "topic-head-tree"),
    ] {
        topic[key] = json!(value);
    }
    topic["origin"]["input_id"] = topic["input_id"].clone();
    topic["origin"]["tree"] = topic["tree"].clone();
    let mut request = json_fixture("admission.json");
    request["mode"] = wire["mode"].clone();
    request["cutoff_at"] = json!(CUTOFF);
    request["subject_head_input_id"] = topic["input_id"].clone();
    request["inputs"][1]["selection"] = json!({"kind":"historical_pin","branch":"main",
        "rev":"client-before-rev","available_at":CUTOFF,"evidence":"archive"});
    let mut topic_binding = request["slots"][2].clone();
    for key in ["input_id", "tree", "rev"] {
        topic_binding[key] = topic[key].clone();
    }
    topic_binding["slot"] = topic["input_id"].clone();
    request["slots"]
        .as_array_mut()
        .unwrap()
        .push(topic_binding.clone());
    request["inputs"].as_array_mut().unwrap().push(json!({"binding":topic_binding,
        "snapshot":{"snapshot_kind":"commit","rev":topic["rev"],"tree":topic["tree"],"committed_at":CUTOFF}}));
    request["slots"][2]["rev"] = Value::Null;
    request["inputs"][2]["binding"]["rev"] = Value::Null;
    request["inputs"][2]["snapshot"] = snapshot.clone();
    request["merge"] = json!({"id":"merge-result","base":"service-before-rev","head":topic["rev"],
        "target":"main","procedure":"merge","result_tree":tree,"computed_at":computed_at,
        "cutoff_evidence":"exact-result"});
    request["expected_merge"] = request["merge"].clone();
    request["receipts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"exact-result","kind":"source","origin":origin}));
    request["claims"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"effective","input_id":"service-after",
        "slot":"service-after","purpose":"metadata","receipt_ids":[]}));
    let admitted = admission(request);
    assert_eq!(
        admitted.claims["effective"].status, "admitted",
        "{admitted:?}"
    );
    assert!(admitted.counts.is_empty(), "{admitted:?}");

    wire["inputs"].as_array_mut().unwrap().push(topic);
    let after = wire["inputs"][1].as_object_mut().unwrap();
    for key in ["rev", "committed_at", "subject"] {
        after.insert(key.into(), Value::Null);
    }
    after.extend(snapshot.as_object().unwrap().clone());
    after.insert("origin".into(), origin);
    wire["change"]["selection"] = json!({"subject_head_input_id":"topic-head",
        "merge_receipt":"merge-result","proposed_head":"topic-head-rev"});
    let encoded = serde_json::to_vec(&wire).unwrap();
    let decoded = prism_boundary::ImpactDocument::from_json(&encoded).unwrap();
    let report = decoded.validate();
    assert!(report.counts.is_empty(), "{:?}", report.counts);
    for id in ["input:service-after", "input:topic-head", "report-cutoff"] {
        assert_eq!(report.claims[id].status, "admitted", "{id}: {report:?}");
    }
    for (pointer, wrong) in [
        ("/change/selection/proposed_head", json!("other-topic-rev")),
        ("/inputs/2/committed_at", json!("2026-11-26T12:00:00Z")),
        ("/inputs/1/origin/covered_tree", json!("other-result-tree")),
        (
            "/inputs/1/origin/available_at",
            json!("2026-11-26T12:00:00Z"),
        ),
        ("/inputs/1/origin/covered_tree", Value::Null),
    ] {
        let mut damaged = wire.clone();
        *damaged.pointer_mut(pointer).unwrap() = wrong;
        let report = impact(damaged).validate();
        assert_eq!(
            report.claims["report-cutoff"].status, "unchecked",
            "{pointer}"
        );
    }
}

#[test]
fn fold3_required_media_self_cycle_refuses_schema() {
    let (mut input, bytes) = media_request();
    input["receipts"][0]["dependencies"] = json!(["config", "derived"]);
    input["receipts"].as_array_mut().unwrap().push(json!({
        "id":"derived","kind":"derived","origin":{"kind":"derived","version":"0.1"},
        "dependencies":["media-proof"]
    }));
    assert!(admission_with_bytes(input.clone(), &bytes)
        .counts
        .is_empty());
    input["receipts"][11]["dependencies"] = json!(["handler", "media-proof"]);
    let required = admission_with_bytes(input.clone(), &bytes);
    assert_eq!(required.claims["sibling"].status, "admitted");
    assert_eq!(
        required.claims["schema-claim"].status, "unchecked",
        "{required:?}"
    );
    assert_eq!(
        required.claims["schema-claim"].reasons[0].code,
        "dependency_cycle"
    );
    input["receipts"][0]["dependencies"] = json!(["config", "media-proof"]);
    let optional = admission_with_bytes(input, &bytes);
    assert_eq!(optional.claims["schema-claim"].status, "admitted");
    assert_eq!(optional.counts.get("dependency_cycle"), Some(&1));
}

#[test]
fn fold3_frozen_receipt_checks_expected_branch() {
    let mut wire = json_fixture("client-only.json");
    let receipt = wire["change"]["selection"]["receipt"].clone();
    wire["inputs"][0]["branch"] = json!("main");
    wire["change"]["selection"] = json!({"receipts":{"service-held":receipt}});
    assert!(impact(wire.clone()).validate().counts.is_empty());
    wire["change"]["selection"]["receipts"]["service-held"]["branch"] = json!("release");

    let mut input = json_fixture("admission.json");
    input["claims"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"held","input_id":"service-held",
        "slot":"service-held","purpose":"metadata","receipt_ids":[]}));
    input["inputs"][3]["selection"] = receipt;
    assert!(admission(input.clone()).counts.is_empty());
    input["inputs"][3]["selection"]["branch"] = json!("release");
    assert_eq!(admission(input).counts.get("missing_cutoff"), Some(&1));

    let report = impact(wire).validate();
    assert_eq!(report.claims["input:service-held"].status, "admitted");
    let cutoff = &report.claims["report-cutoff"];
    assert_eq!(cutoff.status, "unchecked");
    assert_eq!(cutoff.reasons.len(), 1);
    assert_eq!(cutoff.reasons[0].code, "missing_cutoff");
    assert_eq!(cutoff.reasons[0].record_id.as_deref(), Some("service-held"));
    assert_eq!(cutoff.reasons[0].dimension.as_deref(), Some("branch"));
    assert_eq!(report.counts.get("missing_cutoff"), Some(&4));
}

#[test]
fn fold3_current_binding_checks_normalized_linked_signature() {
    let mut wire = json_fixture("mixed.json");
    let mut scan = json_fixture("scan.json");
    let index = schema(&fixture("inventory.json"), "service-before");
    scan["source_hash"] = json!(index.content_hash);
    scan["matches"] = json!([{"location_id":"known-site","element_id":"op:GET /kiln",
        "grade":"bound","signature":[]}]);
    wire["saved_scans"] = json!([scan]);
    wire["binding_status"] = json!([{"scan_id":"scan-1","element_id":"op:GET /kiln",
        "selected_input_id":"service-before","side":"before","saved_location":"known-site",
        "status":"binding_current","reason":null,"selected_presence":"present",
        "saved_shape":[],"selected_shape":[]}]);
    assert!(impact(wire.clone()).validate().counts.is_empty());
    let shape = serde_json::from_str::<Value>(&signature(&index.fields[0].shape)).unwrap();
    wire["saved_scans"][0]["matches"][0]["signature"] = json!([
        ["field:resp:GET /kiln:200#/stamp", shape],
        ["field:resp:GET /kiln:200#/other", shape]
    ]);
    let mut reordered = wire.clone();
    reordered["binding_status"][0]["saved_shape"] = json!([
        ["field:resp:GET /kiln:200#/other", shape],
        ["field:resp:GET /kiln:200#/stamp", shape]
    ]);
    reordered["binding_status"][0]["selected_shape"] =
        reordered["binding_status"][0]["saved_shape"].clone();
    assert!(impact(reordered).validate().counts.is_empty());
    let report = impact(wire).validate();
    assert_eq!(
        report.claims["binding-status:0"].status, "unchecked",
        "{report:?}"
    );
    assert_eq!(report.counts.get("slot_mismatch"), Some(&1));
}

fn counterpart_admission() -> Value {
    let mut input = json_fixture("admission.json");
    let mut after = input["inputs"][1].clone();
    for (key, value) in [
        ("input_id", "client-after"),
        ("slot", "client-after"),
        ("rev", "client-after-rev"),
        ("tree", "client-after-tree"),
        ("branch", "topic"),
    ] {
        after["binding"][key] = json!(value);
    }
    after["snapshot"]["rev"] = json!("client-after-rev");
    after["snapshot"]["tree"] = json!("client-after-tree");
    after["selection"] = live_pin("client-after-rev");
    after["selection"]["branch"] = json!("topic");
    input["slots"]
        .as_array_mut()
        .unwrap()
        .push(after["binding"].clone());
    input["inputs"].as_array_mut().unwrap().push(after);
    input["claims"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"after","input_id":"client-after",
        "slot":"client-after","purpose":"metadata","receipt_ids":[]}));
    input
}

#[test]
fn fold3_client_subject_needs_no_service_side() {
    let mut input = counterpart_admission();
    input["subject_head_input_id"] = json!("client-after");
    input["inputs"][0]["selection"] = live_pin("service-before-rev");
    assert!(admission(input.clone()).counts.is_empty());
    input["inputs"][1]["selection"] = Value::Null;
    let report = admission(input);
    assert_eq!(report.claims["schema-claim"].status, "admitted");
    assert_eq!(report.claims["after"].status, "admitted");
    assert_eq!(report.claims["sibling"].status, "admitted", "{report:?}");
}

#[test]
fn fold3_admission_refuses_conflicting_branch_observations() {
    let mut input = counterpart_admission();
    assert!(admission(input.clone()).counts.is_empty());
    for path in [
        "/slots/4/branch",
        "/inputs/4/binding/branch",
        "/inputs/4/selection/branch",
    ] {
        *input.pointer_mut(path).unwrap() = json!("main");
    }
    input["inputs"][4]["selection"]["observed_at"] = json!("2026-11-26T05:00:00-07:00");
    let report = admission(input);
    assert_eq!(report.claims["schema-claim"].status, "admitted");
    assert_eq!(report.claims["sibling"].status, "unchecked", "{report:?}");
    assert_eq!(report.claims["after"].status, "unchecked");
    assert_eq!(report.counts.get("receipt_conflict"), Some(&2));
}

#[test]
fn fold3_frozen_refuses_conflicting_branch_observations() {
    let mut wire = json_fixture("mixed.json");
    let selection = &mut wire["change"]["selection"]["receipts"];
    selection["client-after"]["branch"] = json!("topic");
    wire["inputs"][2]["branch"] = json!("main");
    wire["inputs"][3]["branch"] = json!("topic");
    assert!(impact(wire.clone()).validate().counts.is_empty());
    wire["inputs"][3]["branch"] = json!("main");
    wire["change"]["selection"]["receipts"]["client-after"]["branch"] = json!("main");
    let report = impact(wire).validate();
    assert_eq!(report.claims["input:client-after"].status, "admitted");
    assert_eq!(
        report.claims["report-cutoff"].status, "unchecked",
        "{report:?}"
    );
    assert_eq!(report.counts.get("receipt_conflict"), Some(&1));
    assert_eq!(
        report.claims["report-cutoff"].reasons[0].evidence,
        ["client-after", "client-before"]
    );
}

#[test]
fn fold3_membership_needs_each_complete_region_to_cover_operation() {
    let mut change = json_fixture("membership.json")["change"].clone();
    change["after_presence"] = json!("absent");
    change["after_element_id"] = Value::Null;
    let change = serde_json::from_value(change).unwrap();
    let mut wire = json_fixture("producer.json");
    wire["operations"] = json!([]);
    let check = |wire| {
        unobserved_elements(&MembershipRequest {
            change: &change,
            producer: &RoleSlots {
                before_input_id: Some("service-before".into()),
                after_input_id: None,
                reason: None,
            },
            consumer: None,
            expected_inputs: &["service-before".into()],
            admitted_bindings: &[serde_json::from_value(wire).unwrap()],
            membership_evidence: &[
                json!({"input_id":"service-before","side":"before","complete":true}),
            ],
        })
    };
    let control = check(wire.clone());
    assert_eq!(control["unobserved"].as_array().unwrap().len(), 1);
    assert_eq!(control["coverage"], json!([]));
    let scope = |path| json!({"kind":"path_prefix","segments":[[{"kind":"literal","value":path}]]});
    wire["file_status"][0]["scope"] = scope("kiln");
    assert_eq!(check(wire.clone()), control);
    wire["file_status"][0]["scope"] = scope("neighbor");
    let result = check(wire);
    assert_eq!(result["unobserved"], json!([]));
    assert_eq!(
        result["coverage"],
        json!([{"input_id":"service-before","reason":"missing_membership_evidence"}])
    );
}
