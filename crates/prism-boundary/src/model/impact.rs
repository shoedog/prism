use super::{DocumentError, DocumentInput, Scope, SourceSite, WireSignature};
use super::{FieldRecord, Scalar};
use crate::revision::{AdmissionReport, BindingStatus, ReviewMode};
use crate::SchemaDiff;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod membership;
mod projection;
mod validation;
pub use membership::unobserved_elements;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
    pub warnings: Vec<UncheckedClaim>,
    pub standing_mismatches: Vec<StandingMismatch>,
    pub summary: BTreeMap<String, Metric>,
    pub unobserved: Vec<UnobservedElement>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Change {
    pub subject_role: String,
    pub producer: Option<RoleSlots>,
    pub consumer: Option<RoleSlots>,
    pub elements: Vec<ElementChangeRecord>,
    pub status_additions: Vec<StatusEdit>,
    pub status_removals: Vec<StatusEdit>,
    pub response_root_pairs: Vec<RootPair>,
    pub call_changes: Vec<Value>,
    pub producer_sites: Vec<SourceSite>,
    pub risks: Vec<Value>,
    pub review_action: Option<ReviewAction>,
    pub selection: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoleSlots {
    pub before_input_id: Option<String>,
    pub after_input_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Comparison {
    pub comparison_id: String,
    pub purpose: String,
    pub service_input_id: String,
    pub client_input_id: String,
    pub matches: BTreeMap<String, MatchRecord>,
    pub unchecked_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchRef {
    pub comparison_id: String,
    pub site_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Usage {
    pub site_id: String,
    pub before_site: Option<SourceSite>,
    pub after_site: Option<SourceSite>,
    pub before_presence: String,
    pub after_presence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correspondence_evidence: Option<Value>,
    pub requested_target: Value,
    pub match_before: Option<MatchRef>,
    pub match_after: Option<MatchRef>,
    pub match_service_first: Option<MatchRef>,
    pub match_client_first: Option<MatchRef>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_declaration_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_declaration_id: Option<String>,
    pub coverage_limits: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_added: Vec<Scalar>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_removed: Vec<Scalar>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before_fields: Vec<FieldRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after_fields: Vec<FieldRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RootPair {
    pub operation_id: String,
    pub before_status: String,
    pub after_status: String,
    pub fields: Vec<FieldPair>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FieldPair {
    pub before_element_id: String,
    pub after_element_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoverageLimit {
    pub limit_id: String,
    pub comparison_id: String,
    pub input_id: String,
    pub side: String,
    pub authority: String,
    pub scope: LimitRegion,
    pub pointer: Vec<WirePointer>,
    pub claim: String,
    pub reason: String,
    pub source: String,
    pub evidence: Vec<Value>,
    pub affected_claims: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReviewAction {
    pub claim_id: String,
    pub comparison_id: String,
    pub evidence: Vec<MatchRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supporting_row_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatusEdit {
    pub operation_id: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UncheckedClaim {
    pub claim_id: String,
    pub status: String,
    pub coverage_limits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StandingMismatch {
    SchemaMismatch {
        saved_content_hash: String,
        selected_content_hash: String,
        saved_info_version: Option<String>,
        selected_info_version: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LimitRegion {
    Schema {
        #[serde(skip_serializing_if = "Option::is_none")]
        path_scope: Option<Scope>,
    },
    Operation {
        operation_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path_scope: Option<Scope>,
    },
    Status {
        operation_id: String,
        status: Option<String>,
    },
    Root {
        operation_id: String,
        direction: String,
        status: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WirePointer {
    Property { name: String },
    ListItem,
    MapValue,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnobservedElement {
    pub before_element_id: Option<String>,
    pub after_element_id: Option<String>,
    pub membership_basis: Vec<String>,
}

impl ImpactDocument {
    pub fn from_json(bytes: &[u8]) -> Result<Self, DocumentError> {
        super::decode::document(bytes, super::decode::Kind::Impact)
    }

    pub fn validate(&self) -> AdmissionReport {
        validation::validate(self)
    }
}
