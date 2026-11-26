//! Public schema inventory types. The P1b reader fills these from admitted inputs.

use crate::element::{FieldId, OperationId, PointerToken, SuccessKey};
use crate::revision::AdmittedSchemaInput;
pub use crate::schema::site::{Diagnostic as SchemaDiagnostic, Site as SchemaSite};
use crate::schema::SchemaError;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct SchemaIndex {
    pub operations: Vec<Operation>,
    pub fields: Vec<Field>,
    pub observed_fields: Vec<Field>,
    pub limits: Vec<RegionLimit>,
    pub warnings: Vec<String>,
    pub diagnostics: Vec<SchemaDiagnostic>,
    pub source: Option<SchemaSource>,
    pub info_version: Option<String>,
    pub byte_sha256: String,
    pub content_hash: String,
    pub(crate) reached: BTreeMap<OperationId, (bool, Vec<DefinitionId>)>,
    pub(crate) inverse: BTreeMap<DefinitionId, Vec<OperationId>>,
    pub(crate) operation_coverage_incomplete: bool,
    pub(crate) public_fields: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SchemaSource {
    pub api: String,
    pub authority: String,
    pub tree: String,
    pub commit: Option<String>,
    pub path: String,
    pub blob: String,
}

#[derive(Debug)]
pub struct Operation {
    pub id: OperationId,
    pub success_keys: Vec<SuccessKey>,
    pub response_keys: Vec<String>,
    pub messages: Vec<MessageCoverage>,
    pub operation_id: Option<String>,
    pub declarations: Vec<OperationSite>,
    pub(crate) ranged_responses: bool,
}

#[derive(Debug)]
pub struct OperationSite {
    pub path: String,
    pub canonical_path: crate::element::CanonicalPath,
    pub method: String,
    pub site: SchemaSite,
    pub referring_site: SchemaSite,
    pub reference_chain: Vec<SchemaSite>,
}

impl Operation {
    pub fn response_key(&self, code: u16) -> Option<&str> {
        if !(100..=599).contains(&code) {
            return None;
        }
        let find = |key: &str| {
            self.response_keys
                .binary_search_by(|value| value.as_str().cmp(key))
                .ok()
                .map(|i| self.response_keys[i].as_str())
        };
        find(&code.to_string())
            .or_else(|| {
                self.ranged_responses
                    .then(|| find(&format!("{}XX", code / 100)))
                    .flatten()
            })
            .or_else(|| find("default"))
    }
}

#[derive(Debug)]
pub struct Field {
    pub id: FieldId,
    pub shape: FieldShape,
    pub definitions: Vec<DefinitionId>,
    pub sites: Vec<SchemaSite>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootKey {
    Request,
    Response(SuccessKey),
    Fallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    Checked,
    ObservedOnly,
    Partial,
    Withheld,
}

#[derive(Debug)]
pub struct MessageCoverage {
    pub key: RootKey,
    pub coverage: Coverage,
    pub limits: Vec<RegionLimit>,
    pub feasible: Option<bool>,
    pub declared_media: Vec<String>,
    pub json_source: Option<JsonMediaSource>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum JsonMediaSource {
    MessageContent,
    OperationMedia,
    DocumentMedia,
    Producer { input_id: String, entry: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub struct RegionLimit {
    pub pointer: Vec<PointerToken>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefinitionId {
    pub resource: String,
    pub pointer: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Lookup<'a, T> {
    Checked(&'a [T]),
    Partial(&'a [T]),
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Property<T> {
    Missing,
    Known(T),
    BranchDependent,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnumValue {
    Null,
    Boolean(bool),
    Number(String),
    String(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldShape {
    pub field_type: Property<Vec<String>>,
    pub format: Property<String>,
    pub required: Property<bool>,
    pub enum_values: Property<Vec<EnumValue>>,
    pub item_type: Property<Vec<String>>,
}

pub fn index_schema(input: &AdmittedSchemaInput<'_>) -> Result<SchemaIndex, SchemaError> {
    crate::schema::project(input)
}

pub fn definitions_of<'a>(
    index: &'a SchemaIndex,
    operation: &OperationId,
) -> Lookup<'a, DefinitionId> {
    match index.reached.get(operation) {
        Some((false, definitions)) => Lookup::Checked(definitions),
        Some((true, definitions)) => Lookup::Partial(definitions),
        None => Lookup::Unknown,
    }
}

pub fn operations_reaching<'a>(
    index: &'a SchemaIndex,
    definition: &DefinitionId,
) -> Lookup<'a, OperationId> {
    let Some(operations) = index.inverse.get(definition) else {
        return Lookup::Unknown;
    };
    let incomplete = index.operation_coverage_incomplete
        || index
            .reached
            .iter()
            .any(|(id, (partial, _))| *partial && operations.binary_search(id).is_err());
    if incomplete {
        Lookup::Partial(operations)
    } else {
        Lookup::Checked(operations)
    }
}

pub use crate::schema::signature;

impl SchemaIndex {
    /// Canonical public projection, as UTF-8 lines with operations before fields.
    pub fn public_ids_tsv(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for line in self
            .operations
            .iter()
            .map(|op| op.id.to_string())
            .chain(self.public_fields.iter().cloned())
        {
            bytes.extend_from_slice(line.as_bytes());
            bytes.push(b'\n');
        }
        bytes
    }
}
