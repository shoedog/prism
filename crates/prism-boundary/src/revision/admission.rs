//! Admission binds named-tree bytes and typed receipts to the selected slots.
//! Admission preserves independently supported claims and counts scoped refusals.
//! Bindings preserve snapshot identity, raw observations, and extraction coverage.
//! Impact retains both revision-local sides and each comparison's own occurrences.
//! Impact stores each standing limit once and links supported affected claims.
//! Saved status compares the same admitted element on the selected service side.
//! Membership negatives require complete evidence across all applicable inputs.
//! Both 0.1 schemas validate their own DTOs and enforce closed records.

mod graph;
mod projection;
mod selection;
mod slots;
pub(super) use graph::Ledger;
pub use projection::{admit_schema_input, SchemaAdmission};
pub(crate) use selection::instant;

use super::{ReviewMode, Role, SnapshotInput};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_side: Option<super::ServiceSide>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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
        #[serde(skip_serializing_if = "Option::is_none")]
        covered_tree: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        repo: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        blob: Option<String>,
    },
    Derived {
        version: Option<String>,
    },
    #[serde(other)]
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Missing,
    Unreadable,
    #[serde(other)]
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AdmissionReceipt {
    pub id: String,
    pub kind: ReceiptKind,
    pub origin: ReceiptOrigin,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub format: Option<String>,
    pub digest: Option<ByteDigest>,
    pub retrieval_uri: Option<String>,
    pub entry: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ByteDigest {
    pub algorithm: String,
    pub framing: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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
    HistoricalSnapshot {
        branch: String,
        rev: String,
        captured_at: String,
        evidence: String,
    },
    HistoricalRefHistory {
        branch: String,
        rev: String,
        valid_from: String,
        valid_until: String,
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
    #[serde(other)]
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ScopeProjection {
    pub requested: Scope,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AdmissionClaim {
    pub id: String,
    pub input_id: String,
    pub slot: Option<String>,
    pub receipt_ids: Vec<String>,
    pub projection: Option<ScopeProjection>,
    #[serde(default)]
    pub requires_selection: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AdmissionInput {
    pub binding: InputSlot,
    pub snapshot: SnapshotInput,
    pub dirty: Option<bool>,
    pub selection: Option<SelectionReceipt>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_scope: Option<Scope>,
    pub effective_scope: Option<Scope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<SelectionReceipt>,
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
    request: &AdmissionRequest,
    bytes: &BTreeMap<String, Vec<u8>>,
) -> AdmissionReport {
    Ledger::new(request, bytes).report()
}

impl Refusal {
    pub(crate) fn new(code: &str, record: &str) -> Self {
        Self {
            code: code.into(),
            dimension: None,
            record_id: Some(record.into()),
            expected: None,
            received: None,
            evidence: vec![record.into()],
        }
    }

    pub(crate) fn dimension(mut self, dimension: &str) -> Self {
        self.dimension = Some(dimension.into());
        self
    }

    fn mismatch(id: &str, dimension: &str, expected: Option<&str>, received: Option<&str>) -> Self {
        let mut refusal = Self::new("slot_mismatch", id).dimension(dimension);
        refusal.expected = expected.map(str::to_owned);
        refusal.received = received.map(str::to_owned);
        refusal
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.code)?;
        if let Some(id) = &self.record_id {
            write!(f, " at {id}")?;
        }
        if let Some(dimension) = &self.dimension {
            write!(f, " ({dimension})")?;
        }
        if let (Some(expected), Some(received)) = (&self.expected, &self.received) {
            write!(f, ", expected {expected}, received {received}")?;
        }
        Ok(())
    }
}

impl ClaimDecision {
    pub(crate) fn from_reasons(input_id: Option<&str>, reasons: Vec<Refusal>) -> Self {
        let mut by_code: BTreeMap<String, Refusal> = BTreeMap::new();
        for refusal in reasons {
            let existing = by_code
                .entry(refusal.code.clone())
                .or_insert_with(|| refusal.clone());
            for item in refusal.evidence {
                if !existing.evidence.contains(&item) {
                    existing.evidence.push(item);
                }
            }
        }
        let admitted = by_code.is_empty();
        Self {
            status: if admitted { "admitted" } else { "unchecked" }.into(),
            reasons: by_code.into_values().collect(),
            input_id: input_id.filter(|_| admitted).map(str::to_owned),
            requested_scope: None,
            effective_scope: None,
            selection: None,
            uncovered_scope: None,
            dirty: None,
            evidence: Vec::new(),
        }
    }
}

impl AdmissionReport {
    pub(crate) fn insert(&mut self, id: &str, decision: ClaimDecision) {
        if let Some(previous) = self.claims.get(id) {
            for reason in &previous.reasons {
                if let Some(count) = self.counts.get_mut(&reason.code) {
                    *count -= 1;
                    if *count == 0 {
                        self.counts.remove(&reason.code);
                    }
                }
            }
        }
        for reason in &decision.reasons {
            *self.counts.entry(reason.code.clone()).or_default() += 1;
        }
        self.claims.insert(id.into(), decision);
    }
}
