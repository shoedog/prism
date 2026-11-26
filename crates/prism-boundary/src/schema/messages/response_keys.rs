//! Inspect raw object members before the ordinary JSON map loses duplicate keys.

use super::super::references::Located;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

type Members<'a> = Vec<(String, &'a RawValue)>;

struct RawObject<'a>(Members<'a>);

impl<'de> Deserialize<'de> for RawObject<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Object;
        impl<'de> Visitor<'de> for Object {
            type Value = RawObject<'de>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut members = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    members.push(pair);
                }
                Ok(RawObject(members))
            }
        }
        deserializer.deserialize_map(Object)
    }
}

type Duplicates = BTreeMap<Vec<String>, BTreeSet<String>>;

pub(super) struct DuplicateKeys {
    by_document: RefCell<BTreeMap<usize, Duplicates>>,
}

impl DuplicateKeys {
    pub fn new() -> Self {
        Self {
            by_document: RefCell::new(BTreeMap::new()),
        }
    }

    pub fn contains(&self, operation: &Located<'_, '_>, key: &str) -> bool {
        let id = std::ptr::from_ref(operation.document) as usize;
        let mut documents = self.by_document.borrow_mut();
        let duplicates = documents.entry(id).or_insert_with(|| {
            let mut duplicates = BTreeMap::new();
            collect(
                operation.document.blob.bytes,
                &mut Vec::new(),
                &mut duplicates,
            );
            duplicates
        });
        duplicates
            .get(&operation.site.child("responses").pointer)
            .is_some_and(|keys| keys.contains(key))
    }
}

fn collect(bytes: &[u8], path: &mut Vec<String>, duplicates: &mut Duplicates) {
    match bytes
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
    {
        Some(b'{') => {
            let Ok(RawObject(members)) = serde_json::from_slice(bytes) else {
                return;
            };
            if path.last().is_some_and(|key| key == "responses") {
                let mut seen = BTreeSet::new();
                for (key, _) in &members {
                    if !seen.insert(key) {
                        duplicates
                            .entry(path.clone())
                            .or_default()
                            .insert(key.clone());
                    }
                }
            }
            for (key, raw) in members {
                path.push(key);
                collect(raw.get().as_bytes(), path, duplicates);
                path.pop();
            }
        }
        Some(b'[') => {
            let Ok(values) = serde_json::from_slice::<Vec<&RawValue>>(bytes) else {
                return;
            };
            for (i, raw) in values.into_iter().enumerate() {
                path.push(i.to_string());
                collect(raw.get().as_bytes(), path, duplicates);
                path.pop();
            }
        }
        _ => {}
    }
}
