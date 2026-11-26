//! Versioned snapshot and review document declarations for the P1d RED overlay.

mod bindings;
mod impact;
pub use crate::revision::{BindingStatus, SnapshotInput};
pub use bindings::{Binding, BindingsDocument, Unbound};
pub use impact::{unobserved_elements, ElementChangeRecord, ImpactDocument, MembershipRequest};

use crate::revision::{ConfigSource, ReceiptOrigin, Role, Scope};
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
pub struct DocumentInput {
    pub input_id: String,
    pub api: String,
    pub repo: String,
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
}

#[derive(Clone, Debug, Deserialize)]
pub struct ToolIdentity {
    pub name: String,
    pub build: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SchemaEvidence {
    pub source: SchemaOrigin,
    pub content_hash: Option<String>,
    pub info_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SchemaOrigin {
    File {
        repo: String,
        tree: String,
        rev: Option<String>,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct WireSignature(pub [Value; 5]);

#[derive(Clone, Debug, Deserialize)]
pub struct SourceSite {
    pub tree: String,
    pub file: String,
    pub blob: String,
    pub line: usize,
    pub span: Option<[usize; 2]>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EvidenceRecord {
    pub input_id: String,
    pub origin: ReceiptOrigin,
    pub dependencies: Vec<String>,
    pub tool: ToolIdentity,
    pub scope: Scope,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExtractionRegion {
    pub path: String,
    pub scope: Scope,
    pub status: ExtractionStatus,
    pub reason: Option<String>,
    #[serde(default)]
    pub possible_keys: Vec<String>,
    pub evidence: Option<Value>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionStatus {
    Complete,
    Partial,
    Unreadable,
    Skipped,
    Unknown,
}
