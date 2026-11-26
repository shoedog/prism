use super::{AdmissionRequest, Refusal, Scope};
use crate::SchemaIndex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize)]
pub struct SavedScan {
    pub scan_id: String,
    pub client_input_id: String,
    pub client_tree: String,
    pub api: String,
    pub authority: String,
    pub source_tree: String,
    pub source_hash: String,
    pub config_era: String,
    pub schema_inventory_id: String,
    pub expected_segments: Vec<String>,
    pub admitted_segments: Vec<String>,
    pub dependencies: Vec<String>,
    pub matches: Vec<SavedLocation>,
    pub scope: Option<Scope>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SavedLocation {
    pub location_id: String,
    pub element_id: String,
    pub grade: String,
    pub signature: Value,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceSide {
    Before,
    After,
    Held,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SavedQuery {
    pub element_id: String,
    pub selected_input_id: String,
    pub side: ServiceSide,
    pub saved_location: Option<String>,
}

pub struct SavedScanEvidence<'a> {
    pub admission: &'a AdmissionRequest,
    pub bytes: &'a BTreeMap<String, Vec<u8>>,
    pub scan: Option<&'a SavedScan>,
    pub inventory: Option<&'a SchemaIndex>,
    pub selected: &'a SchemaIndex,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BindingStatus {
    pub scan_id: Option<String>,
    pub element_id: String,
    pub selected_input_id: String,
    pub side: ServiceSide,
    pub saved_location: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    pub selected_presence: String,
    pub refusal: Option<Refusal>,
    pub saved_shape: Option<Value>,
    pub selected_shape: Option<Value>,
}

#[derive(Debug, Default, Serialize)]
pub struct SavedStatusReport {
    pub binding_status: Vec<BindingStatus>,
    pub standing_mismatches: Vec<Value>,
    pub admitted_raw_inputs: Vec<String>,
    pub admitted_saved_locations: Vec<String>,
}

pub fn saved_binding_status(
    _query: &SavedQuery,
    _evidence: &SavedScanEvidence<'_>,
) -> SavedStatusReport {
    SavedStatusReport::default()
}
