use super::matrix::{fixture, rows, Row};
use super::wire_cases::{members, run, strings};
use prism_boundary::{signature, EnumValue, Property, SchemaIndex};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub fn is_wire_row(row: &Row<'_>) -> bool {
    if row.id.starts_with("control-document-")
        || row.id.starts_with("control-g0")
        || row.id.starts_with("control-g1")
        || row.id.starts_with("control-p2-")
    {
        return false;
    }
    !row.fields.is_empty()
        || matches!(
            row.class,
            "allof"
                | "collision"
                | "cycle"
                | "depth-cut"
                | "field-punctuation"
                | "formdata"
                | "oneof-anyof"
                | "request-body"
                | "status-keys"
                | "wire-neutral-rename"
                | "wire-tree"
        )
        || row.id == "control-s1-oas31-external-path-item-server-withheld"
}

pub fn companion() -> Value {
    serde_json::from_str(include_str!("../fixtures/ids_matrix/expectations.json")).unwrap()
}

pub fn check_companion(row: &Row<'_>, expected: &Value, inputs: &[Value]) {
    for reason in expected["reason"]
        .as_str()
        .or(row.reason)
        .into_iter()
        .chain(row.warnings.iter().copied())
    {
        assert!(
            super::wire_cases::names_reason(inputs, reason),
            "{} has no coverage or warning assertion for {reason}",
            row.id
        );
    }
    if let Some(coverage) = expected["coverage"].as_str() {
        let dispositions = inputs
            .iter()
            .flat_map(|input| members(&input["operations"]))
            .flat_map(|(_, op)| members(&op["roots"]))
            .map(|(_, root)| root["coverage"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        if coverage == "not_checked" {
            assert!(
                inputs
                    .iter()
                    .any(|input| input.get("error").is_some() || input.get("limits").is_some()),
                "{} not-checked projection",
                row.id
            );
        } else {
            assert!(
                dispositions.contains(coverage)
                    || coverage == "partial" && dispositions.contains("withheld"),
                "{} {coverage} projection missing",
                row.id
            );
        }
    }
    if let Some(observed) = expected["structural_field_media"].as_str() {
        assert_eq!(observed, "unverified");
        assert!(!strings(&inputs[0]["observed"]).is_empty(), "{}", row.id);
    }
}

fn field_records(index: &SchemaIndex) -> BTreeMap<String, String> {
    index
        .fields
        .iter()
        .chain(&index.observed_fields)
        .map(|field| (field.id.to_string(), signature(&field.shape)))
        .collect()
}

fn success(index: &SchemaIndex) -> BTreeSet<String> {
    index
        .operations
        .iter()
        .flat_map(|op| op.success_keys.iter())
        .map(ToString::to_string)
        .collect()
}

#[test]
fn before_and_after_p1b_inventories() {
    let companion = companion();
    let mut failures = Vec::new();
    for row in rows().filter(is_wire_row) {
        let expected = &companion[row.id];
        let Some(before_name) = expected["before"].as_str() else {
            continue;
        };
        // The matrix also carries P1c expectations. Here we verify only their P1b input inventories.
        let before = run(&row, &json!({"fixture":before_name})).unwrap();
        let after = run(&row, &Value::Null).unwrap();
        let before_fields = field_records(&before);
        let after_fields = field_records(&after);
        let before_keys = success(&before);
        let after_keys = success(&after);
        let fields_added: Vec<_> = after_fields
            .keys()
            .filter(|id| !before_fields.contains_key(*id))
            .collect();
        let statuses_removed: Vec<_> = before_keys
            .difference(&after_keys)
            .map(String::as_str)
            .collect();
        let statuses_added: Vec<_> = after_keys
            .difference(&before_keys)
            .map(String::as_str)
            .collect();
        let mut valid = !before.operations.is_empty() && !after.operations.is_empty();
        if let Some(added) = expected["changes"].get("FieldAdded") {
            let pointers: Vec<_> = fields_added
                .iter()
                .map(|id| id.split_once('#').unwrap().1)
                .collect();
            valid &= pointers == strings(added);
        }
        for (actual, kind) in [
            (&statuses_removed, "SuccessStatusRemoved"),
            (&statuses_added, "SuccessStatusAdded"),
        ] {
            if let Some(keys) = expected["changes"].get(kind) {
                valid &= *actual == strings(keys);
            }
        }
        if expected["changes"]
            .as_object()
            .is_some_and(|m| m.is_empty())
        {
            valid &= before_fields == after_fields && before.content_hash == after.content_hash;
            if row.id == "control-formatting-only" {
                valid &= before.byte_sha256 != after.byte_sha256;
            }
        }
        for change in [
            "FieldFormatChanged",
            "FieldItemTypeChanged",
            "FieldRequiredAdded",
            "FieldRequiredRemoved",
            "EnumDeclarationChanged",
        ] {
            if let Some(pointers) = expected["changes"].get(change) {
                for pointer in strings(pointers) {
                    valid &= before_fields.iter().any(|(id, old)| {
                        id.ends_with(&format!("#{pointer}"))
                            && after_fields.get(id).is_some_and(|new| old != new)
                    });
                }
            }
        }
        if row.id == "control-enum-member-swap" {
            let enum_strings = |index: &SchemaIndex| {
                index
                    .fields
                    .iter()
                    .flat_map(|field| match &field.shape.enum_values {
                        Property::Known(values) => values.as_slice(),
                        _ => &[],
                    })
                    .filter_map(|member| match member {
                        EnumValue::String(value) => Some(value.clone()),
                        _ => None,
                    })
                    .collect::<BTreeSet<_>>()
            };
            let old = enum_strings(&before);
            let new = enum_strings(&after);
            valid &= new.difference(&old).map(String::as_str).collect::<Vec<_>>()
                == strings(&expected["changes"]["EnumValuesAdded"]);
            valid &= old.difference(&new).map(String::as_str).collect::<Vec<_>>()
                == strings(&expected["changes"]["EnumValuesRemoved"]);
        }
        if let Some(pair) = expected["element_pair"].as_array() {
            valid &= before_fields.contains_key(pair[0].as_str().unwrap())
                && after_fields.contains_key(pair[1].as_str().unwrap());
        }
        if row.id == "control-bodyless-success-loss" {
            valid &= before_fields == after_fields && before.content_hash != after.content_hash;
        }
        if !valid {
            failures.push(format!(
                "{}: before/after fields or explicit success keys differ from fixture expectations",
                row.id
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "paired inventory inputs\n{}",
        failures.join("\n")
    );
}

#[test]
fn named_tree_ignores_working_copy_schema() {
    let row = rows()
        .find(|row| row.id == "control-document-tracked-dirtiness")
        .unwrap();
    let receipt: Value = serde_json::from_slice(&fixture(&row, None)).unwrap();
    let index = run(&row, &json!({"fixture":receipt["admitted_blob"]})).unwrap();
    let ids: Vec<_> = index
        .operations
        .iter()
        .map(|op| op.id.to_string())
        .collect();
    let working = run(&row, &json!({"fixture":receipt["working_copy"]})).unwrap();
    assert!(
        ids == ["op:GET /kiln"]
            && working
                .operations
                .iter()
                .map(|op| op.id.to_string())
                .collect::<Vec<_>>()
                == ["op:GET /forge"],
        "named tree fixture must use its admitted schema and differ from the working copy"
    );
}
