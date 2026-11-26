use super::admission::admitted;
use super::matrix::{fixture, Row};
use prism_boundary::revision::{
    Blob, DependencyReceipt, Format, ProducerMediaEvidence, ReferenceBlob,
};
use prism_boundary::{
    index_schema, signature, Coverage, EnumValue, JsonMediaSource, MessageCoverage, Property,
    RegionLimit, RootKey, SchemaIndex,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect()
}

pub fn run(row: &Row<'_>, input: &Value) -> Result<SchemaIndex, prism_boundary::SchemaError> {
    let media = input["media_proof"]
        .as_str()
        .map(|name| fixture(row, Some(name)));
    run_catalog(row, input, media.as_deref())
}

pub fn run_catalog(
    row: &Row<'_>,
    input: &Value,
    media: Option<&[u8]>,
) -> Result<SchemaIndex, prism_boundary::SchemaError> {
    let bytes = fixture(row, input["fixture"].as_str());
    let references: Vec<_> = input["references"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    let ref_bytes: Vec<_> = references
        .iter()
        .map(|record| fixture(row, record["fixture"].as_str()))
        .collect();
    let references: Vec<_> = references
        .iter()
        .zip(&ref_bytes)
        .map(|(record, bytes)| ReferenceBlob {
            retrieval_uri: record["uri"].as_str().unwrap(),
            blob: Blob {
                path: record["path"].as_str().unwrap(),
                tree: record["tree"].as_str().unwrap(),
                oid: record["fixture"].as_str().unwrap(),
                format: Format::Json,
                bytes,
            },
        })
        .collect();
    let count = media.map_or(0, |bytes| {
        let proof: Value = serde_json::from_slice(bytes).unwrap();
        proof.as_array().unwrap().len()
    });
    let media_receipts: Vec<_> = (1..=count)
        .map(|entry| ProducerMediaEvidence {
            bytes: media.unwrap(),
            receipt: DependencyReceipt {
                input_id: "media-catalog",
                tree: "test-tree",
                path: input["media_proof"].as_str().unwrap_or("media.json"),
                blob: "media-catalog-blob",
                entry,
            },
        })
        .collect();
    let format = if row.fixture.ends_with(".yaml") {
        Format::Yaml
    } else {
        Format::Json
    };
    if format == Format::Json {
        serde_json::from_slice::<Value>(&bytes).expect("well-formed JSON fixture");
    }
    let mut input_record = admitted(
        &bytes,
        format,
        input["api"].as_str().unwrap_or("test-api"),
        input["retrieval_uri"].as_str(),
        &references,
    );
    input_record.media_receipts = &media_receipts;
    index_schema(&input_record)
}

// Record every mismatch so the RED receipt shows all inputs, including controls.
pub fn check(row: &Row<'_>, input: &Value, failures: &mut Vec<String>) {
    let label = format!(
        "{}/{}",
        row.id,
        input["fixture"].as_str().unwrap_or(row.fixture)
    );
    let result = run(row, input);
    if let Some(reason) = input["error"].as_str() {
        let actual = result.as_ref().err().map(|err| (err.code(), err.scope()));
        compare(
            failures,
            &label,
            "format refusal",
            actual,
            Some((reason, "schema_format")),
        );
        return;
    }
    let index = match result {
        Ok(index) => index,
        Err(error) => {
            failures.push(format!("{label}: admitted inventory returned {error}"));
            return;
        }
    };
    let mut operations: Vec<_> = index.operations.iter().map(|op| &op.id).collect();
    operations.sort();
    let actual_ops: Vec<_> = operations.iter().map(|id| id.to_string()).collect();
    let expected_ops = input["operations"].as_object().unwrap();
    compare(
        failures,
        &label,
        "operation IDs",
        actual_ops,
        expected_ops
            .keys()
            .map(|id| format!("op:{id}"))
            .collect::<Vec<_>>(),
    );
    let mut fields: Vec<_> = index
        .fields
        .iter()
        .map(|field| field.id.to_string())
        .collect();
    fields.sort();
    compare(
        failures,
        &label,
        "comparable field IDs",
        fields,
        strings(&input["fields"]),
    );
    let mut observed: Vec<_> = index
        .observed_fields
        .iter()
        .map(|field| field.id.to_string())
        .collect();
    observed.sort();
    compare(
        failures,
        &label,
        "observed-only field IDs",
        observed,
        strings(&input["observed"]),
    );
    compare(
        failures,
        &label,
        "inventory comparison limits",
        limit_counts(&index.limits, false),
        expected_counts(&input["limits"]),
    );
    compare(
        failures,
        &label,
        "warnings",
        counts(index.warnings.into_iter()),
        expected_counts(&input["warnings"]),
    );
    if let Some(api) = input["api"].as_str() {
        compare(
            failures,
            &label,
            "source API",
            index.source.as_ref().map(|source| source.api.as_str()),
            Some(api),
        );
    }
    for op in &index.operations {
        let op_id = op.id.to_string();
        let Some(expected) = expected_ops.get(&op_id[3..]) else {
            continue;
        };
        let label = format!("{label}/{op_id}");
        let mut success: Vec<_> = op.success_keys.iter().map(ToString::to_string).collect();
        success.sort();
        compare(
            failures,
            &label,
            "explicit success keys",
            success,
            strings(&expected["success"]),
        );
        let mut responses = op.response_keys.clone();
        responses.sort();
        compare(
            failures,
            &label,
            "response declarations",
            responses,
            strings(&expected["responses"]),
        );
        for (code, status) in members(&expected["selected"]) {
            compare(
                failures,
                &label,
                &format!("selected status for {code}"),
                op.response_key(code.parse().unwrap()),
                status.as_str(),
            );
        }
        let mut root_keys: Vec<_> = op.messages.iter().map(message_key).collect();
        root_keys.sort();
        compare(
            failures,
            &label,
            "message roots",
            root_keys,
            members(&expected["roots"])
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
        );
        for message in &op.messages {
            let key = message_key(message);
            let Some(expectation) = expected["roots"].get(&key) else {
                continue;
            };
            check_message(message, expectation, &format!("{label}/{key}"), failures);
        }
    }
    for (key, expected) in members(&input["signatures"]) {
        let actual = index
            .fields
            .iter()
            .find(|field| field.id.to_string() == *key)
            .map(|field| signature(&field.shape));
        compare(
            failures,
            &label,
            &format!("signature {key}"),
            actual.as_deref(),
            expected.as_str(),
        );
    }
    let type_value = |types: &Vec<String>| match types.as_slice() {
        [name] => json!(name),
        _ => json!(types),
    };
    for (key, shape) in members(&input["shapes"]) {
        let Some(field) = index
            .fields
            .iter()
            .chain(&index.observed_fields)
            .find(|field| field.id.to_string() == *key)
        else {
            failures.push(format!("{label}: field shape absent for {key}"));
            continue;
        };
        let shape_found = &field.shape;
        let values = [
            ("type", property(&shape_found.field_type, type_value)),
            ("format", property(&shape_found.format, |s| json!(s))),
            ("required", property(&shape_found.required, |b| json!(b))),
            (
                "enum",
                property(&shape_found.enum_values, |values| {
                    let mut values: Vec<_> = values.iter().map(enum_value).collect();
                    values.sort_by_key(Value::to_string);
                    json!(values)
                }),
            ),
            ("item_type", property(&shape_found.item_type, type_value)),
        ];
        for (name, actual) in values {
            if let Some(expected) = shape.get(name) {
                compare(
                    failures,
                    &label,
                    &format!("shape {key} {name}"),
                    &actual,
                    expected,
                );
            }
        }
    }
}

fn enum_value(value: &EnumValue) -> Value {
    match value {
        EnumValue::Null => Value::Null,
        EnumValue::Boolean(b) => json!(b),
        EnumValue::Number(n) => json!({"number": n}),
        EnumValue::String(s) => json!(s),
    }
}

fn property<T>(value: &Property<T>, known: impl FnOnce(&T) -> Value) -> Value {
    match value {
        Property::Missing => json!("missing"),
        Property::Known(value) => known(value),
        Property::BranchDependent => json!("branch_dependent"),
        Property::Unknown => json!("unknown"),
    }
}

fn check_message(
    message: &MessageCoverage,
    expected: &Value,
    label: &str,
    failures: &mut Vec<String>,
) {
    let coverage = match message.coverage {
        Coverage::Checked => "checked",
        Coverage::ObservedOnly => "observed_only",
        Coverage::Partial => "partial",
        Coverage::Withheld => "withheld",
    };
    compare(
        failures,
        label,
        "root coverage",
        Some(coverage),
        expected["coverage"].as_str(),
    );
    compare(
        failures,
        label,
        "scoped counted limits",
        limit_counts(&message.limits, true),
        expected_counts(&expected["limits"]),
    );
    if let Some(feasible) = expected.get("feasible") {
        compare(
            failures,
            label,
            "root feasibility",
            message.feasible,
            feasible.as_bool(),
        );
    }
    if let Some(media) = expected.get("declared_media") {
        let mut actual = message.declared_media.clone();
        actual.sort();
        compare(failures, label, "declared media", actual, strings(media));
    }
    if let Some(source) = expected.get("json_source") {
        let actual = match &message.json_source {
            Some(JsonMediaSource::MessageContent) => json!("message_content"),
            Some(JsonMediaSource::OperationMedia) => json!("operation_media"),
            Some(JsonMediaSource::DocumentMedia) => json!("document_media"),
            Some(JsonMediaSource::Producer { input_id, entry }) => {
                json!({"input":input_id, "entry":entry})
            }
            None => Value::Null,
        };
        compare(failures, label, "JSON media provenance", &actual, source);
    }
}

fn message_key(message: &MessageCoverage) -> String {
    match &message.key {
        RootKey::Request => "req".into(),
        RootKey::Response(key) => key.to_string(),
        RootKey::Fallback => "default".into(),
    }
}

fn limit_counts(limits: &[RegionLimit], scoped: bool) -> BTreeMap<String, usize> {
    counts(limits.iter().map(|limit| {
        if !scoped {
            return limit.reason.clone();
        }
        let mut path = String::new();
        for token in &limit.pointer {
            path.push('/');
            path.push_str(&token.to_string());
        }
        format!("{path}|{}", limit.reason)
    }))
}

fn counts(keys: impl IntoIterator<Item = String>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for key in keys {
        *counts.entry(key).or_default() += 1;
    }
    counts
}

fn expected_counts(value: &Value) -> BTreeMap<String, usize> {
    members(value)
        .map(|(key, count)| (key.clone(), count.as_u64().unwrap() as usize))
        .collect()
}

pub fn members(value: &Value) -> impl Iterator<Item = (&String, &Value)> {
    value.as_object().into_iter().flat_map(Map::iter)
}

fn compare<A, B>(failures: &mut Vec<String>, label: &str, claim: &str, actual: A, expected: B)
where
    A: PartialEq<B> + std::fmt::Debug,
    B: std::fmt::Debug,
{
    if actual != expected {
        failures.push(format!(
            "{label}: {claim}: expected {expected:?}, got {actual:?}"
        ));
    }
}

pub fn names_reason(inputs: &[Value], reason: &str) -> bool {
    let scope_suffix = format!("|{reason}");
    inputs.iter().any(|input| {
        input["error"] == reason
            || input["limits"].get(reason).is_some()
            || input["warnings"].get(reason).is_some()
            || members(&input["operations"]).any(|(_, op)| {
                members(&op["roots"]).any(|(_, root)| {
                    members(&root["limits"]).any(|(scope, _)| scope.ends_with(&scope_suffix))
                })
            })
    })
}
