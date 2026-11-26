use super::{DocumentError, DocumentInput, EvidenceRecord, ExtractionRegion, SourceSite};
use crate::SchemaIndex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct BindingsDocument {
    pub schema_version: String,
    #[serde(flatten)]
    pub input: Option<DocumentInput>,
    pub expected_files: Vec<String>,
    pub file_status: Vec<ExtractionRegion>,
    pub entries: Vec<ExtractionRegion>,
    pub unaccounted_scopes: Vec<ExtractionRegion>,
    pub dependency_records: Vec<EvidenceRecord>,
    pub operations: Option<Vec<Binding>>,
    pub calls: Option<Vec<Unbound>>,
    pub saved_scan: Option<Value>,
}

#[derive(Clone, Debug, Deserialize)]
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
    pub message_roots: Vec<Value>,
    #[serde(default)]
    pub fields: Vec<Value>,
    #[serde(default)]
    pub observed_fields: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize)]
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
    pub saved_match: Option<Value>,
}

impl BindingsDocument {
    pub fn from_json(_bytes: &[u8]) -> Result<Self, DocumentError> {
        Ok(Self::default())
    }

    pub fn with_schema_inventory(self, _inventory: &SchemaIndex) -> Self {
        self
    }

    pub fn with_extraction_coverage(self) -> Self {
        self
    }
}

impl Serialize for BindingsDocument {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}
