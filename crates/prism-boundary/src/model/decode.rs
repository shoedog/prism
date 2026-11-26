use super::DocumentError;
use crate::schema::parse_json;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub(super) enum Kind {
    Bindings,
    Impact,
}

const BINDINGS: &str = "urn:prism:boundary:bindings:0.1";
static SCHEMAS: LazyLock<Contracts> = LazyLock::new(Contracts::load);

pub(super) fn document<T: DeserializeOwned>(bytes: &[u8], kind: Kind) -> Result<T, DocumentError> {
    let parsed = parse_json(bytes).map_err(|_| {
        let looks_like_yaml = std::str::from_utf8(bytes).is_ok_and(|text| {
            let text = text.trim_start();
            text.starts_with("---")
                || text
                    .lines()
                    .next()
                    .and_then(|line| line.split_once(':'))
                    .is_some_and(|(key, _)| {
                        !key.is_empty()
                            && key
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                    })
        });
        if looks_like_yaml {
            DocumentError::UnsupportedFormat
        } else {
            DocumentError::InvalidDocument
        }
    })?;
    if !parsed.duplicates.is_empty() {
        return Err(DocumentError::DuplicateJsonMember);
    }
    let mut value = parsed.value;
    normalize_numbers(&mut value);
    if !value.is_object() {
        return Err(DocumentError::InvalidDocument);
    }
    if value.get("schema_version").is_none() {
        return Err(DocumentError::InvalidDocument);
    }
    if value["schema_version"] != "0.1" {
        return Err(DocumentError::UnsupportedVersion);
    }
    match kind {
        Kind::Bindings if value.get("report_id").is_some() => {
            return Err(DocumentError::WrongDocumentKind)
        }
        Kind::Impact if value.get("report_id").is_none() && value.get("input_id").is_some() => {
            return Err(DocumentError::WrongDocumentKind);
        }
        _ => {}
    }
    let schema = match kind {
        Kind::Bindings => &SCHEMAS.bindings,
        Kind::Impact => &SCHEMAS.impact,
    };
    if !SCHEMAS.check(&value, schema, schema) {
        return Err(DocumentError::InvalidDocument);
    }
    serde_json::from_value(value).map_err(|_| DocumentError::InvalidDocument)
}

struct Contracts {
    bindings: Value,
    impact: Value,
    patterns: BTreeMap<String, Regex>,
}

impl Contracts {
    fn load() -> Self {
        let bindings =
            serde_json::from_str(include_str!("../../schemas/boundary-bindings.schema.json"))
                .expect("bundled bindings contract");
        let impact =
            serde_json::from_str(include_str!("../../schemas/boundary-impact.schema.json"))
                .expect("bundled impact contract");
        let mut patterns = BTreeMap::new();
        for schema in [&bindings, &impact] {
            Self::read_patterns(schema, &mut patterns);
        }
        Self {
            bindings,
            impact,
            patterns,
        }
    }

    fn read_patterns(schema: &Value, patterns: &mut BTreeMap<String, Regex>) {
        if let Some(pattern) = schema["pattern"].as_str() {
            patterns
                .entry(pattern.into())
                .or_insert_with(|| Regex::new(pattern).expect("contract regex"));
        }
        if let Some(object) = schema.as_object() {
            for child in object.values() {
                Self::read_patterns(child, patterns);
            }
        } else if let Some(array) = schema.as_array() {
            for child in array {
                Self::read_patterns(child, patterns);
            }
        }
    }

    // This interpreter is for the bundled contracts. Unknown keywords fail closed.
    fn check(&self, value: &Value, node: &Value, root: &Value) -> bool {
        if let Some(flag) = node.as_bool() {
            return flag;
        }
        node.as_object().is_some_and(|keywords| {
            keywords
                .iter()
                .all(|(keyword, rule)| match keyword.as_str() {
                    "$schema" | "$id" | "$defs" | "title" | "then" | "else" => true,
                    "$ref" => rule.as_str().is_some_and(|reference| {
                        reference.split_once('#').is_some_and(|(uri, path)| {
                            let target = match uri {
                                "" => root,
                                BINDINGS => &self.bindings,
                                _ => return false,
                            };
                            target
                                .pointer(path)
                                .is_some_and(|node| self.check(value, node, target))
                        })
                    }),
                    "type" => {
                        rule.as_str().is_some_and(|kind| has_type(value, kind))
                            || rule.as_array().is_some_and(|types| {
                                types.iter().any(|kind| {
                                    kind.as_str().is_some_and(|kind| has_type(value, kind))
                                })
                            })
                    }
                    "const" => value == rule,
                    "enum" => rule
                        .as_array()
                        .is_some_and(|choices| choices.contains(value)),
                    "format" => {
                        rule == "date-time"
                            && value
                                .as_str()
                                .is_some_and(|s| crate::revision::instant(s).is_some())
                    }
                    "pattern" => rule
                        .as_str()
                        .and_then(|pattern| self.patterns.get(pattern))
                        .zip(value.as_str())
                        .is_some_and(|(pattern, text)| pattern.is_match(text)),
                    "minimum" => value
                        .as_u64()
                        .zip(rule.as_u64())
                        .is_some_and(|(value, floor)| value >= floor),
                    "minLength" => value
                        .as_str()
                        .zip(rule.as_u64())
                        .is_some_and(|(text, n)| text.chars().count() as u64 >= n),
                    "minItems" => value
                        .as_array()
                        .zip(rule.as_u64())
                        .is_some_and(|(items, n)| items.len() as u64 >= n),
                    "maxItems" => value
                        .as_array()
                        .zip(rule.as_u64())
                        .is_some_and(|(items, n)| items.len() as u64 <= n),
                    "uniqueItems" => {
                        rule == false
                            || value.as_array().is_some_and(|items| {
                                let mut seen = std::collections::BTreeSet::new();
                                items.iter().all(|item| {
                                    let normalized = item.as_number().map(|number| {
                                        crate::schema::signature::canonical_number(
                                            &number.to_string(),
                                        )
                                    });
                                    seen.insert(normalized.unwrap_or_else(|| item.to_string()))
                                })
                            })
                    }
                    "items" => value
                        .as_array()
                        .is_some_and(|items| items.iter().all(|item| self.check(item, rule, root))),
                    "prefixItems" => {
                        value
                            .as_array()
                            .zip(rule.as_array())
                            .is_some_and(|(items, schemas)| {
                                items
                                    .iter()
                                    .zip(schemas)
                                    .all(|(item, schema)| self.check(item, schema, root))
                            })
                    }
                    "required" => rule.as_array().is_some_and(|keys| {
                        keys.iter()
                            .all(|key| key.as_str().is_some_and(|key| value.get(key).is_some()))
                    }),
                    "properties" => rule.as_object().is_some_and(|props| {
                        props.iter().all(|(key, schema)| {
                            value
                                .get(key)
                                .is_none_or(|item| self.check(item, schema, root))
                        })
                    }),
                    "propertyNames" => value.as_object().is_some_and(|object| {
                        object
                            .keys()
                            .all(|key| self.check(&Value::String(key.clone()), rule, root))
                    }),
                    "additionalProperties" => value.as_object().is_some_and(|object| {
                        object.iter().all(|(key, field)| {
                            node["properties"].get(key).is_some() || self.check(field, rule, root)
                        })
                    }),
                    "allOf" => rule.as_array().is_some_and(|nodes| {
                        nodes.iter().all(|schema| self.check(value, schema, root))
                    }),
                    "anyOf" => rule.as_array().is_some_and(|nodes| {
                        nodes.iter().any(|schema| self.check(value, schema, root))
                    }),
                    "oneOf" => rule.as_array().is_some_and(|nodes| {
                        nodes
                            .iter()
                            .filter(|schema| self.check(value, schema, root))
                            .take(2)
                            .count()
                            == 1
                    }),
                    "not" => !self.check(value, rule, root),
                    "if" => {
                        let branch = if self.check(value, rule, root) {
                            "then"
                        } else {
                            "else"
                        };
                        node.get(branch)
                            .is_none_or(|schema| self.check(value, schema, root))
                    }
                    _ => false,
                })
        })
    }
}

fn has_type(value: &Value, kind: &str) -> bool {
    match kind {
        "array" => value.is_array(),
        "boolean" => value.is_boolean(),
        "integer" => value.as_u64().is_some() || value.as_i64().is_some(),
        "null" => value.is_null(),
        "number" => value.is_number(),
        "object" => value.is_object(),
        "string" => value.is_string(),
        _ => false,
    }
}

// Normalize exact integers before schema checks and DTO decoding. Leave fractional
// and out-of-range numbers intact for fields that allow arbitrary JSON numbers.
fn normalize_numbers(value: &mut Value) {
    match value {
        Value::Number(number) if number.as_u64().is_none() && number.as_i64().is_none() => {
            let canonical = crate::schema::signature::canonical_number(&number.to_string());
            let (coefficient, exponent) = canonical.split_once('e').unwrap_or((&canonical, "0"));
            let Some(multiplier) = exponent
                .parse::<u32>()
                .ok()
                .and_then(|n| 10i128.checked_pow(n))
            else {
                return;
            };
            let Some(integer) = coefficient
                .parse::<i128>()
                .ok()
                .and_then(|n| n.checked_mul(multiplier))
            else {
                return;
            };
            if let Ok(n) = u64::try_from(integer) {
                *number = n.into();
            } else if let Ok(n) = i64::try_from(integer) {
                *number = n.into();
            }
        }
        Value::Array(items) => items.iter_mut().for_each(normalize_numbers),
        Value::Object(object) => object.values_mut().for_each(normalize_numbers),
        _ => {}
    }
}
