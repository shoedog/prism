use crate::revision::Blob;
use crate::schema::{servers::Uri, Site};
use serde_json::Value;
use std::cell::OnceCell;

pub(in crate::schema) struct Document<'a> {
    pub value: Value,
    pub base: Option<Uri>,
    pub(super) blob: Blob<'a>,
    schema_resolution_keywords: OnceCell<bool>,
}

impl<'a> Document<'a> {
    pub(super) fn new(blob: Blob<'a>, value: Value, base: Option<Uri>) -> Self {
        Self {
            blob,
            value,
            base,
            schema_resolution_keywords: OnceCell::new(),
        }
    }

    pub fn site(&self, pointer: Vec<String>) -> Site {
        Site::from_blob(self.blob, pointer)
    }

    pub fn has_schema_resolution_keywords(&self) -> bool {
        *self
            .schema_resolution_keywords
            .get_or_init(|| contains_member(&self.value, &["$id", "$anchor", "$dynamicRef"]))
    }
}

pub(in crate::schema) fn contains_member(value: &Value, names: &[&str]) -> bool {
    match value {
        Value::Object(fields) => {
            names.iter().any(|name| fields.contains_key(*name))
                || fields.values().any(|value| contains_member(value, names))
        }
        Value::Array(values) => values.iter().any(|value| contains_member(value, names)),
        _ => false,
    }
}
