use serde_json::Value;

/// Small in-repo Draft 2020-12 structural checker. It deliberately implements
/// only the schema features the targets contract uses: required/property
/// closure, const/enum, arrays, integer minima, the id/hash pattern, and local
/// `$ref` resolution into `$defs`.
pub(super) fn check_against_schema(doc: &Value, schema: &Value) {
    fn resolve<'a>(root: &'a Value, node: &'a Value) -> &'a Value {
        match node.get("$ref").and_then(Value::as_str) {
            Some(reference) => root
                .pointer(reference.strip_prefix('#').expect("local $ref"))
                .unwrap_or_else(|| panic!("unresolved schema ref {reference}")),
            None => node,
        }
    }

    fn walk(value: &Value, raw_schema: &Value, root: &Value, path: &str) {
        let schema = resolve(root, raw_schema);
        if let Some(expected) = schema.get("const") {
            assert_eq!(value, expected, "const mismatch at {path}");
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            assert!(values.contains(value), "enum mismatch at {path}: {value}");
        }
        if schema.get("pattern").and_then(Value::as_str) == Some("^[0-9a-f]{64}$") {
            let text = value
                .as_str()
                .unwrap_or_else(|| panic!("non-string hash at {path}"));
            assert_eq!(text.len(), 64, "hash length at {path}");
            assert!(
                text.bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                "hash alphabet at {path}: {text}"
            );
        }

        match schema.get("type").and_then(Value::as_str) {
            Some("object") => {
                let object = value
                    .as_object()
                    .unwrap_or_else(|| panic!("non-object at {path}"));
                let properties = schema["properties"].as_object().unwrap();
                for key in schema["required"].as_array().into_iter().flatten() {
                    let key = key.as_str().unwrap();
                    assert!(object.contains_key(key), "missing {path}.{key}");
                }
                if schema["additionalProperties"] == false {
                    for key in object.keys() {
                        assert!(properties.contains_key(key), "unknown {path}.{key}");
                    }
                }
                for (key, child) in object {
                    if let Some(child_schema) = properties.get(key) {
                        walk(child, child_schema, root, &format!("{path}.{key}"));
                    }
                }
            }
            Some("array") => {
                let array = value
                    .as_array()
                    .unwrap_or_else(|| panic!("non-array at {path}"));
                if let Some(item_schema) = schema.get("items") {
                    for (index, item) in array.iter().enumerate() {
                        walk(item, item_schema, root, &format!("{path}[{index}]"));
                    }
                }
            }
            Some("string") => assert!(value.is_string(), "non-string at {path}"),
            Some("integer") => {
                let integer = value
                    .as_u64()
                    .unwrap_or_else(|| panic!("non-integer at {path}"));
                if let Some(minimum) = schema.get("minimum").and_then(Value::as_u64) {
                    assert!(integer >= minimum, "{path} below minimum {minimum}");
                }
            }
            Some(other) => panic!("unsupported schema type {other} at {path}"),
            None => {}
        }
    }

    walk(doc, schema, schema, "$");
}

pub(super) fn contract_schema(filename: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts")
        .join(filename);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
