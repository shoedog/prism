use super::{DocumentError, DocumentInput, Scope, SourceSite, WireSignature};
use crate::revision::{AdmissionReport, BindingStatus, ReviewMode};
use crate::SchemaDiff;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ImpactDocument {
    pub schema_version: String,
    pub report_id: String,
    pub api: String,
    pub mode: Option<ReviewMode>,
    pub report_time: String,
    pub cutoff_at: String,
    pub inputs: Vec<DocumentInput>,
    pub reference: Option<String>,
    pub comparisons: Vec<Comparison>,
    pub change: Change,
    pub impacts: Vec<ElementChangeRecord>,
    pub usages: Vec<Usage>,
    pub binding_status: Vec<BindingStatus>,
    pub coverage: Vec<CoverageLimit>,
    pub warnings: Vec<Value>,
    pub standing_mismatches: Vec<Value>,
    pub summary: BTreeMap<String, Metric>,
    pub unobserved: Vec<Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Change {
    pub subject_role: String,
    pub producer: Option<RoleSlots>,
    pub consumer: Option<RoleSlots>,
    pub elements: Vec<ElementChangeRecord>,
    pub status_additions: Vec<Value>,
    pub status_removals: Vec<Value>,
    pub response_root_pairs: Vec<RootPair>,
    pub call_changes: Vec<Value>,
    pub producer_sites: Vec<SourceSite>,
    pub risks: Vec<Value>,
    pub review_action: Option<Value>,
    pub selection: Option<Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoleSlots {
    pub before_input_id: Option<String>,
    pub after_input_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Comparison {
    pub comparison_id: String,
    pub purpose: String,
    pub service_input_id: String,
    pub client_input_id: String,
    pub matches: BTreeMap<String, MatchRecord>,
    pub unchecked_count: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MatchRecord {
    pub service_input_id: String,
    pub client_input_id: String,
    pub status: String,
    pub grade: Option<String>,
    pub reason: Option<String>,
    pub mapping: Option<Value>,
    pub target_ids: Vec<String>,
    pub alternates: Vec<Value>,
    pub overlaps: Vec<Value>,
    pub prefix_hints: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MatchRef {
    pub comparison_id: String,
    pub site_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Usage {
    pub site_id: String,
    pub before_site: Option<SourceSite>,
    pub after_site: Option<SourceSite>,
    pub before_presence: String,
    pub after_presence: String,
    pub usage_id: Option<String>,
    pub correspondence_evidence: Option<Value>,
    pub requested_target: Value,
    pub match_before: Option<MatchRef>,
    pub match_after: Option<MatchRef>,
    pub match_service_first: Option<MatchRef>,
    pub match_client_first: Option<MatchRef>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ElementChangeRecord {
    pub row_id: String,
    pub operation_id: String,
    pub before_element_id: Option<String>,
    pub after_element_id: Option<String>,
    pub change_kinds: Vec<String>,
    pub before_presence: String,
    pub after_presence: String,
    pub before_shape: Option<WireSignature>,
    pub after_shape: Option<WireSignature>,
    pub before_declaration_id: Option<String>,
    pub after_declaration_id: Option<String>,
    pub coverage_limits: Vec<String>,
    #[serde(default)]
    pub before_fields: Vec<Value>,
    #[serde(default)]
    pub after_fields: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RootPair {
    pub operation_id: String,
    pub before_status: String,
    pub after_status: String,
    pub fields: Vec<FieldPair>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FieldPair {
    pub before_element_id: String,
    pub after_element_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CoverageLimit {
    pub limit_id: String,
    pub comparison_id: String,
    pub input_id: String,
    pub side: String,
    pub authority: String,
    pub scope: Value,
    pub pointer: Vec<Value>,
    pub claim: String,
    pub reason: String,
    pub source: String,
    pub evidence: Vec<Value>,
    pub affected_claims: Vec<String>,
    pub presence: Option<Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Metric {
    pub version: String,
    pub scope: Scope,
    pub observed: usize,
    pub total: Option<usize>,
    pub reason: Option<String>,
}

pub struct MembershipRequest<'a> {
    pub change: &'a ElementChangeRecord,
    pub expected_inputs: &'a [String],
    pub admitted_bindings: &'a [super::BindingsDocument],
    pub membership_evidence: &'a [Value],
}

pub fn unobserved_elements(_request: &MembershipRequest<'_>) -> Value {
    Value::Null
}

impl ImpactDocument {
    pub fn from_json(_bytes: &[u8]) -> Result<Self, DocumentError> {
        Ok(Self::default())
    }

    pub fn validate(&self) -> AdmissionReport {
        AdmissionReport::default()
    }

    pub fn with_schema_diff(self, _diff: &SchemaDiff<'_>) -> Self {
        self
    }

    pub fn with_usage_counts(self, _bindings: &[super::BindingsDocument]) -> Self {
        self
    }

    pub fn with_coverage_limits(self, _limits: &[CoverageLimit]) -> Self {
        self
    }
}

impl Serialize for ImpactDocument {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}
