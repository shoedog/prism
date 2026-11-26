use super::{DocumentError, DocumentInput, EvidenceRecord, ExtractionRegion, SourceSite};
use super::{SchemaOrigin, WireSignature};
use crate::element::MessageRoot;
use crate::revision::SavedScan;
use crate::{Coverage, Field, JsonMediaSource, RootKey, SchemaIndex};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BindingsDocument {
    pub schema_version: String,
    #[serde(flatten)]
    pub input: Option<DocumentInput>,
    pub expected_files: Vec<String>,
    pub file_status: Vec<ExtractionRegion>,
    pub entries: Vec<ExtractionRegion>,
    pub unaccounted_scopes: Vec<ExtractionRegion>,
    pub dependency_records: Vec<EvidenceRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operations: Option<Vec<Binding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls: Option<Vec<Unbound>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_scan: Option<SavedScan>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Binding {
    pub operation_id: String,
    pub original_method: String,
    pub original_path: String,
    pub provenance: String,
    pub sites: Vec<SourceSite>,
    pub config_source: Option<usize>,
    pub suffix: Option<String>,
    pub plugin: Option<String>,
    #[serde(default)]
    pub success_statuses: Vec<String>,
    #[serde(default)]
    pub message_roots: Vec<MessageRecord>,
    #[serde(default)]
    pub fields: Vec<FieldRecord>,
    #[serde(default)]
    pub observed_fields: Vec<FieldRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Unbound {
    pub site_id: String,
    pub site: Option<SourceSite>,
    pub enclosing_function: Option<String>,
    pub function_span: Option<[usize; 2]>,
    pub requested_method: Option<String>,
    pub original_url_text: Option<String>,
    pub mapped_path: Option<String>,
    pub mapping_source: Option<usize>,
    pub target_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_match: Option<SavedMatch>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedMatch {
    pub grade: String,
    pub target_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_input_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlaps: Vec<OverlapEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_use: Vec<FieldUse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OverlapEvidence {
    pub element_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FieldUse {
    pub field_id: String,
    pub site_id: String,
    pub kind: String,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FieldRecord {
    pub field_id: String,
    pub signature: WireSignature,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<String>,
}

impl From<&Field> for FieldRecord {
    fn from(field: &Field) -> Self {
        Self {
            field_id: field.id.to_string(),
            signature: WireSignature::from_shape(&field.shape),
            presence: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MessageRecord {
    pub direction: String,
    pub status: Option<String>,
    pub body_presence: String,
    pub declared_media: Vec<String>,
    pub effective_media: Option<String>,
    pub coverage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_source: Option<MediaSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_schema: Option<SchemaLocation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reader_limits: Vec<MessageLimit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer_media_ref: Option<MediaRef>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSource {
    MessageContent,
    OperationMedia,
    DocumentMedia,
    Producer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaLocation {
    pub tree: String,
    pub file: String,
    pub blob: String,
    pub pointer: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MessageLimit {
    pub pointer: Vec<super::WirePointer>,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MediaRef {
    pub input_id: String,
    pub entry: usize,
}

impl BindingsDocument {
    pub fn from_json(bytes: &[u8]) -> Result<Self, DocumentError> {
        super::decode::document(bytes, super::decode::Kind::Bindings)
    }

    pub fn with_schema_inventory(mut self, inventory: &SchemaIndex) -> Result<Self, DocumentError> {
        let Some((input, source)) = self.input.as_mut().zip(inventory.source.as_ref()) else {
            return Err(DocumentError::InvalidDocument);
        };
        if !input.matches_schema_source(source) || !input.scope.same_region(&inventory.scope) {
            return Err(DocumentError::InvalidDocument);
        }
        let Some(schema) = input.schema.as_mut() else {
            return Err(DocumentError::InvalidDocument);
        };
        let SchemaOrigin::File { sha256, .. } = &mut schema.source else {
            return Err(DocumentError::InvalidDocument);
        };
        *sha256 = inventory.byte_sha256.clone();
        schema.content_hash = Some(inventory.content_hash.clone());
        schema.info_version = inventory.info_version.clone();

        let operations = self.operations.get_or_insert_with(Vec::new);
        for op in &inventory.operations {
            let Some(decl) = op.declarations.first() else {
                continue;
            };
            let position = operations
                .iter()
                .position(|row| row.operation_id == op.id.to_string());
            let belongs = |field: &&Field| match &field.id.root {
                MessageRoot::Request(owner) | MessageRoot::Response(owner, _) => *owner == op.id,
            };
            let fields = inventory
                .fields
                .iter()
                .filter(belongs)
                .map(FieldRecord::from)
                .collect();
            let observed_fields = inventory
                .observed_fields
                .iter()
                .filter(belongs)
                .map(FieldRecord::from)
                .collect();
            let message_roots = op
                .messages
                .iter()
                .map(|message| {
                    let (direction, status) = match &message.key {
                        RootKey::Request => ("request", None),
                        RootKey::Response(status) => ("response", Some(status.to_string())),
                        RootKey::Fallback => ("response", Some("default".into())),
                    };
                    let body_presence = if message.feasible.is_some()
                        || !message.declared_media.is_empty()
                        || message.json_source.is_some()
                    {
                        "present"
                    } else if message.coverage == Coverage::Checked {
                        "absent"
                    } else {
                        "unknown"
                    };
                    MessageRecord {
                        direction: direction.into(),
                        status,
                        body_presence: body_presence.into(),
                        declared_media: message.declared_media.clone(),
                        effective_media: message
                            .json_source
                            .as_ref()
                            .map(|_| "application/json".into()),
                        coverage: match message.coverage {
                            Coverage::Checked => "checked",
                            Coverage::ObservedOnly => "observed_only",
                            Coverage::Partial => "partial",
                            Coverage::Withheld => "withheld",
                        }
                        .into(),
                        media_source: message.json_source.as_ref().map(|source| match source {
                            JsonMediaSource::MessageContent => MediaSource::MessageContent,
                            JsonMediaSource::OperationMedia => MediaSource::OperationMedia,
                            JsonMediaSource::DocumentMedia => MediaSource::DocumentMedia,
                            JsonMediaSource::Producer { .. } => MediaSource::Producer,
                        }),
                        selected_schema: message.selected_schema.as_ref().map(|site| {
                            SchemaLocation {
                                tree: site.tree.clone(),
                                file: site.file.clone(),
                                blob: site.blob.clone(),
                                pointer: site.pointer.clone(),
                            }
                        }),
                        reader_limits: message
                            .limits
                            .iter()
                            .map(|limit| MessageLimit {
                                pointer: limit
                                    .pointer
                                    .iter()
                                    .map(super::WirePointer::from)
                                    .collect(),
                                reason: limit.reason.clone(),
                            })
                            .collect(),
                        producer_media_ref: match &message.json_source {
                            Some(JsonMediaSource::Producer { input_id, entry }) => Some(MediaRef {
                                input_id: input_id.clone(),
                                entry: *entry,
                            }),
                            _ => None,
                        },
                    }
                })
                .collect();
            let success_statuses = op.success_keys.iter().map(ToString::to_string).collect();
            if let Some(i) = position {
                let row = &mut operations[i];
                row.success_statuses = success_statuses;
                row.message_roots = message_roots;
                row.fields = fields;
                row.observed_fields = observed_fields;
            } else {
                operations.push(Binding {
                    operation_id: op.id.to_string(),
                    original_method: decl.method.clone(),
                    original_path: decl.path.clone(),
                    provenance: "proven".into(),
                    sites: Vec::new(),
                    config_source: None,
                    suffix: None,
                    plugin: None,
                    success_statuses,
                    message_roots,
                    fields,
                    observed_fields,
                });
            }
        }
        Ok(self)
    }

    pub fn with_extraction_coverage(mut self) -> Self {
        if let Some(input) = &self.input {
            for path in &self.expected_files {
                if self
                    .file_status
                    .iter()
                    .chain(&self.unaccounted_scopes)
                    .any(|region| region.path == *path)
                {
                    continue;
                }
                self.unaccounted_scopes.push(super::ExtractionRegion {
                    path: path.clone(),
                    scope: input.scope.clone(),
                    status: super::ExtractionStatus::Unknown,
                    reason: Some("missing_file_status".into()),
                    possible_keys: Vec::new(),
                    evidence: None,
                });
            }
        }
        self
    }
}
