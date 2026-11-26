use super::{ReviewMode, Role, SnapshotInput};
use crate::AdmittedSchemaInput;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scope {
    Api,
    PathPrefix {
        segments: Vec<Vec<crate::element::PathToken>>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct ScopeRemainder {
    pub requested: Scope,
    pub excluded: Scope,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigSource {
    pub tree: String,
    pub path: String,
    pub blob: String,
    pub entry: usize,
    pub era: String,
    pub defaults: String,
    pub export: String,
    pub mount: String,
    pub strip: String,
    pub add: String,
    pub api: String,
    pub dependencies: Vec<String>,
    pub hook: Option<String>,
    pub delegate: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InputSlot {
    pub slot: String,
    pub input_id: String,
    pub api: String,
    pub role: Role,
    pub repo: String,
    pub authority: String,
    pub scope: Scope,
    pub rev: Option<String>,
    pub tree: String,
    pub config: Option<ConfigSource>,
    pub branch: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReceiptOrigin {
    NamedTree {
        input_id: String,
        repo: String,
        tree: String,
        path: String,
        blob: String,
        availability: Availability,
    },
    IndependentCutoff {
        evidence: String,
        available_at: String,
        covered_tree: Option<String>,
    },
    Derived {
        version: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Missing,
    Unreadable,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    Blob,
    Config,
    Mapping,
    Source,
    Media,
    Inventory,
    Derived,
    #[serde(other)]
    Unsupported,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdmissionReceipt {
    pub id: String,
    pub kind: ReceiptKind,
    pub origin: ReceiptOrigin,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub format: Option<String>,
    pub digest: Option<ByteDigest>,
    pub retrieval_uri: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ByteDigest {
    pub algorithm: String,
    pub framing: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SelectionReceipt {
    LiveRef {
        branch: String,
        rev: String,
        observed_at: String,
    },
    HistoricalPin {
        branch: String,
        rev: String,
        available_at: String,
        evidence: String,
    },
    ExplicitPin {
        rev: String,
        selection_time: Option<String>,
        selected_by: Option<String>,
        selection_reason: String,
    },
    DateRule {
        rev: String,
        selection_time: String,
        selected_by: String,
        selection_reason: String,
        rule: Option<String>,
        history_tip: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct MergeReceipt {
    pub id: String,
    pub base: Option<String>,
    pub head: Option<String>,
    pub target: Option<String>,
    pub procedure: Option<String>,
    pub result_tree: Option<String>,
    pub computed_at: Option<String>,
    pub cutoff_evidence: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ScopeProjection {
    pub requested: Scope,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdmissionClaim {
    pub id: String,
    pub input_id: String,
    pub slot: Option<String>,
    pub receipt_ids: Vec<String>,
    pub projection: Option<ScopeProjection>,
    #[serde(default)]
    pub requires_selection: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdmissionInput {
    pub binding: InputSlot,
    pub snapshot: SnapshotInput,
    pub dirty: Option<bool>,
    pub selection: Option<SelectionReceipt>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdmissionRequest {
    pub mode: ReviewMode,
    pub report_time: String,
    pub cutoff_at: String,
    pub selection_time: Option<String>,
    pub subject_head_committed_at: Option<String>,
    pub slots: Vec<InputSlot>,
    pub inputs: Vec<AdmissionInput>,
    pub receipts: Vec<AdmissionReceipt>,
    pub claims: Vec<AdmissionClaim>,
    pub expected_merge: Option<MergeReceipt>,
    pub merge: Option<MergeReceipt>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Refusal {
    pub code: String,
    pub dimension: Option<String>,
    pub record_id: Option<String>,
    pub expected: Option<String>,
    pub received: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaimDecision {
    pub status: String,
    pub reasons: Vec<Refusal>,
    pub input_id: Option<String>,
    pub effective_scope: Option<Scope>,
    pub uncovered_scope: Option<ScopeRemainder>,
    pub dirty: Option<bool>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct AdmissionReport {
    pub claims: BTreeMap<String, ClaimDecision>,
    pub counts: BTreeMap<String, usize>,
    pub mode: Option<ReviewMode>,
}

pub fn admit_inputs(
    _request: &AdmissionRequest,
    _bytes: &BTreeMap<String, Vec<u8>>,
) -> AdmissionReport {
    AdmissionReport::default()
}

pub struct SchemaAdmission<'a> {
    pub input: Option<AdmittedSchemaInput<'a>>,
    pub report: AdmissionReport,
}

pub fn admit_schema_input<'a>(
    _request: &'a AdmissionRequest,
    _bytes: &'a BTreeMap<String, Vec<u8>>,
    _claim_id: &str,
) -> SchemaAdmission<'a> {
    SchemaAdmission {
        input: None,
        report: AdmissionReport::default(),
    }
}
