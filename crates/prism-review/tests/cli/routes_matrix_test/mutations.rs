//! Single-field mutations shared by the listing and receipt tests.
use serde_json::{json, Value};

#[derive(Clone)]
pub(super) struct Edit {
    pub(super) pointer: String,
    pub(super) pattern: String,
    replacement: Option<Value>,
}

impl Edit {
    pub(super) fn apply(&self, value: &Value) -> Value {
        let mut changed = value.clone();
        if let Some(replacement) = &self.replacement {
            *changed.pointer_mut(&self.pointer).unwrap() = replacement.clone();
        } else {
            let (parent, key) = self.pointer.rsplit_once('/').unwrap();
            let key = key.replace("~1", "/").replace("~0", "~");
            changed
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(&key);
        }
        changed
    }
}

pub(super) fn edits(value: &Value) -> Vec<Edit> {
    fn visit(value: &Value, pointer: &str, pattern: &str, out: &mut Vec<Edit>) {
        let replacement = match value {
            Value::Object(fields) => {
                for (key, value) in fields {
                    let name = if pattern.is_empty() {
                        key.clone()
                    } else {
                        format!("{pattern}.{key}")
                    };
                    let pointer =
                        format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                    out.push(Edit {
                        pointer: pointer.clone(),
                        pattern: name.clone(),
                        replacement: None,
                    });
                    visit(value, &pointer, &name, out);
                }
                return;
            }
            Value::Array(items) => {
                for (index, value) in items.iter().enumerate() {
                    visit(
                        value,
                        &format!("{pointer}/{index}"),
                        &format!("{pattern}[]"),
                        out,
                    );
                }
                if items.is_empty() {
                    json!([null])
                } else {
                    json!([])
                }
            }
            Value::Bool(value) => json!(!value),
            Value::Number(value) => json!(value.as_i64().unwrap() + 100_000),
            Value::String(value) => json!(format!("{value}--corrupted--")),
            Value::Null => json!("--corrupted--"),
        };
        out.push(Edit {
            pointer: pointer.into(),
            pattern: pattern.into(),
            replacement: Some(replacement),
        });
    }
    let mut edits = Vec::new();
    visit(value, "", "", &mut edits);
    edits
}
