use crate::diff_input::Input;
use crate::diff_rows::{entries, shape_value, sorted, Checks};
use crate::matrix::p1c_rows;
use prism_boundary::{element::MessageRoot, Field, SchemaIndex};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn audit_inputs() {
    let entries = entries();
    let mut failures = Vec::new();
    for row in p1c_rows() {
        let entry = &entries[row.id];
        let mut check = Checks {
            id: row.id,
            failures: &mut failures,
        };
        let before = Input::load(&entry["inputs"][0], 0);
        let after = Input::load(&entry["inputs"][1], 1);
        if let (Some(b), Some(a)) = (&before.schema, &after.schema) {
            audit_schema(&mut check, b, &entry["inputs"][0]);
            audit_schema(&mut check, a, &entry["inputs"][1]);
            audit_changes(&mut check, b, entry, "before");
            audit_changes(&mut check, a, entry, "after");
            audit_standing_slots(&mut check, b, entry, "Before");
            audit_standing_slots(&mut check, a, entry, "After");
            if entry["equal_content"] == true {
                check.eq(
                    "canonical content hash",
                    json!(b.content_hash),
                    json!(a.content_hash),
                );
            }
            if entry["changed_bytes"] == true {
                check.eq(
                    "different bytes",
                    json!(b.byte_sha256 != a.byte_sha256),
                    json!(true),
                );
            }
            if entry["changed_definitions"] == true {
                let definitions = |schema: &SchemaIndex| {
                    schema
                        .fields
                        .iter()
                        .flat_map(|f| f.definitions.clone())
                        .collect::<BTreeSet<_>>()
                };
                check.eq(
                    "changed definition identities",
                    json!(definitions(b) != definitions(a)),
                    json!(true),
                );
            }
        }
    }
    assert!(
        failures.is_empty(),
        "P1b preconditions\n{}",
        failures.join("\n")
    );
}

fn audit_schema(check: &mut Checks<'_>, schema: &SchemaIndex, expected: &Value) {
    let field_ids = |fields: &[Field]| {
        sorted(json!(fields
            .iter()
            .map(|f| f.id.to_string())
            .collect::<Vec<_>>()))
    };
    for (label, fields) in [
        ("fields", &schema.fields),
        ("observed", &schema.observed_fields),
    ] {
        if let Some(value) = expected.get(label) {
            check.eq(label, field_ids(fields), sorted(value.clone()));
        }
    }
    if let Some(value) = expected.get("success") {
        check.eq(
            "explicit success keys",
            sorted(json!(schema.operations[0]
                .success_keys
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>())),
            sorted(value.clone()),
        );
    }
    if let Some(fields) = expected["slots"].as_object() {
        for (id, slots) in fields {
            let actual = schema
                .fields
                .iter()
                .chain(&schema.observed_fields)
                .find(|field| field.id.to_string() == *id)
                .map(|field| shape_value(&field.shape))
                .unwrap_or_default();
            for (name, expected) in slots.as_object().unwrap() {
                check.eq(
                    &format!("{id} signed {name}"),
                    actual[name].clone(),
                    expected.clone(),
                );
            }
        }
    }
}

fn audit_changes(check: &mut Checks<'_>, schema: &SchemaIndex, entry: &Value, side: &str) {
    let wire = |id: &str| {
        schema
            .fields
            .iter()
            .find(|field| field.id.to_string() == id)
    };
    let observed = |id: &str| {
        schema
            .observed_fields
            .iter()
            .find(|field| field.id.to_string() == id)
    };
    for change in entry["changes"].as_array().unwrap() {
        if let Some(id) = change[side][0].as_str() {
            let projected = if id.starts_with("field:") {
                wire(id).is_some()
            } else if id.starts_with("op:") {
                schema.operations.iter().any(|op| op.id.to_string() == id)
            } else {
                // In D1 the final consensus is a new per-declaration check. This
                // audit can only validate the still-public lexical representative.
                schema.operations.iter().any(|op| {
                    op.success_keys.iter().any(|key| {
                        id == format!("status:{} {}:{key}", op.id.method(), op.id.path())
                    })
                })
            };
            check.eq(
                &format!("{side} projects {id}"),
                json!(projected),
                json!(true),
            );
        }
        if let Some(id) = change[format!("{side}_declaration")].as_str() {
            check.eq(
                &format!("{side} retains declaration {id}"),
                json!(wire(id).or_else(|| observed(id)).is_some()),
                json!(true),
            );
            if change[side][1] == "Absent" {
                check.eq(
                    &format!("{side} excludes empty-domain field {id}"),
                    json!(wire(id).is_none()),
                    json!(true),
                );
            }
        }
        if let Some(slots) = change[format!("{side}_slots")].as_object() {
            let id = change[format!("{side}_declaration")]
                .as_str()
                .or(change[side][0].as_str())
                .unwrap();
            let actual = wire(id)
                .or_else(|| observed(id))
                .map(|f| shape_value(&f.shape))
                .unwrap_or_default();
            for (slot, expected) in slots {
                check.eq(
                    &format!("{side} {id} signed {slot}"),
                    actual[slot].clone(),
                    expected.clone(),
                );
            }
        }
        if let Some(fields) = change[format!("{side}_fields")].as_array() {
            for detail in fields {
                let id = detail[0].as_str().unwrap();
                let projected = if detail[1] == "Present" {
                    wire(id)
                } else {
                    observed(id)
                };
                check.eq(
                    &format!("{side} retains field detail {id}"),
                    json!(projected.is_some()),
                    json!(true),
                );
            }
        }
    }
}

fn audit_standing_slots(check: &mut Checks<'_>, schema: &SchemaIndex, entry: &Value, side: &str) {
    if entry["subset_limits"] == true || entry["error"].is_string() {
        return;
    }
    let expected = entry["limits"].as_array().unwrap();
    for field in &schema.fields {
        let root = match &field.id.root {
            MessageRoot::Request(op) => format!("root:req:{} {}", op.method(), op.path()),
            MessageRoot::Response(op, key) => {
                format!("root:resp:{} {}:{key}", op.method(), op.path())
            }
        };
        let pointer = field
            .id
            .pointer
            .iter()
            .map(|t| format!("/{t}"))
            .collect::<String>();
        let shape = shape_value(&field.shape);
        for (name, slot) in [
            ("type", "Type"),
            ("format", "Format"),
            ("required", "Required"),
            ("enum", "Enum"),
            ("item_type", "ItemType"),
        ] {
            if matches!(shape[name].as_str(), Some("branch_dependent" | "unknown")) {
                let covered = expected.iter().any(|limit| {
                    limit["side"] == side
                        && limit["scope"] == root
                        && limit["pointer"] == pointer
                        && limit["claim"] == format!("Property({slot})")
                });
                check.eq(
                    &format!("standing {slot} limit for {side} {}", field.id),
                    json!(covered),
                    json!(true),
                );
            }
        }
    }
}
