use crate::matrix::{matrix_path, p1d_rows};
use prism_boundary::revision::{AdmittedSchemaInput, Blob, Format, Snapshot};
use prism_boundary::{index_schema, SchemaIndex};
use serde_json::{json, Value};
use std::fs;

pub const API: &str = "pilot-api";

pub fn run(scenarios: &[&str], check: fn(&Case<'_>) -> Result<(), String>) {
    let mut entries = serde_json::Map::new();
    for shard in [
        include_str!("../fixtures/p1d_rows/expected-admission.json"),
        include_str!("../fixtures/p1d_rows/expectations.json"),
    ] {
        let records: serde_json::Map<String, Value> =
            serde_json::from_str(shard).expect("P1d row expectations");
        for (id, record) in records {
            assert!(
                entries.insert(id.clone(), record).is_none(),
                "duplicate row {id}"
            );
        }
    }
    let mut failures = Vec::new();
    let mut count = 0;
    for row in p1d_rows() {
        let entry = entries.get(row.id).expect("every TSV row has expectations");
        let scenario = entry["scenario"].as_str().expect("row scenario");
        if !scenarios.contains(&scenario) {
            continue;
        }

        let cases = json_fixture(
            entry["variants_fixture"]
                .as_str()
                .expect("case fixture path"),
        );
        let support = cases.get(row.id).expect("case fixture row");
        let variants = support["variants"].as_array().expect("row variants");
        for variant in variants {
            let label = variant["name"].as_str().unwrap_or("default");
            let fixture = variant["fixture"].as_str().unwrap_or_else(|| {
                entry["inputs"][0]["fixture"]
                    .as_str()
                    .expect("primary fixture")
            });
            let case = Case {
                id: row.id,
                scenario,
                fixture,
                data: variant,
                entry,
                base_patches: &support["base_patches"],
            };
            count += 1;
            if let Err(reason) = check(&case) {
                failures.push(format!("RED\t{}\t{label}\t{reason}", row.id));
            }
        }
    }
    assert!(count > 0, "matrix selected zero cases");
    assert!(
        failures.is_empty(),
        "{} of {count} cases failed\n{}",
        failures.len(),
        failures.join("\n"),
    );
}

pub struct Case<'a> {
    pub id: &'a str,
    pub scenario: &'a str,
    pub fixture: &'a str,
    pub data: &'a Value,
    pub entry: &'a Value,
    base_patches: &'a Value,
}

impl Case<'_> {
    pub fn baseline(&self) -> Value {
        json_fixture(self.fixture)
    }

    pub fn input(&self) -> Value {
        let mut value = self.baseline();
        apply_patches(&mut value, self.base_patches);
        apply_patches(&mut value, &self.data["patches"]);
        value
    }

    pub fn flag(&self, name: &str) -> bool {
        self.data[name] == true
    }

    pub fn expect(&self, actual: &Value) -> Result<(), String> {
        expect_paths(actual, &self.data["expect"])
    }

    pub fn pinned(&self, file: &str) -> Option<Value> {
        let suffix = self.entry["pinned"].as_str()?;
        let path = format!("fixtures/control-document-{suffix}/{file}");
        Some(serde_json::from_slice(&fs::read(matrix_path("ids_matrix", &path)).unwrap()).unwrap())
    }
}

pub fn expect_paths(actual: &Value, expected: &Value) -> Result<(), String> {
    let failures: Vec<_> = expected
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(path, expected)| {
            let seen = actual.pointer(path);
            let expected_value = (expected != &json!({"$missing":true})).then_some(expected);
            (seen != expected_value)
                .then(|| format!("{path} expected {expected_value:?}, got {seen:?}"))
        })
        .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

pub fn apply_patches(value: &mut Value, patches: &Value) {
    for operation in patches.as_array().into_iter().flatten() {
        let path = operation["path"].as_str().expect("patch path");
        let (parent, key) = path.rsplit_once('/').expect("JSON pointer");
        let parent = value
            .pointer_mut(parent)
            .unwrap_or_else(|| panic!("patch parent for {path}"));
        let key = key.replace("~1", "/").replace("~0", "~");
        match parent {
            Value::Object(object) => {
                if let Some(new) = operation.get("value") {
                    object.insert(key, new.clone());
                } else {
                    assert!(object.remove(&key).is_some(), "{path}");
                }
            }
            Value::Array(items) if key == "-" => {
                items.push(operation["value"].clone());
            }
            Value::Array(items) => {
                let index: usize = key.parse().expect("patch array index");
                if let Some(new) = operation.get("value") {
                    items[index] = new.clone();
                } else {
                    items.remove(index);
                }
            }
            _ => panic!("patch target at {path} is a scalar"),
        }
    }
}

pub fn fixture(name: &str) -> Vec<u8> {
    if name == "formatted" {
        return serde_json::to_vec(&json_fixture("inventory.json")).unwrap();
    }
    let (matrix, name) = name
        .strip_prefix("p1c:")
        .map_or(("p1d_rows", name), |p| ("p1c_rows", p));
    let path = matrix_path(matrix, &format!("fixtures/{name}"));
    fs::read(&path).unwrap_or_else(|error| panic!("fixture {}: {error}", path.display()))
}

pub fn json_fixture(name: &str) -> Value {
    serde_json::from_slice(&fixture(name)).unwrap_or_else(|error| panic!("fixture {name}: {error}"))
}

pub fn schema(bytes: &[u8], id: &str) -> SchemaIndex {
    let tree = format!("{id}-tree");
    let commit = format!("{id}-rev");
    let input = AdmittedSchemaInput {
        api: API,
        authority: "schema",
        snapshot: Snapshot::Commit {
            tree: &tree,
            commit: &commit,
            subject: None,
            committer_date: None,
        },
        entry: Blob {
            path: "openapi.json",
            tree: &tree,
            oid: "schema-blob",
            bytes,
            format: Format::Json,
        },
        retrieval_uri: None,
        references: &[],
        config_receipts: &[],
        media_receipts: &[],
    };
    index_schema(&input).expect("supported schema fixture")
}
