use crate::revision::Blob;
use crate::schema::{servers::Uri, Site};
use serde_json::Value;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::schema) struct Document<'a> {
    pub value: Value,
    pub base: Option<Uri>,
    pub(in crate::schema) blob: Blob<'a>,
    pub readable: bool,
    duplicates: BTreeMap<Vec<String>, BTreeSet<String>>,
    schema_resolution_or_dialect_keywords: OnceCell<bool>,
}

impl<'a> Document<'a> {
    pub(super) fn new(blob: Blob<'a>, parsed: Parsed, base: Option<Uri>) -> Self {
        Self {
            blob,
            readable: true,
            value: parsed.value,
            duplicates: parsed.duplicates,
            base,
            schema_resolution_or_dialect_keywords: OnceCell::new(),
        }
    }

    pub fn site(&self, pointer: Vec<String>) -> Site {
        Site::from_blob(self.blob, pointer)
    }

    pub fn duplicate(&self, pointer: &[String], member: &str) -> bool {
        self.duplicates
            .get(pointer)
            .is_some_and(|keys| keys.contains(member))
    }

    pub fn has_duplicates(&self, pointer: &[String]) -> bool {
        self.duplicates.contains_key(pointer)
    }

    pub fn duplicates_under(&self, pointer: &[String]) -> bool {
        self.duplicates
            .range(pointer.to_vec()..)
            .next()
            .is_some_and(|(path, _)| path.starts_with(pointer))
    }

    pub fn ambiguous_path(&self, pointer: &[String]) -> bool {
        !self.duplicates.is_empty()
            && pointer
                .iter()
                .enumerate()
                .any(|(i, member)| self.duplicate(&pointer[..i], member))
    }

    pub fn has_schema_resolution_or_dialect_keywords(&self) -> bool {
        *self.schema_resolution_or_dialect_keywords.get_or_init(|| {
            self.value.get("jsonSchemaDialect").is_some()
                || contains_member(&self.value, &["$id", "$anchor", "$dynamicRef", "$schema"])
        })
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
use super::raw::Parsed;
