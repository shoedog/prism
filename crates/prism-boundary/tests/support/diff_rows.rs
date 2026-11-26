use crate::diff_input::Input;
use crate::matrix::p1c_rows;
use prism_boundary::{
    diff_boundary, diff_schemas, signature, ChangeSide, ComparisonLimit, ElementId,
    ElementPresence, EnumValue, FieldShape, LimitScope, RootKey, SchemaDiff, StatusInventory,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn entries() -> Value {
    serde_json::from_str(include_str!("../fixtures/p1c_rows/expectations.json")).unwrap()
}

pub fn run(classes: &[&str]) {
    let entries = entries();
    let mut failures = Vec::new();
    for row in p1c_rows().filter(|row| classes.contains(&row.class)) {
        let entry = &entries[row.id];
        check_case(row.id, entry, false, &mut failures);
        if entry["reverse"] == true {
            check_case(row.id, entry, true, &mut failures);
        }
        for control in entry["controls"].as_array().into_iter().flatten() {
            let id = control.as_str().unwrap();
            check_case(
                &format!("{}/control/{id}", row.id),
                &entries[id],
                false,
                &mut failures,
            );
        }
    }
    assert!(
        failures.is_empty(),
        "P1c diff assertions\n{}",
        failures.join("\n")
    );
}

fn check_case(id: &str, entry: &Value, reverse: bool, failures: &mut Vec<String>) {
    let inputs = [
        Input::load(&entry["inputs"][0], 0),
        Input::load(&entry["inputs"][1], 1),
    ];
    let (before, after) = if reverse {
        (&inputs[1], &inputs[0])
    } else {
        (&inputs[0], &inputs[1])
    };
    let result = match (&before.schema, &after.schema) {
        (Some(b), Some(a)) if before.basis.is_none() && after.basis.is_none() => diff_schemas(b, a),
        _ => diff_boundary(&before.side(), &after.side()),
    };
    let id = if reverse {
        format!("{id}/reverse")
    } else {
        id.into()
    };
    let mut check = Checks { id: &id, failures };
    if let Some(error) = entry["error"].as_str() {
        check.eq(
            "basis error",
            json!(result.err().map(|error| format!("{error:?}"))),
            json!(error),
        );
        return;
    }
    let diff = match result {
        Ok(result) => result,
        Err(error) => {
            check
                .failures
                .push(format!("{id}: expected scoped result, got {error}"));
            return;
        }
    };
    check_changes(&mut check, &diff, entry, reverse);
    check_limits(&mut check, &diff, entry, reverse);
    check_pairs(&mut check, &diff, entry, reverse);
    for (i, label) in ["Before", "After"].iter().enumerate() {
        if let Some(expected) = entry["inventories"].get(label) {
            let schema = inputs[i].schema.as_ref().unwrap();
            let actual: serde_json::Map<_, _> = schema.operations[0]
                .declarations
                .iter()
                .map(|site| {
                    let inventory = match &site.status_inventory {
                        StatusInventory::Readable(keys) => sorted(json!(keys)),
                        StatusInventory::Withheld => Value::Null,
                    };
                    (site.path.clone(), inventory)
                })
                .collect();
            check.eq(
                "per-declaration status inventories",
                json!(actual),
                expected.clone(),
            );
        }
    }
}

fn presence(element: &ElementPresence) -> Value {
    json!([
        element.id.as_ref().map(element_id),
        format!("{:?}", element.presence)
    ])
}

fn element_id(id: &ElementId) -> String {
    match id {
        ElementId::Operation(op) => op.to_string(),
        ElementId::Status(op, code) => format!("status:{} {}:{code}", op.method(), op.path()),
        ElementId::Field(field) => field.to_string(),
    }
}

pub fn shape_value(shape: &FieldShape) -> Value {
    let slots: Vec<Vec<Value>> = serde_json::from_str(&signature(shape)).unwrap();
    let mut object = serde_json::Map::new();
    for (name, slot) in ["type", "format", "required", "enum", "item_type"]
        .into_iter()
        .zip(slots)
    {
        object.insert(name.into(), slot.get(1).unwrap_or(&slot[0]).clone());
    }
    Value::Object(object)
}

fn enum_value(value: &EnumValue) -> Value {
    match value {
        EnumValue::Null => Value::Null,
        EnumValue::Boolean(value) => json!(value),
        EnumValue::Number(value) => serde_json::from_str(value).unwrap(),
        EnumValue::String(value) => json!(value),
    }
}

pub fn sorted(values: Value) -> Value {
    let mut items = values.as_array().unwrap().clone();
    items.sort_by_key(Value::to_string);
    Value::Array(items)
}

fn reversed_kind(kind: &str, reverse: bool) -> &str {
    if !reverse {
        return kind;
    }
    match kind {
        "OpAdded" => "OpRemoved",
        "OpRemoved" => "OpAdded",
        "SuccessStatusAdded" => "SuccessStatusRemoved",
        "SuccessStatusRemoved" => "SuccessStatusAdded",
        "FieldAdded" => "FieldRemoved",
        "FieldRemoved" => "FieldAdded",
        "FieldRequiredAdded" => "FieldRequiredRemoved",
        "FieldRequiredRemoved" => "FieldRequiredAdded",
        "EnumValuesAdded" => "EnumValuesRemoved",
        "EnumValuesRemoved" => "EnumValuesAdded",
        kind => kind,
    }
}

fn side_labels(reverse: bool) -> [&'static str; 2] {
    if reverse {
        ["after", "before"]
    } else {
        ["before", "after"]
    }
}

fn check_changes(check: &mut Checks<'_>, diff: &SchemaDiff<'_>, entry: &Value, reverse: bool) {
    let [before, after] = side_labels(reverse);
    let expected = entry["changes"].as_array().unwrap();
    let kind_set = |rows: Vec<Vec<String>>| rows.into_iter().flatten().collect::<BTreeSet<_>>();
    let expected_kinds = |row: &Value| {
        sorted(json!(row["kinds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|kind| reversed_kind(kind.as_str().unwrap(), reverse))
            .collect::<Vec<_>>()))
    };
    let actual_kinds = diff
        .changes
        .iter()
        .map(|row| {
            row.kinds
                .iter()
                .map(|kind| format!("{kind:?}"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    check.eq(
        "ChangeKind set",
        json!(kind_set(actual_kinds)),
        json!(kind_set(
            expected
                .iter()
                .map(|row| serde_json::from_value(expected_kinds(row)).unwrap())
                .collect()
        )),
    );
    check.eq(
        "element change count",
        json!(diff.changes.len()),
        json!(expected.len()),
    );
    for row in expected {
        let target = json!([row[before], row[after], expected_kinds(row)]);
        let matches: Vec<_> = diff
            .changes
            .iter()
            .filter(|change| {
                let kinds = sorted(json!(change
                    .kinds
                    .iter()
                    .map(|kind| format!("{kind:?}"))
                    .collect::<Vec<_>>()));
                json!([
                    presence(&change.before.element),
                    presence(&change.after.element),
                    kinds
                ]) == target
            })
            .collect();
        if matches.len() != 1 {
            check.failures.push(format!(
                "{}: expected one side-local record {target}, found {}",
                check.id,
                matches.len()
            ));
        }
        for change in matches {
            let endpoint = [&change.before.element, &change.after.element]
                .into_iter()
                .find_map(|side| side.id.as_ref())
                .unwrap();
            let expected_owner = match endpoint {
                ElementId::Operation(op) | ElementId::Status(op, _) => op,
                ElementId::Field(field) => match &field.root {
                    prism_boundary::element::MessageRoot::Request(op)
                    | prism_boundary::element::MessageRoot::Response(op, _) => op,
                },
            };
            check.eq(
                "owning operation",
                json!(change.operation.to_string()),
                json!(expected_owner.to_string()),
            );
            check_side(check, &change.before, row, before);
            check_side(check, &change.after, row, after);
            let enum_expectation = if reverse {
                ["enum_removed", "enum_added"]
            } else {
                ["enum_added", "enum_removed"]
            };
            for (key, actual) in enum_expectation
                .into_iter()
                .zip([&change.enum_added, &change.enum_removed])
            {
                check.eq(
                    key,
                    sorted(json!(actual.iter().map(enum_value).collect::<Vec<_>>())),
                    sorted(row.get(key).cloned().unwrap_or(json!([]))),
                );
            }
        }
    }
}

fn check_side(check: &mut Checks<'_>, side: &ChangeSide<'_>, row: &Value, label: &str) {
    let declaration = format!("{label}_declaration");
    if let Some(expected) = row.get(&declaration) {
        check.eq(
            &declaration,
            json!(side.declaration_id.as_ref().map(ToString::to_string)),
            expected.clone(),
        );
    }
    let slots = format!("{label}_slots");
    if let Some(expected) = row[&slots].as_object() {
        let actual = side.shape.as_ref().map(shape_value).unwrap_or_default();
        for (slot, value) in expected {
            check.eq(
                &format!("{slots}/{slot}"),
                actual[slot].clone(),
                value.clone(),
            );
        }
    }
    let details = format!("{label}_fields");
    if let Some(expected) = row.get(&details) {
        let actual = side
            .fields
            .iter()
            .map(|field| json!([field.id.to_string(), format!("{:?}", field.presence)]))
            .collect::<Vec<_>>();
        check.eq(&details, sorted(json!(actual)), sorted(expected.clone()));
    }
    let statuses = format!("{label}_statuses");
    if let Some(expected) = row.get(&statuses) {
        let actual = side
            .success_keys
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        check.eq(&statuses, sorted(json!(actual)), sorted(expected.clone()));
    }
    let tree = format!("{label}-tree");
    if let Some(pointers) = row[format!("{label}_site_pointers")].as_array() {
        let actual = side
            .sites
            .iter()
            .map(|site| json!(site.pointer))
            .collect::<Vec<_>>();
        for pointer in pointers {
            check.eq(
                "original declaration site",
                json!(actual.contains(pointer)),
                json!(true),
            );
        }
    }
    if row["producer_evidence"] == true {
        let actual = side.producer.map(|producer| {
            [
                &producer.declaration.sites,
                &producer.validation.sites,
                &producer.reference.sites,
                &producer.code.sites,
            ]
            .into_iter()
            .all(|sites| !sites.is_empty() && sites.iter().all(|site| site.tree == tree))
        });
        check.eq("side-local producer evidence", json!(actual), json!(true));
    } else if side.element.id.is_some() || side.declaration_id.is_some() {
        check.eq(
            "side-local schema sites",
            json!(!side.sites.is_empty() && side.sites.iter().all(|site| site.tree == tree)),
            json!(true),
        );
    }
}

fn limit_value(limit: &ComparisonLimit) -> Value {
    let scope = match &limit.scope {
        LimitScope::Schema => "schema".into(),
        LimitScope::Operation(Some(op)) => op.to_string(),
        LimitScope::Operation(None) => "unprojected".into(),
        LimitScope::Status(op, None) => format!("statuses:{} {}", op.method(), op.path()),
        LimitScope::Status(op, Some(code)) => {
            format!("status:{} {}:{code}", op.method(), op.path())
        }
        LimitScope::Root(op, RootKey::Request) => format!("root:req:{} {}", op.method(), op.path()),
        LimitScope::Root(op, RootKey::Response(code)) => {
            format!("root:resp:{} {}:{code}", op.method(), op.path())
        }
        LimitScope::Root(op, RootKey::Fallback) => {
            format!("root:resp:{} {}:default", op.method(), op.path())
        }
    };
    json!({
        "side": format!("{:?}", limit.side),
        "scope": scope,
        "claim": format!("{:?}", limit.claim),
        "reason": limit.reason,
        "pointer": limit.pointer.iter().map(|token| format!("/{token}")).collect::<String>(),
        "presence": limit.presence.as_ref().map(|sides| vec![presence(&sides.before), presence(&sides.after)]),
    })
}

fn check_limits(check: &mut Checks<'_>, diff: &SchemaDiff<'_>, entry: &Value, reverse: bool) {
    let actual = diff.limits.iter().map(limit_value).collect::<Vec<_>>();
    let counts =
        ["Before", "After"].map(|side| actual.iter().filter(|value| value["side"] == side).count());
    check.eq(
        "per-side limit totals",
        json!([diff.limit_counts.before, diff.limit_counts.after]),
        json!(counts),
    );
    let mut unique = BTreeSet::new();
    for limit in &actual {
        let key = json!([
            limit["side"],
            limit["scope"],
            limit["pointer"],
            limit["claim"],
            limit["reason"]
        ])
        .to_string();
        check.eq(
            "unique limit per side, scope, pointer, claim and reason",
            json!(unique.insert(key)),
            json!(true),
        );
    }
    let expected = entry["limits"].as_array().unwrap();
    if entry["subset_limits"] != true {
        check.eq("limit count", json!(actual.len()), json!(expected.len()));
    }
    for pointer in entry["forbidden_limit_pointers"]
        .as_array()
        .into_iter()
        .flatten()
    {
        check.eq(
            "independent pointer stays comparable",
            json!(actual.iter().any(|limit| limit["pointer"] == *pointer)),
            json!(false),
        );
    }
    for expected in expected {
        let mut wanted = expected.as_object().unwrap().clone();
        if reverse {
            wanted.insert(
                "side".into(),
                json!(if expected["side"] == "Before" {
                    "After"
                } else {
                    "Before"
                }),
            );
            if let Some(sides) = wanted.get_mut("presence") {
                sides.as_array_mut().unwrap().reverse();
            }
        }
        let matches: Vec<_> = actual
            .iter()
            .zip(&diff.limits)
            .filter(|(value, _)| wanted.iter().all(|(key, expected)| value[key] == *expected))
            .collect();
        if matches.len() != 1 {
            check.failures.push(format!(
                "{}: expected one comparison limit {:?}, got {:?}",
                check.id, wanted, actual
            ));
        } else {
            let (_, actual) = matches[0];
            if wanted["scope"] != "schema" {
                let original_side = expected["side"].as_str().unwrap().to_lowercase();
                let tree = format!("{original_side}-tree");
                check.eq(
                    "limit retains side-local evidence",
                    json!(
                        (!actual.evidence.is_empty() || !actual.producer_sites.is_empty())
                            && actual.evidence.iter().all(|site| site.tree == tree)
                            && actual.producer_sites.iter().all(|site| site.tree == tree)
                    ),
                    json!(true),
                );
            }
        }
    }
}

fn check_pairs(check: &mut Checks<'_>, diff: &SchemaDiff<'_>, entry: &Value, reverse: bool) {
    let expected = entry["pairs"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    check.eq(
        "response-root pair count",
        json!(diff.response_root_pairs.len()),
        json!(expected.len()),
    );
    let [before, after] = side_labels(reverse);
    for pair in expected {
        let matching = diff.response_root_pairs.iter().find(|actual| {
            pair["operation"] == actual.operation.to_string()
                && pair[before] == actual.before.to_string()
                && pair[after] == actual.after.to_string()
        });
        let Some(actual) = matching else {
            check
                .failures
                .push(format!("{}: missing response-root pair {pair}", check.id));
            continue;
        };
        let actual_fields = actual
            .fields
            .iter()
            .map(|field| json!([field.before.to_string(), field.after.to_string()]))
            .collect::<Vec<_>>();
        let mut expected_fields = pair["fields"].as_array().unwrap().clone();
        if reverse {
            for ids in &mut expected_fields {
                ids.as_array_mut().unwrap().reverse();
            }
        }
        if entry["pair_fields_subset"] == true {
            for pair in expected_fields {
                check.eq(
                    "paired fields preserve both original IDs",
                    json!(actual_fields.contains(&pair)),
                    json!(true),
                );
            }
        } else {
            check.eq(
                "paired fields preserve both original IDs",
                sorted(json!(actual_fields)),
                sorted(json!(expected_fields)),
            );
        }
    }
}

pub struct Checks<'a> {
    pub id: &'a str,
    pub failures: &'a mut Vec<String>,
}

impl Checks<'_> {
    pub fn eq(&mut self, claim: &str, actual: Value, expected: Value) {
        if actual != expected {
            self.failures.push(format!(
                "{}: {claim}: expected {expected}, got {actual}",
                self.id
            ));
        }
    }
}
