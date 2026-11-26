//! Public schema inventory types. The P1b reader fills these from admitted inputs.

use crate::element::{FieldId, OperationId, PointerToken, SuccessKey};
use crate::revision::AdmittedSchemaInput;
use crate::SchemaError;

#[derive(Debug, Default)]
pub struct SchemaIndex {
    pub operations: Vec<Operation>,
    pub fields: Vec<Field>,
    pub observed_fields: Vec<Field>,
    pub limits: Vec<RegionLimit>,
    pub warnings: Vec<String>,
    pub source: Option<SchemaSource>,
    pub info_version: Option<String>,
    pub byte_sha256: String,
    pub content_hash: String,
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
}

impl Operation {
    pub fn response_key(&self, _code: u16) -> Option<&str> {
        None
    }
}

#[derive(Debug)]
pub struct Field {
    pub id: FieldId,
    pub shape: FieldShape,
    pub definitions: Vec<DefinitionId>,
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
    pub field_type: Property<String>,
    pub format: Property<String>,
    pub required: Property<bool>,
    pub enum_values: Property<Vec<EnumValue>>,
    pub item_type: Property<String>,
}

pub fn index_schema(_input: &AdmittedSchemaInput<'_>) -> Result<SchemaIndex, SchemaError> {
    Ok(SchemaIndex::default())
}

pub fn definitions_of<'a>(
    _index: &'a SchemaIndex,
    _operation: &OperationId,
) -> Lookup<'a, DefinitionId> {
    Lookup::Unknown
}

pub fn operations_reaching<'a>(
    _index: &'a SchemaIndex,
    _definition: &DefinitionId,
) -> Lookup<'a, OperationId> {
    Lookup::Unknown
}

pub fn signature(_shape: &FieldShape) -> String {
    String::new()
}

impl SchemaIndex {
    /// Canonical public projection, as UTF-8 lines with operations before fields.
    pub fn public_ids_tsv(&self) -> Vec<u8> {
        Vec::new()
    }
}
