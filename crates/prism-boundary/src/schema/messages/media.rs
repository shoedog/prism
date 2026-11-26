use super::super::references::Located;
use super::super::Dialect;
use crate::element::{OperationId, PointerToken, SuccessKey};
use crate::inventory::{Coverage, JsonMediaSource, RegionLimit};
use crate::revision::AdmittedSchemaInput;
use serde_json::Value;

pub(super) struct Message<'r, 'a> {
    pub schema: Option<Located<'r, 'a>>,
    pub coverage: Coverage,
    pub limits: Vec<RegionLimit>,
    pub declared_media: Vec<String>,
    pub json_source: Option<JsonMediaSource>,
    pub blocks: Vec<Vec<PointerToken>>,
}

impl Message<'_, '_> {
    pub fn empty() -> Self {
        Self {
            schema: None,
            coverage: Coverage::Checked,
            limits: Vec::new(),
            declared_media: Vec::new(),
            json_source: None,
            blocks: Vec::new(),
        }
    }

    pub fn withheld(reason: &str) -> Self {
        let mut message = Self::empty();
        message.coverage = Coverage::Withheld;
        message.limit(Vec::new(), reason);
        message
    }

    pub fn limit(&mut self, pointer: Vec<PointerToken>, reason: &str) {
        self.limits.push(RegionLimit {
            pointer,
            reason: reason.into(),
        });
    }

    fn partial(&mut self, pointer: Vec<PointerToken>, reason: &str, block: bool) {
        self.coverage = Coverage::Partial;
        if block {
            self.blocks.push(pointer.clone());
        }
        self.limit(pointer, reason);
    }
}

pub(super) fn openapi<'r, 'a>(
    location: Located<'r, 'a>,
    request: bool,
    dialect: Dialect,
) -> Message<'r, 'a> {
    let Some(content) = location.child("content") else {
        return if request {
            Message::withheld("request_body_unreadable")
        } else {
            Message::empty()
        };
    };
    let Some(entries) = content
        .value
        .as_object()
        .filter(|entries| !entries.is_empty())
    else {
        return Message::withheld(if request {
            "request_body_unreadable"
        } else {
            "media_unreadable"
        });
    };
    let mut message = Message::empty();
    message.declared_media = entries.keys().cloned().collect();
    let matches: Vec<_> = entries.keys().filter(|key| json_media(key)).collect();
    let refusal = match matches.as_slice() {
        [] if entries
            .keys()
            .any(|key| matches!(media_type(key).as_str(), "*/*" | "application/*")) =>
        {
            Some("json_media_range_only")
        }
        [] => Some("non_json_media"),
        [key] if key.contains(';') => Some("json_media_variant_unchecked"),
        _ if matches.len() > 1 && matches.iter().any(|key| key.contains(';')) => {
            Some("json_media_variant_unchecked")
        }
        [..] if matches.len() > 1 => Some("json_media_ambiguous"),
        _ => None,
    };
    if let Some(reason) = refusal {
        let mut withheld = Message::withheld(reason);
        withheld.declared_media = message.declared_media;
        return withheld;
    }
    let key = matches[0];
    let entry = content.child(key).unwrap();
    message.json_source = Some(JsonMediaSource::MessageContent);
    let Some(schema) = entry.child("schema") else {
        message.coverage = Coverage::Withheld;
        message.limit(
            Vec::new(),
            if entry.value.is_object() {
                "media_schema_missing"
            } else {
                "media_schema_unreadable"
            },
        );
        return message;
    };
    if !(schema.value.is_object() || dialect == Dialect::OpenApi31 && schema.value.is_boolean()) {
        message.coverage = Coverage::Withheld;
        message.limit(Vec::new(), "media_schema_unreadable");
        return message;
    }
    if entries.iter().any(|(other, entry)| {
        other != key
            && entry.get("schema").is_some_and(|schema| {
                !request
                    || schema.get("properties").is_some()
                    || schema.get("items").is_some()
                    || schema.get("additionalProperties").is_some()
                    || schema.get("$ref").is_some()
            })
    }) {
        message.partial(Vec::new(), "non_json_media", false);
    }
    if request && location.value.get("required").is_some() {
        message.partial(Vec::new(), "body_presence_unchecked", false);
    }
    if request {
        if let Some(encoding) = entry.value.get("encoding") {
            if let Some(names) = encoding.as_object() {
                for key in names.keys() {
                    message.partial(
                        vec![PointerToken::property(key)],
                        "media_encoding_unchecked",
                        true,
                    );
                }
            } else {
                message.partial(Vec::new(), "media_encoding_unchecked", true);
            }
        }
    } else if location.value.get("links").is_some() {
        message.partial(Vec::new(), "response_link_unchecked", true);
    }
    message.schema = Some(schema);
    message
}

pub(super) fn swagger<'r, 'a>(
    input: &AdmittedSchemaInput<'_>,
    operation: &OperationId,
    location: &Located<'r, 'a>,
    body: Located<'r, 'a>,
    status: Option<SuccessKey>,
) -> Message<'r, 'a> {
    let mut message = Message::empty();
    let Some(schema) = body.child("schema") else {
        return message;
    };
    let key = if status.is_some() {
        "produces"
    } else {
        "consumes"
    };
    let media = location
        .value
        .get(key)
        .map(|raw| (raw, JsonMediaSource::OperationMedia))
        .or_else(|| {
            location
                .document
                .value
                .get(key)
                .map(|raw| (raw, JsonMediaSource::DocumentMedia))
        });
    if let Some((raw, source)) = media {
        let Some(entries) = raw
            .as_array()
            .and_then(|values| values.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        else {
            return Message::withheld("media_unreadable");
        };
        message.declared_media = entries.iter().map(|s| (*s).into()).collect();
        if !entries.iter().any(|entry| json_media(entry)) {
            message.coverage = Coverage::Withheld;
            message.limit(Vec::new(), "non_json_media");
            return message;
        }
        message.json_source = Some(source);
        if entries.iter().any(|entry| !json_media(entry)) {
            message.partial(Vec::new(), "non_json_media", false);
        }
    } else if let Some(source) = omission_proof(input, operation, status) {
        message.json_source = Some(source);
    } else {
        message.coverage = if input.media_receipts.is_empty() {
            Coverage::ObservedOnly
        } else {
            Coverage::Withheld
        };
        message.limit(Vec::new(), "media_unverified");
    }
    if status.is_none() && body.value.get("required").is_some() {
        if message.coverage != Coverage::Withheld {
            message.coverage = Coverage::Partial;
        }
        message.limit(Vec::new(), "body_presence_unchecked");
    }
    message.schema = Some(schema);
    message
}

fn media_type(value: &str) -> String {
    value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn json_media(value: &str) -> bool {
    media_type(value) == "application/json"
}

fn omission_proof(
    input: &AdmittedSchemaInput<'_>,
    operation: &OperationId,
    status: Option<SuccessKey>,
) -> Option<JsonMediaSource> {
    let operation = operation.to_string();
    let expected_status = status.map(|key| key.to_string());
    let direction = if status.is_some() {
        "response"
    } else {
        "request"
    };
    let mut verified = None;
    for evidence in input.media_receipts {
        let receipt = evidence.receipt;
        let Some(entry) = receipt.entry.checked_sub(1) else {
            continue;
        };
        let Ok(catalog) = serde_json::from_slice::<Value>(evidence.bytes) else {
            continue;
        };
        let Some(record) = catalog.get(entry) else {
            continue;
        };
        if record["operation"] != operation
            || record["direction"] != direction
            || record.get("status")
                != Some(
                    &expected_status
                        .as_deref()
                        .map_or(Value::Null, |status| Value::String(status.into())),
                )
        {
            continue;
        }
        if receipt.tree.is_empty()
            || receipt.input_id.is_empty()
            || receipt.path.is_empty()
            || receipt.blob.is_empty()
            || record["api"] != input.api
            || record["producer_tree"] != input.snapshot.tree()
            || record["schema_blob"] != input.entry.oid
            || record["cutoff_admitted"] != true
            || !record["media_type"].as_str().is_some_and(json_media)
            || !proof_paths(record, status.is_none())
            || !match record["route"].as_str() {
                Some("producer_source") => true,
                Some("server_codec") => {
                    let generator = &record["generator"];
                    ["name", "version", "settings", "contract"]
                        .iter()
                        .all(|key| text(&generator[key]))
                        && generator["input"] == input.entry.oid
                        && generator["installed"] == true
                        && generator["final_dispatch_json"] == true
                }
                _ => false,
            }
        {
            return None;
        }
        let source = JsonMediaSource::Producer {
            input_id: receipt.input_id.into(),
            entry: receipt.entry,
        };
        if verified.as_ref().is_some_and(|other| other != &source) {
            return None;
        }
        verified = Some(source);
    }
    verified
}

fn proof_paths(record: &Value, request: bool) -> bool {
    let coverage = &record["coverage"];
    coverage["complete"] == true
        && coverage["paths"].as_array().is_some_and(|paths| {
            paths.iter().any(|path| path["outcome"] == "json")
                && paths.iter().all(|path| {
                    text(&path["file"])
                        && text(&path["blob"])
                        && path["span"].as_array().is_some_and(|span| {
                            matches!(span.as_slice(), [start, end]
                    if start.as_u64().zip(end.as_u64()).is_some_and(|(start,end)| start <= end))
                        })
                        && match path["outcome"].as_str() {
                            Some("json") => {
                                path["final_content_type"].as_str().is_some_and(json_media)
                                    && (!request || path["selects_request_content_type"] == true)
                            }
                            Some("excluded") => text(&path["reason"]),
                            _ => false,
                        }
                })
        })
}

fn text(value: &Value) -> bool {
    value.as_str().is_some_and(|s| !s.is_empty())
}
