mod media;
pub(crate) use media::valid_media_proof;

use super::openapi::{entry_value, scan_operations};
use super::references::{Documents, Located};
use super::{signature, wire, Declaration, Dialect, SchemaError};
use crate::element::{MessageRoot, SuccessKey};
use crate::inventory::{
    BlockEffect, Coverage, FieldBlock, MessageCoverage, Operation, OperationSite, RegionLimit,
    RootKey, SchemaIndex, SchemaSource, StatusInventory, WireEvidence,
};
use crate::revision::{AdmittedSchemaInput, Snapshot};
use media::Message;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn project(input: &AdmittedSchemaInput<'_>) -> Result<SchemaIndex, SchemaError> {
    let documents = Documents::new(input, entry_value(input)?);
    let inventory = scan_operations(&documents, input)?;
    let mut index = SchemaIndex {
        source: Some(SchemaSource {
            api: inventory.api,
            authority: inventory.authority,
            tree: input.snapshot.tree().into(),
            commit: match input.snapshot {
                Snapshot::Commit { commit, .. } => Some(commit.into()),
                Snapshot::ComputedTree { .. } => None,
            },
            path: input.entry.path.into(),
            blob: input.entry.oid.into(),
        }),
        input_closure: Some(input_closure(input)),
        info_version: inventory.info_version,
        diagnostics: inventory.diagnostics,
        operation_gaps: inventory.operation_gaps,
        byte_sha256: inventory
            .digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        operation_coverage_incomplete: !inventory.unprojected.is_empty(),
        ..SchemaIndex::default()
    };
    for diagnostic in &index.diagnostics {
        // These are represented at their message root below.
        if matches!(diagnostic.code, "fallback_success" | "duplicate_operation") {
            continue;
        }
        if diagnostic.code == "duplicate_member" && diagnostic.scope == "responses" {
            continue;
        }
        if diagnostic.scope == "origin" {
            continue;
        }
        index.operation_coverage_incomplete = true;
        index.limits.push(RegionLimit {
            pointer: Vec::new(),
            reason: diagnostic.code.into(),
        });
    }
    for document in documents.documents() {
        for definition in wire::declared_definitions(document) {
            index.inverse.entry(definition).or_default();
        }
    }
    for (id, operation) in inventory.operations {
        let declaration = &operation.declarations[0];
        let location = documents
            .locate(declaration.resource.as_deref(), &declaration.site)
            .expect("selected operation has an admitted location");
        let dialect = operation.dialect;
        let duplicate = operation.declarations.len() > 1;
        let mut status_refusals = BTreeMap::new();
        let request_eligible = request_eligible(dialect, id.method());
        let mut request_authored_but_excluded = false;
        let mut projected = Operation {
            id: id.clone(),
            success_keys: operation.success_keys().to_vec(),
            response_keys: operation.responses().keys().cloned().collect(),
            operation_id: operation.operation_id.clone(),
            declarations: operation
                .declarations
                .iter()
                .map(|declaration| {
                    let location = documents
                        .locate(declaration.resource.as_deref(), &declaration.site)
                        .expect("indexed declaration has an admitted location");
                    request_authored_but_excluded |=
                        !request_eligible && location.value.get("requestBody").is_some();
                    let status_refusal = if location.duplicate("responses") {
                        Some("duplicate_member")
                    } else {
                        match location.value.get("responses") {
                            Some(value) if value.is_object() => None,
                            None if dialect == Dialect::OpenApi31 => None,
                            _ => Some("responses_unreadable"),
                        }
                    };
                    if let Some(reason) = status_refusal {
                        status_refusals.insert(declaration.site.clone(), reason);
                    }
                    OperationSite {
                        path: declaration.source_path.clone(),
                        canonical_path: declaration.path.clone(),
                        method: declaration.source_method.clone(),
                        site: declaration.site.clone(),
                        referring_site: declaration.referring_site.clone(),
                        reference_chain: declaration.reference_chain.clone(),
                        status_inventory: if status_refusal.is_some() {
                            StatusInventory::Withheld
                        } else {
                            StatusInventory::Readable(
                                declaration.responses.keys().cloned().collect(),
                            )
                        },
                    }
                })
                .collect(),
            ranged_responses: dialect != Dialect::Swagger2,
            request_eligible,
            request_authored_but_excluded,
            status_refusals,
            wire_evidence: BTreeMap::new(),
            messages: Vec::new(),
        };
        let mut reached = BTreeSet::new();
        let mut incomplete = duplicate
            || location
                .value
                .get("responses")
                .is_some_and(|value| !value.is_object());
        let mut messages = Vec::new();
        if let Some(message) = request(&documents, input, &operation, &location) {
            messages.push((RootKey::Request, message));
        }
        let responses = location.child("responses");
        for key in operation.success_keys() {
            let spelling = key.to_string();
            let message = if responses
                .as_ref()
                .is_some_and(|value| value.duplicate(&spelling))
            {
                let mut message = Message::withheld("response_key_ambiguous");
                message.limit(Vec::new(), "duplicate_member");
                message
            } else if !(200..=299).any(|code| {
                operation.can_have_body(code) && operation.response_key(code) == Some(&spelling)
            }) {
                Message::empty()
            } else {
                let raw = responses.as_ref().unwrap().child(&spelling).unwrap();
                response(&documents, input, &operation, &location, *key, raw)
            };
            messages.push((RootKey::Response(*key), message));
        }
        if location.duplicate("responses")
            || responses
                .as_ref()
                .is_some_and(|value| value.duplicate("default"))
        {
            messages.push((RootKey::Fallback, Message::withheld("duplicate_member")));
        } else if operation.default_response().is_some() {
            let message = if (200..=299).any(|code| operation.response_key(code) == Some("default"))
            {
                Message::withheld("fallback_success")
            } else {
                Message::empty()
            };
            messages.push((RootKey::Fallback, message));
        }
        for (key, mut message) in messages {
            let mut evidence = WireEvidence::default();
            let root = match key {
                RootKey::Request => Some(MessageRoot::Request(id.clone())),
                RootKey::Response(key) => Some(MessageRoot::Response(id.clone(), key)),
                RootKey::Fallback => None,
            };
            let mut feasible = None;
            let selected_schema = message.schema.as_ref().map(|schema| schema.site.clone());
            if let (Some(schema), Some(root)) = (message.schema.take(), root) {
                let result = wire::read(&documents, dialect, &root, schema);
                feasible = result.feasible;
                evidence = result.evidence;
                if message.coverage == Coverage::ObservedOnly
                    && result.fields.is_empty()
                    && result.limits.is_empty()
                {
                    evidence.empty_projection_with_unproved_media = true;
                    message.coverage = Coverage::Checked;
                    message
                        .limits
                        .retain(|limit| limit.reason != "media_unverified");
                }
                reached.extend(result.definitions);
                index.warnings.extend(result.warnings);
                for (field, comparable) in result.fields {
                    let blocked = message
                        .blocks
                        .iter()
                        .any(|prefix| field.id.pointer.starts_with(prefix));
                    if comparable
                        && !blocked
                        && !duplicate
                        && message.coverage != Coverage::Withheld
                    {
                        index.public_fields.push(field.id.to_string());
                    }
                    if comparable
                        && !blocked
                        && !duplicate
                        && message.coverage != Coverage::Withheld
                        && message.json_source.is_some()
                    {
                        index.fields.push(field);
                    } else {
                        index.observed_fields.push(field);
                    }
                }
                if !result.limits.is_empty() && message.coverage != Coverage::Withheld {
                    message.coverage = Coverage::Partial;
                }
                message.limits.extend(result.limits);
            }
            if duplicate {
                message.coverage = Coverage::Withheld;
                message.limit(Vec::new(), "duplicate_operation");
            }
            for limit in &message.limits {
                if message.coverage == Coverage::Withheld || message.blocks.contains(&limit.pointer)
                {
                    evidence.blocks.push(FieldBlock {
                        pointer: limit.pointer.clone(),
                        reason: limit.reason.clone(),
                        effect: BlockEffect::Field,
                    });
                }
            }
            incomplete |= !matches!(message.coverage, Coverage::Checked)
                && message.limits.iter().any(|limit| {
                    !matches!(
                        limit.reason.as_str(),
                        "media_unverified" | "non_json_media" | "body_presence_unchecked"
                    )
                });
            projected.wire_evidence.insert(key.clone(), evidence);
            projected.messages.push(MessageCoverage {
                key,
                coverage: message.coverage,
                limits: message.limits,
                feasible,
                selected_schema,
                declared_media: message.declared_media,
                declaration_source: message.declaration_source,
                json_source: message.json_source,
            });
        }
        let reached: Vec<_> = reached.into_iter().collect();
        for definition in &reached {
            index
                .inverse
                .entry(definition.clone())
                .or_default()
                .push(id.clone());
        }
        index.reached.insert(id, (incomplete, reached));
        index.operations.push(projected);
    }
    index.operations.sort_by_key(|op| op.id.to_string());
    index.fields.sort_by_key(|field| field.id.to_string());
    index
        .observed_fields
        .sort_by_key(|field| field.id.to_string());
    index.public_fields.sort();
    index.public_fields.dedup();
    index.content_hash = signature::content_hash(&index);
    Ok(index)
}

fn request_eligible(dialect: Dialect, method: &str) -> bool {
    match dialect {
        Dialect::Swagger2 => true,
        Dialect::OpenApi30 => matches!(method, "POST" | "PUT" | "PATCH"),
        Dialect::OpenApi31 => method != "TRACE",
    }
}

fn request<'r, 'a>(
    documents: &'r Documents<'a>,
    input: &AdmittedSchemaInput<'_>,
    operation: &super::Operation,
    location: &Located<'r, 'a>,
) -> Option<Message<'r, 'a>> {
    let dialect = operation.dialect;
    if dialect == Dialect::Swagger2 {
        return match swagger_body(documents, &operation.declarations[0], location) {
            Ok(Body::None) => None,
            Ok(Body::Form) => Some(Message::withheld("non_json_media")),
            Ok(Body::Json(body)) => {
                Some(media::swagger(input, &operation.id, location, body, None))
            }
            Err(reason) => Some(Message::withheld(reason)),
        };
    }
    if !request_eligible(dialect, operation.id.method()) {
        return None;
    }
    if location.duplicate("requestBody") {
        return Some(Message::withheld("duplicate_member"));
    }
    let raw = location.child("requestBody")?;
    Some(
        match resolve_message(documents, dialect, raw, MessageRole::RequestBody) {
            Ok(body) => media::openapi(body, true, dialect),
            Err(reason) => Message::withheld(reason),
        },
    )
}

fn response<'r, 'a>(
    documents: &'r Documents<'a>,
    input: &AdmittedSchemaInput<'_>,
    operation: &super::Operation,
    location: &Located<'r, 'a>,
    key: SuccessKey,
    raw: Located<'r, 'a>,
) -> Message<'r, 'a> {
    let dialect = operation.dialect;
    let response = match resolve_message(documents, dialect, raw, MessageRole::Response) {
        Ok(response) => response,
        Err(reason) => return Message::withheld(reason),
    };
    if dialect == Dialect::Swagger2 {
        media::swagger(input, &operation.id, location, response, Some(key))
    } else {
        media::openapi(response, false, dialect)
    }
}

#[derive(Clone, Copy)]
enum MessageRole {
    RequestBody,
    Response,
    SwaggerParameter,
}

impl MessageRole {
    const fn unreadable(self) -> &'static str {
        match self {
            Self::RequestBody => "request_body_unreadable",
            Self::Response => "response_unreadable",
            Self::SwaggerParameter => "body_parameters_unreadable",
        }
    }

    fn admits_terminal(self, dialect: Dialect, value: &Value) -> bool {
        let required = match self {
            Self::Response => "description",
            Self::RequestBody => "content",
            Self::SwaggerParameter => return true,
        };
        value.as_object().is_some_and(|object| {
            object.contains_key(required)
                && object.iter().all(|(key, member)| {
                    if key.starts_with("x-") {
                        return true;
                    }
                    match (self, dialect, key.as_str()) {
                        (_, _, "description") => member.is_string(),
                        (Self::Response, Dialect::Swagger2, "schema") => {
                            member.is_object() || member.is_boolean()
                        }
                        (Self::Response, Dialect::Swagger2, "examples")
                        | (
                            Self::Response,
                            Dialect::OpenApi30 | Dialect::OpenApi31,
                            "content" | "links",
                        )
                        | (Self::Response, _, "headers")
                        | (Self::RequestBody, _, "content") => member.is_object(),
                        (Self::RequestBody, _, "required") => member.is_boolean(),
                        _ => false,
                    }
                })
        })
    }
}

fn resolve_message<'r, 'a>(
    documents: &'r Documents<'a>,
    dialect: Dialect,
    raw: Located<'r, 'a>,
    role: MessageRole,
) -> Result<Located<'r, 'a>, &'static str> {
    if raw.has_duplicates() {
        return Err("duplicate_member");
    }
    if !raw.value.is_object() {
        return Err(role.unreadable());
    }
    let mut chain = documents
        .resolve(raw)
        .map_err(|failure| wire::reference_reason(failure.code))?;
    if chain.iter().any(Located::has_duplicates) {
        return Err("duplicate_member");
    }
    let terminal = chain.pop().unwrap();
    if !chain.is_empty() && !role.admits_terminal(dialect, terminal.value) {
        return Err("reference_unresolved");
    }
    if dialect == Dialect::OpenApi31
        && chain.iter().any(|node| {
            node.value
                .as_object()
                .unwrap()
                .keys()
                .any(|key| !matches!(key.as_str(), "$ref" | "summary" | "description"))
        })
    {
        return Err("reference_siblings_unchecked");
    }
    Ok(terminal)
}

enum Body<'r, 'a> {
    None,
    Form,
    Json(Located<'r, 'a>),
}

fn swagger_body<'r, 'a>(
    documents: &'r Documents<'a>,
    declaration: &Declaration,
    operation: &Located<'r, 'a>,
) -> Result<Body<'r, 'a>, &'static str> {
    const UNREADABLE: &str = MessageRole::SwaggerParameter.unreadable();
    let path = declaration
        .parameters
        .as_ref()
        .map_err(|_| UNREADABLE)?
        .as_ref()
        .and_then(|(uri, site)| documents.locate(uri.as_deref(), site));
    let mut effective = BTreeMap::new();
    for list in [path, operation.child("parameters")].into_iter().flatten() {
        if list.document.ambiguous_path(&list.site.pointer) {
            return Err("duplicate_member");
        }
        let items = list.value.as_array().ok_or(UNREADABLE)?;
        let mut local = BTreeSet::new();
        for (i, value) in items.iter().enumerate() {
            let parameter = resolve_message(
                documents,
                Dialect::Swagger2,
                Located {
                    value,
                    document: list.document,
                    site: list.site.child(&i.to_string()),
                },
                MessageRole::SwaggerParameter,
            )
            .map_err(|reason| {
                if reason == "duplicate_member" {
                    reason
                } else {
                    UNREADABLE
                }
            })?;
            let key = (
                parameter.value["name"].as_str().ok_or(UNREADABLE)?,
                parameter.value["in"].as_str().ok_or(UNREADABLE)?,
            );
            if !local.insert(key) {
                return Err(UNREADABLE);
            }
            effective.insert(key, parameter);
        }
    }
    let has_form = effective
        .keys()
        .any(|(_, location)| *location == "formData");
    let mut bodies = effective
        .into_iter()
        .filter_map(|((_, location), parameter)| (location == "body").then_some(parameter));
    let Some(body) = bodies.next() else {
        return Ok(if has_form { Body::Form } else { Body::None });
    };
    if has_form {
        return Err("body_form_conflict");
    }
    if bodies.next().is_some() {
        return Err("multiple_request_bodies");
    }
    if !body
        .value
        .get("schema")
        .is_some_and(serde_json::Value::is_object)
    {
        return Err(UNREADABLE);
    }
    Ok(Body::Json(body))
}

fn input_closure(input: &AdmittedSchemaInput<'_>) -> crate::inventory::SchemaInputClosure {
    let mut references: Vec<_> = input
        .references
        .iter()
        .map(|record| {
            let blob = record.blob;
            (
                record.retrieval_uri.into(),
                blob.tree.into(),
                blob.path.into(),
                blob.oid.into(),
                format!("{:x}", Sha256::digest(blob.bytes)),
            )
        })
        .collect();
    let mut config: Vec<_> = input
        .config_receipts
        .iter()
        .map(|record| {
            (
                record.tree.into(),
                record.path.into(),
                record.blob.into(),
                record.entry,
            )
        })
        .collect();
    let mut media: Vec<_> = input
        .media_receipts
        .iter()
        .map(|record| {
            let receipt = record.receipt;
            (
                receipt.input_id.into(),
                receipt.tree.into(),
                receipt.path.into(),
                receipt.blob.into(),
                receipt.entry,
                format!("{:x}", Sha256::digest(record.bytes)),
            )
        })
        .collect();
    references.sort_unstable();
    references.dedup();
    config.sort_unstable();
    config.dedup();
    media.sort_unstable();
    media.dedup();
    crate::inventory::SchemaInputClosure {
        retrieval_uri: input.retrieval_uri.map(str::to_owned),
        references,
        config,
        media,
    }
}
