//! Admission binds named-tree bytes and typed receipts to the selected slots.
//! Admission preserves independently supported claims and counts scoped refusals.
//! Bindings preserve snapshot identity, raw observations, and extraction coverage.
//! Impact retains both revision-local sides and each comparison's own occurrences.
//! Impact stores each standing limit once and links supported affected claims.
//! Saved status compares the same admitted element on the selected service side.
//! Membership negatives require complete evidence across all applicable inputs.
//! Both 0.1 schemas validate their own DTOs and enforce closed records.

mod bindings;
mod decode;
mod impact;
mod producer;
pub use crate::revision::{BindingStatus, SnapshotInput};
pub use bindings::*;
pub use impact::*;
pub use producer::*;

use crate::revision::{ConfigSource, ReceiptOrigin, Role, Scope};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DocumentInput {
    pub input_id: String,
    pub api: String,
    pub repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub role: Role,
    #[serde(flatten)]
    pub snapshot: SnapshotInput,
    pub dirty: Option<bool>,
    pub tool: ToolIdentity,
    pub operation_authority: String,
    pub scope: Scope,
    pub origin: ReceiptOrigin,
    pub dependencies: Vec<String>,
    pub config_sources: Vec<ConfigSource>,
    pub schema: Option<SchemaEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_ids: Option<Vec<String>>,
}

impl DocumentInput {
    pub(super) fn matches_schema_source(&self, source: &crate::inventory::SchemaSource) -> bool {
        self.role == Role::Producer
            && self.api == source.api
            && self.operation_authority == source.authority
            && self.snapshot.identity() == Some((&source.tree, source.commit.as_deref()))
            && self.declares_schema_source(source)
    }

    /// Whether the schema this document declares (path, blob, repo, tree; or the named-tree
    /// entry when it carries no schema evidence) is the one `source` indexed.
    pub(crate) fn declares_schema_source(&self, source: &crate::inventory::SchemaSource) -> bool {
        match self.schema.as_ref().map(|schema| &schema.source) {
            Some(SchemaOrigin::File {
                path,
                blob,
                repo,
                tree,
                ..
            }) => {
                (path, blob, repo, tree) == (&source.path, &source.blob, &self.repo, &source.tree)
            }
            None => matches!(&self.origin, ReceiptOrigin::NamedTree {
                input_id, repo, tree, path, blob, availability: crate::revision::Availability::Available,
            } if (input_id, repo, tree, path, blob) ==
                (&self.input_id, &self.repo, &source.tree, &source.path, &source.blob)),
            _ => false,
        }
    }
}

impl DocumentInput {
    /// For an explicit schema file receipt, whether every value it declares that the index also
    /// carries agrees with the index: the SHA-256 of the bytes, the source commit (when the
    /// index has one), and the content hash and info version when declared (non-empty). The
    /// named-tree fallback declares none of these and is judged by `declares_schema_source`.
    pub(crate) fn declared_receipt_agrees(&self, schema: &crate::SchemaIndex) -> bool {
        let Some(evidence) = &self.schema else {
            return true;
        };
        let SchemaOrigin::File { rev, sha256, .. } = &evidence.source else {
            return true;
        };
        fn declared(value: &Option<String>) -> Option<&str> {
            value.as_deref().filter(|v| !v.is_empty())
        }
        let commit = schema
            .source
            .as_ref()
            .and_then(|source| source.commit.as_deref());
        *sha256 == schema.byte_sha256
            && declared(rev).is_none_or(|rev| commit.is_none_or(|commit| commit == rev))
            && declared(&evidence.content_hash).is_none_or(|hash| hash == schema.content_hash)
            && declared(&evidence.info_version)
                .is_none_or(|version| schema.info_version.as_deref() == Some(version))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolIdentity {
    pub name: String,
    pub build: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SchemaEvidence {
    pub source: SchemaOrigin,
    pub content_hash: Option<String>,
    pub info_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SchemaOrigin {
    File {
        repo: String,
        tree: String,
        rev: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subject: Option<String>,
        committed_at: Option<String>,
        path: String,
        blob: String,
        format: String,
        sha256: String,
    },
    RouteTable {
        repo: String,
        tree: String,
        input_id: String,
    },
    MissingExpectedSchema {
        repo: String,
        tree: String,
        path: String,
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentError {
    DuplicateJsonMember,
    UnsupportedFormat,
    UnsupportedVersion,
    WrongDocumentKind,
    InvalidDocument,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct WireSignature(
    pub SignatureSlot<TypeSet>,
    pub SignatureSlot<String>,
    pub SignatureSlot<bool>,
    pub SignatureSlot<Vec<Scalar>>,
    pub SignatureSlot<TypeSet>,
);

impl WireSignature {
    fn from_shape(shape: &crate::FieldShape) -> Self {
        serde_json::from_str(&crate::signature(shape))
            .expect("reader produces a five-slot signature")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SignatureSlot<T> {
    Known(KnownTag, T),
    Unresolved((UnresolvedTag,)),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnownTag {
    Known,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnresolvedTag {
    Missing,
    Unknown,
    BranchDependent,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TypeSet {
    Single(WireType),
    Multiple(Vec<WireType>),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WireType {
    Array,
    Boolean,
    Integer,
    Null,
    Number,
    Object,
    String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Scalar {
    Null(()),
    Boolean(bool),
    Number(serde_json::Number),
    String(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SourceSite {
    pub tree: String,
    pub file: String,
    pub blob: String,
    pub line: usize,
    pub span: Option<[usize; 2]>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub input_id: String,
    pub origin: ReceiptOrigin,
    pub dependencies: Vec<String>,
    pub tool: ToolIdentity,
    pub scope: Scope,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExtractionRegion {
    pub path: String,
    pub scope: Scope,
    pub status: ExtractionStatus,
    pub reason: Option<String>,
    #[serde(default)]
    pub possible_keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionStatus {
    Complete,
    Partial,
    Unreadable,
    Skipped,
    Unknown,
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DuplicateJsonMember => "duplicate JSON member",
            Self::UnsupportedFormat => "unsupported document format",
            Self::UnsupportedVersion => "unsupported document schema version",
            Self::WrongDocumentKind => "wrong document kind",
            Self::InvalidDocument => "invalid boundary document",
        })
    }
}

impl std::error::Error for DocumentError {}
