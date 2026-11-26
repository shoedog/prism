use crate::doc_cases::{expect_paths, fixture, json_fixture, schema, Case};
use prism_boundary::{diff_schemas, BindingsDocument, ImpactDocument, SnapshotInput};
use serde_json::{json, Value};

pub fn serialize_dto(input: &Value) -> Value {
    if input.get("report_id").is_some() {
        let doc: ImpactDocument =
            serde_json::from_value(input.clone()).expect("typed impact input");
        serde_json::to_value(doc).unwrap()
    } else {
        let doc: BindingsDocument =
            serde_json::from_value(input.clone()).expect("typed bindings input");
        serde_json::to_value(doc).unwrap()
    }
}

pub fn check(case: &Case<'_>) -> Result<(), String> {
    if case.scenario == "decode" || case.data.get("raw").is_some() {
        return decode(case);
    }
    let input = case.input();
    check_pinned_projection(case, &input);
    check_serialization_fixture(case, &input);
    if matches!(
        case.id,
        "B02" | "control-document-computed-tree-null-metadata"
    ) {
        let snapshot: SnapshotInput = serde_json::from_value(input.clone()).unwrap();
        let wire = serde_json::to_value(&snapshot).unwrap();
        if let Err(error) = expect_paths(&wire, &case.data["expect"]) {
            return Err(format!("snapshot_dto\t{error}"));
        }
        let decoded: SnapshotInput = serde_json::from_value(wire).unwrap();
        assert_eq!(decoded, snapshot);
    }

    let actual = match case.scenario {
        "round_trip" => return round_trip(case, &input),
        "semantic" => {
            let doc: ImpactDocument = serde_json::from_value(input).expect("typed impact input");
            serde_json::to_value(doc.validate()).unwrap()
        }
        "bindings" => serialize_bindings(case, input),
        "impact" => serialize_impact(case, input),
        _ => unreachable!("wire row scenario"),
    };
    case.expect(&actual)
        .map_err(|error| format!("{}\t{error}", case.scenario))?;
    if case.flag("resolve_matches") {
        resolve_matches(&actual)?;
    }
    Ok(())
}

fn check_serialization_fixture(case: &Case<'_>, input: &Value) {
    // For passthrough records, confirm the fixture itself expresses the expected claim.
    // Derived values are checked through the existing reader or supplied independently.
    if matches!(case.scenario, "bindings" | "impact")
        && case.data["inventory"].is_null()
        && case.data["diff"].is_null()
        && !["derive_extraction", "derive_coverage", "derive_counts"]
            .iter()
            .any(|flag| case.flag(flag))
    {
        assert_eq!(case.expect(input), Ok(()), "fixture for {}", case.id);
    }
}

fn round_trip(case: &Case<'_>, expected: &Value) -> Result<(), String> {
    let serialized = serialize_dto(expected);
    case.expect(&serialized)
        .map_err(|error| format!("dto_round_trip\t{error}"))?;
    if serialized != *expected {
        return Err("dto_round_trip\tserialized DTO differs from the frozen golden".into());
    }
    let bytes = serde_json::to_vec(&serialized).unwrap();
    let decoded = if expected.get("report_id").is_some() {
        let doc = ImpactDocument::from_json(&bytes)
            .map_err(|error| format!("dto_round_trip\t{error:?}"))?;
        serde_json::to_value(doc).unwrap()
    } else {
        let doc = BindingsDocument::from_json(&bytes)
            .map_err(|error| format!("dto_round_trip\t{error:?}"))?;
        serde_json::to_value(doc).unwrap()
    };
    (decoded == *expected)
        .then_some(())
        .ok_or("dto_round_trip\tdecode lost frozen evidence".into())
}

fn serialize_bindings(case: &Case<'_>, mut input: Value) -> Value {
    let inventory = case.data["inventory"]
        .as_str()
        .map(|name| schema(&fixture(name), "service-before"));
    if inventory.is_some() {
        input["operations"] = json!([]);
        input["schema"]["content_hash"] = Value::Null;
        input["schema"]["info_version"] = Value::Null;
    }
    let mut doc: BindingsDocument = serde_json::from_value(input).expect("typed bindings input");
    if let Some(inventory) = &inventory {
        if case.id == "B04" && case.data["name"] == "file" {
            assert_eq!(
                case.data["expect"]["/schema"]["content_hash"],
                inventory.content_hash
            );
        }
        if case.id == "B19" {
            assert!(inventory.fields.is_empty());
            assert_eq!(
                inventory.observed_fields[0].id.to_string(),
                "field:resp:GET /loom:200#/x"
            );
        }
        doc = doc.with_schema_inventory(inventory);
    }
    if case.flag("derive_extraction") {
        doc = doc.with_extraction_coverage();
    }
    serde_json::to_value(doc).unwrap()
}

fn serialize_impact(case: &Case<'_>, mut input: Value) -> Value {
    let diff_inputs = case.data["diff"].as_array().map(|files| {
        let before = schema(&fixture(files[0].as_str().unwrap()), "service-before");
        let after = schema(&fixture(files[1].as_str().unwrap()), "service-after");
        for field in [
            "elements",
            "status_additions",
            "status_removals",
            "response_root_pairs",
        ] {
            input["change"][field] = json!([]);
        }
        (before, after)
    });
    if case.flag("derive_counts") {
        return usage_counts(input);
    }
    let mut doc: ImpactDocument = serde_json::from_value(input).expect("typed impact input");
    if let Some((before, after)) = &diff_inputs {
        let diff = diff_schemas(before, after).expect("admitted diff fixture");
        if case.id == "B24" {
            let expected_pairs = case.data["expect"]["/change/response_root_pairs"][0]["fields"]
                .as_array()
                .unwrap();
            let real_pairs = &diff.response_root_pairs[0].fields;
            assert_eq!(real_pairs.len(), expected_pairs.len());
            assert!(
                real_pairs
                    .iter()
                    .zip(expected_pairs)
                    .all(|(real, reference)| {
                        real.before.to_string() == reference["before_element_id"].as_str().unwrap()
                            && real.after.to_string()
                                == reference["after_element_id"].as_str().unwrap()
                    }),
                "fixture preserves the real side-local root IDs"
            );
            assert_eq!(
                diff.changes.len(),
                if case.data["name"] == "move-only" {
                    2
                } else {
                    3
                }
            );
        }
        doc = doc.with_schema_diff(&diff);
    }
    if case.flag("derive_coverage") {
        let limits = std::mem::take(&mut doc.coverage);
        doc = doc.with_coverage_limits(&limits);
    }
    serde_json::to_value(doc).unwrap()
}

fn usage_counts(mut input: Value) -> Value {
    let mut client = json_fixture("consumer.json");
    let prototype = client["calls"][0].clone();
    let mut raw = Vec::new();
    let mut matches = serde_json::Map::new();
    for (id, path, reason, status) in [
        (
            "missing-map",
            Some("/kiln"),
            "origin_unchecked",
            "unchecked",
        ),
        ("unreadable", None, "dynamic_url", "unchecked"),
        ("checked-miss", Some("/forge"), "no_operation", "checked"),
        ("other-api", Some("/elsewhere"), "other_api", "checked"),
    ] {
        let mut call = prototype.clone();
        call["site_id"] = json!(id);
        call["mapped_path"] = json!(path.filter(|_| reason != "origin_unchecked"));
        call["original_url_text"] = json!(path.unwrap_or("urlFromState()"));
        call["target_reason"] = json!((status == "unchecked").then_some(reason));
        let offset = raw.len() * 10;
        call["site"]["span"] = json!([offset, offset + 8]);
        matches.insert(id.into(), json!({
            "client_input_id":"client-before", "service_input_id":"service-held", "status":status,
            "grade": if status=="checked" { Some("unbound") } else { None },
            "reason":reason, "mapping":null, "target_ids":[], "alternates":[], "overlaps":[],
            "prefix_hints":[]
        }));
        raw.push(call);
    }
    client["calls"] = json!(raw);
    input["comparisons"][0]["matches"] = json!(matches);
    input["usages"] = json!([]);
    let doc: ImpactDocument = serde_json::from_value(input).unwrap();
    let client: BindingsDocument = serde_json::from_value(client).unwrap();
    serde_json::to_value(doc.with_usage_counts(&[client])).unwrap()
}

fn resolve_matches(report: &Value) -> Result<(), String> {
    let comparisons = report["comparisons"]
        .as_array()
        .ok_or("missing comparison records")?;
    for usage in report["usages"].as_array().ok_or("missing usages")? {
        for key in [
            "match_before",
            "match_after",
            "match_service_first",
            "match_client_first",
        ] {
            let reference = &usage[key];
            let Some(id) = reference["comparison_id"].as_str() else {
                continue;
            };
            let site = reference["site_id"]
                .as_str()
                .ok_or("match reference lacks a site ID")?;
            let comparison = comparisons
                .iter()
                .find(|pair| pair["comparison_id"] == id)
                .ok_or_else(|| format!("unknown match reference {id}"))?;
            let result = comparison["matches"]
                .get(site)
                .ok_or_else(|| format!("missing occurrence {site} for {id}"))?;
            for field in ["service_input_id", "client_input_id"] {
                if result[field] != comparison[field] {
                    return Err(format!("occurrence {site} in {id} has wrong {field}"));
                }
            }
        }
    }
    Ok(())
}

fn check_pinned_projection(case: &Case<'_>, input: &Value) {
    let Some(pinned) = case.pinned("projection.json") else {
        return;
    };
    match case.id {
        "control-document-computed-tree-null-metadata" => {
            assert_eq!(input["snapshot_kind"], pinned["snapshot_kind"]);
            assert_eq!(input["committed_at"], pinned["committer_date"]);
            assert_eq!(input["tree"], pinned["tree"]);
        }
        "control-document-configured-provenance" => {
            for (field, key) in [
                ("operation_id", "operation"),
                ("suffix", "suffix"),
                ("plugin", "plugin"),
            ] {
                assert_eq!(input["operations"][0][field], pinned[key]);
            }
            assert_eq!(input["config_sources"][0]["entry"], pinned["config_entry"]);
        }
        "control-document-keyless-coverage" => {
            assert_eq!(input["entries"][0]["path"], pinned["coverage"][0]["path"]);
            assert_eq!(
                input["entries"][0]["status"],
                pinned["coverage"][0]["status"]
            );
        }
        "control-document-side-presence" => {
            assert_eq!(pinned["before"]["coverage"], "checked");
            assert_eq!(pinned["after"]["coverage"], "partial");
            assert_eq!(pinned["after"]["unreadable"][0], "routes.js");
        }
        "control-document-unbound-raw-call" => {
            assert_eq!(
                input["calls"][0]["requested_method"],
                pinned["calls"][0]["method"]
            );
            assert_eq!(input["calls"][0]["mapped_path"], pinned["calls"][0]["path"]);
            assert_eq!(
                input["calls"][0]["target_reason"],
                pinned["calls"][0]["target_reason"]
            );
        }
        "control-document-joint-edit-four-comparisons"
        | "control-document-client-only-held-service" => {
            let before = pinned["clients"]["client-before"][0]["id"]
                .as_str()
                .unwrap();
            let after = pinned["clients"]["client-after"][0]["id"].as_str().unwrap();
            assert!(input["comparisons"][0]["matches"].get(before).is_some());
            assert!(input["comparisons"][1]["matches"].get(after).is_some());
            let expected = &pinned["services"]["service-before"];
            assert_eq!(
                &input["comparisons"][0]["matches"][before]["target_ids"],
                expected
            );
        }
        _ => unreachable!("pinned document projection control"),
    }
}

pub fn decode(case: &Case<'_>) -> Result<(), String> {
    let expected = &case.data["expect"];
    let bytes = if let Some(raw) = case.data["raw"].as_str() {
        raw.as_bytes().to_vec()
    } else {
        let input = case.input();
        if let Some(projection) = case.pinned("projection.json") {
            assert_eq!(input["snapshot_kind"], projection["snapshot_kind"]);
            if case.data["name"] == "committed_at" {
                assert_eq!(input["committed_at"], projection["committer_date"]);
            }
        }
        match case.data["duplicate"].as_str() {
            Some(pointer) => duplicate_member(input, pointer),
            None => serde_json::to_vec(&input).unwrap(),
        }
    };
    let actual = if expected["document"] == "impact" {
        ImpactDocument::from_json(&bytes).map(|doc| !doc.report_id.is_empty())
    } else {
        BindingsDocument::from_json(&bytes).map(|doc| doc.input.is_some())
    };
    if let Some(error) = expected["error"].as_str() {
        if actual.err().map(|error| format!("{error:?}")).as_deref() != Some(error) {
            return Err(format!(
                "decode:{error}\texpected rejection for {}",
                case.data["name"]
            ));
        }
    } else if !matches!(actual, Ok(true)) {
        return Err(format!(
            "decode:control\tsupported document lost identity: {actual:?}"
        ));
    }
    Ok(())
}

fn duplicate_member(mut doc: Value, pointer: &str) -> Vec<u8> {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let object = doc.pointer_mut(parent).unwrap();
    let value = object.get(key).unwrap();
    let original = serde_json::to_string(object).unwrap();
    let repeated = format!(
        "{},{key}:{value}}}",
        &original[..original.len() - 1],
        key = json!(key)
    );
    if parent.is_empty() {
        return repeated.into_bytes();
    }
    let marker = json!("p1d-test-duplicate-member-placeholder");
    *object = marker.clone();
    serde_json::to_string(&doc)
        .unwrap()
        .replacen(&marker.to_string(), &repeated, 1)
        .into_bytes()
}
