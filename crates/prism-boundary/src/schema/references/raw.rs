//! Decode JSON members before map insertion and preserve the kind of each token.

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{value::RawValue, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Default)]
pub(crate) struct Parsed {
    pub value: Value,
    pub duplicates: BTreeMap<Vec<String>, BTreeSet<String>>,
}

pub(crate) fn parse(bytes: &[u8]) -> serde_json::Result<Parsed> {
    let raw = serde_json::from_slice::<&RawValue>(bytes)?;
    let mut parsed = Parsed::default();
    parsed.value = read(raw, &mut Vec::new(), &mut parsed)?;
    Ok(parsed)
}

struct Members<'a>(Vec<(String, &'a RawValue)>);

impl<'de> Deserialize<'de> for Members<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Object;

        impl<'de> Visitor<'de> for Object {
            type Value = Members<'de>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut members = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    members.push(pair);
                }
                Ok(Members(members))
            }
        }

        deserializer.deserialize_map(Object)
    }
}

fn read(raw: &RawValue, path: &mut Vec<String>, parsed: &mut Parsed) -> serde_json::Result<Value> {
    let text = raw.get();
    // RawValue skips serde_json's ordinary Value recursion check.
    if path.len() >= 127 && text.starts_with(['{', '[']) {
        return Err(serde::de::Error::custom("recursion limit exceeded"));
    }
    if text.starts_with('{') {
        let Members(members) = serde_json::from_str(text)?;
        let mut object = Map::new();
        for (key, raw) in members {
            if object.contains_key(&key) {
                parsed
                    .duplicates
                    .entry(path.clone())
                    .or_default()
                    .insert(key.clone());
            }
            path.push(key.clone());
            let value = read(raw, path, parsed)?;
            path.pop();
            object.insert(key, value);
        }
        Ok(Value::Object(object))
    } else if text.starts_with('[') {
        let items: Vec<&RawValue> = serde_json::from_str(text)?;
        let mut array = Vec::with_capacity(items.len());
        for (index, raw) in items.into_iter().enumerate() {
            path.push(index.to_string());
            array.push(read(raw, path, parsed)?);
            path.pop();
        }
        Ok(Value::Array(array))
    } else {
        serde_json::from_str(text)
    }
}
