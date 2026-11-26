use crate::doc_cases::{apply_patches, expect_paths, fixture, Case};
use crate::matrix::matrix_path;
use prism_boundary::revision::AdmissionRequest;
use prism_boundary::{admit_inputs, admit_schema_input, index_schema};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

pub fn supplied_bytes(data: &Value) -> BTreeMap<String, Vec<u8>> {
    let mut bytes = BTreeMap::from([("schema".into(), fixture("inventory.json"))]);
    for (id, file) in data["files"].as_object().into_iter().flatten() {
        let name = file.as_str().expect("receipt byte fixture");
        bytes.insert(id.clone(), fixture(name));
    }
    bytes
}

pub fn check(case: &Case<'_>) -> Result<(), String> {
    let mut input = case.input();
    let mut bytes = supplied_bytes(case.data);
    if let Some(projection) = case.pinned("projection.json") {
        let prefix = "fixtures/control-document-tracked-dirtiness/";
        for (receipt_id, key) in [("schema", "admitted_blob"), ("worktree", "working_copy")] {
            let name = projection[key]
                .as_str()
                .expect("pinned tree or worktree file");
            bytes.insert(
                receipt_id.into(),
                fs::read(matrix_path("ids_matrix", &format!("{prefix}{name}"))).unwrap(),
            );
        }
        input["inputs"][0]["dirty"] = projection["tracked_dirty"].clone();
        input["receipts"][0]["digest"]["value"] =
            json!(format!("{:x}", Sha256::digest(&bytes["schema"])));
    }
    if let Some(id) = case.data["mutate_bytes"]["id"].as_str() {
        let mut content: Value = serde_json::from_slice(&bytes[id]).unwrap();
        apply_patches(&mut content, &case.data["mutate_bytes"]["patches"]);
        let content = serde_json::to_vec(&content).unwrap();
        let receipt = input["receipts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|receipt| receipt["id"] == id)
            .unwrap();
        receipt["digest"]["value"] = json!(format!("{:x}", Sha256::digest(&content)));
        bytes.insert(id.into(), content);
    }
    let request: AdmissionRequest = serde_json::from_value(input).expect("typed admission fixture");
    let project = case.data.get("project");
    let schema_admission;
    let basic_admission;
    let (report, admitted) = if project.is_some() {
        schema_admission = admit_schema_input(&request, &bytes, "schema-claim");
        (&schema_admission.report, schema_admission.input())
    } else {
        basic_admission = admit_inputs(&request, &bytes);
        (&basic_admission, None)
    };
    let actual = serde_json::to_value(report).unwrap();
    let mut errors: Vec<_> = expect_paths(&actual, &admission_expectations(case))
        .err()
        .into_iter()
        .collect();
    if case.id == "control-document-tracked-dirtiness"
        && actual.pointer("/claims/schema-claim/dirty") != Some(&json!(true))
    {
        errors.push("dirty context missing from admitted claim".into());
    }
    if let Some(project) = project {
        if let Err(e) = check_projection(admitted, project) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("admission\t{}", errors.join("; ")))
    }
}

fn admission_expectations(case: &Case<'_>) -> Value {
    let mut expected = json!({"/claims/sibling/status":"admitted"});
    if case.flag("admitted") {
        expected["/claims/schema-claim/status"] = json!("admitted");
        expected["/claims/schema-claim/input_id"] = json!("service-before");
        expected["/claims/schema-claim/reasons"] = json!([]);
        expected["/counts"] = json!({});
    } else {
        let refusal = case.data["refusal"]
            .as_object()
            .expect("scoped refusal expectation");
        let code = refusal["code"].as_str().unwrap();
        expected["/claims/schema-claim/status"] = json!("unchecked");
        expected[format!("/counts/{code}")] =
            json!(refusal.get("count").and_then(Value::as_u64).unwrap_or(1));
        if !refusal.contains_key("multiple_evidence") {
            expected["/claims/schema-claim/reasons/1"] = json!({"$missing":true});
        }
        for (key, value) in refusal {
            if !matches!(key.as_str(), "count" | "multiple_evidence") {
                expected[format!("/claims/schema-claim/reasons/0/{key}")] = value.clone();
            }
        }
    }
    for (pointer, value) in case.data["expect"].as_object().into_iter().flatten() {
        expected[pointer] = value.clone();
    }
    expected
}

fn check_projection(
    input: Option<prism_boundary::AdmittedSchemaInput<'_>>,
    expect: &Value,
) -> Result<(), String> {
    if expect.is_null() {
        return input
            .is_none()
            .then_some(())
            .ok_or("refused bytes reached reader".into());
    }
    let input = input.ok_or("admitted claim supplied no reader input")?;
    let index =
        index_schema(&input).map_err(|error| format!("reader refused admitted bytes: {error}"))?;
    let ids = index
        .operations
        .iter()
        .map(|op| op.id.to_string())
        .chain(index.fields.iter().map(|field| field.id.to_string()))
        .collect::<BTreeSet<_>>();
    let expected: BTreeSet<String> = serde_json::from_value(expect["ids"].clone()).unwrap();
    if !(ids == expected || (expect["subset"] == true && ids.is_superset(&expected))) {
        return Err(format!("projected {ids:?}, expected {expected:?}"));
    }
    let sources = index
        .source
        .as_ref()
        .ok_or("projection lost named-tree source")?;
    for (key, actual) in [("tree", &sources.tree), ("blob", &sources.blob)] {
        if let Some(expected) = expect[key].as_str() {
            if expected != actual {
                return Err(format!("{key} expected {expected}, got {actual}"));
            }
        }
    }
    let target_root_limits = index
        .operations
        .iter()
        .filter(|operation| operation.id.to_string() == "op:GET /kiln")
        .flat_map(|operation| &operation.messages)
        .filter(|message| {
            message.key
                == prism_boundary::RootKey::Response(prism_boundary::element::SuccessKey::Exact(
                    200,
                ))
        })
        .flat_map(|message| &message.limits);
    let reasons: BTreeSet<_> = index
        .limits
        .iter()
        .chain(target_root_limits)
        .map(|limit| limit.reason.as_str())
        .collect();
    for reason in expect["reasons"].as_array().into_iter().flatten() {
        if !reasons.contains(reason.as_str().unwrap()) {
            return Err(format!(
                "reader did not record {reason}; observed {reasons:?}"
            ));
        }
    }
    if let Some(observed) = expect["observed"].as_array() {
        let actual: BTreeSet<_> = index
            .observed_fields
            .iter()
            .map(|field| field.id.to_string())
            .collect();
        let expected: BTreeSet<_> = observed
            .iter()
            .map(|field| field.as_str().unwrap().to_owned())
            .collect();
        if !actual.is_superset(&expected) {
            return Err(format!("observed {actual:?}, expected {expected:?}"));
        }
    }
    for (id, keys) in expect["success"].as_object().into_iter().flatten() {
        let actual = index
            .operations
            .iter()
            .find(|op| op.id.to_string() == *id)
            .map(|op| {
                op.success_keys
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            });
        if json!(actual) != *keys {
            return Err(format!(
                "status keys for {id}: expected {keys}, got {actual:?}"
            ));
        }
    }
    Ok(())
}
