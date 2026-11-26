//! Structural comparison records for admitted boundary inventories.

mod producer;
pub use producer::*;

use crate::element::{CanonicalPath, FieldId, OperationId, PointerToken, SuccessKey};
use crate::{EnumValue, FieldShape, RootKey, SchemaIndex, SchemaSite};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComparisonScope {
    Api,
    PathPrefix(CanonicalPath),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffBasis<'a> {
    pub api: &'a str,
    pub authority: &'a str,
    pub tree: &'a str,
    pub scope: ComparisonScope,
}

#[derive(Default)]
pub struct DiffSide<'a> {
    /// An explicit admitted slot when the inventory cannot supply the basis by itself.
    pub basis: Option<DiffBasis<'a>>,
    pub schema: Option<&'a SchemaIndex>,
    pub producer: Option<&'a ProducerInventory<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    OpAdded,
    OpRemoved,
    RouteDeclarationChanged,
    ValidationChanged,
    HandlerReferenceChanged,
    HandlerCodeChanged,
    SuccessStatusAdded,
    SuccessStatusRemoved,
    FieldAdded,
    FieldRemoved,
    FieldTypeChanged,
    FieldFormatChanged,
    FieldItemTypeChanged,
    FieldRequiredAdded,
    FieldRequiredRemoved,
    EnumDeclarationChanged,
    EnumValuesAdded,
    EnumValuesRemoved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElementId {
    Operation(OperationId),
    Status(OperationId, SuccessKey),
    Field(FieldId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug)]
pub struct ElementPresence {
    pub id: Option<ElementId>,
    pub presence: Presence,
}

#[derive(Debug)]
pub struct FieldDetail {
    pub id: FieldId,
    pub presence: Presence,
    pub shape: FieldShape,
    pub sites: Vec<SchemaSite>,
}

#[derive(Debug)]
pub struct ChangeSide<'a> {
    pub element: ElementPresence,
    /// A retained field declaration can carry evidence even when its wire domain is empty.
    pub declaration_id: Option<FieldId>,
    pub shape: Option<FieldShape>,
    pub sites: Vec<SchemaSite>,
    pub success_keys: Vec<SuccessKey>,
    /// Field detail owned by a whole-operation or unmatched status change.
    pub fields: Vec<FieldDetail>,
    pub producer: Option<&'a ProducerOperation<'a>>,
}

#[derive(Debug)]
pub struct ElementChange<'a> {
    pub operation: OperationId,
    pub kinds: Vec<ChangeKind>,
    pub before: ChangeSide<'a>,
    pub after: ChangeSide<'a>,
    pub enum_added: Vec<EnumValue>,
    pub enum_removed: Vec<EnumValue>,
}

#[derive(Debug)]
pub struct FieldPair {
    pub before: FieldId,
    pub after: FieldId,
}

#[derive(Debug)]
pub struct ResponseRootPair {
    pub operation: OperationId,
    pub before: SuccessKey,
    pub after: SuccessKey,
    pub fields: Vec<FieldPair>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonSide {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeSlot {
    Type,
    Format,
    Required,
    Enum,
    ItemType,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ComparisonClaim {
    OperationPresence,
    StatusMembership,
    RootPairing,
    FieldPresence,
    Property(ShapeSlot),
    Producer(ProducerSlot),
    Coverage,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LimitScope {
    Schema,
    Operation(Option<OperationId>),
    Status(OperationId, Option<SuccessKey>),
    Root(OperationId, RootKey),
}

#[derive(Debug)]
pub struct PresenceComparison {
    pub before: ElementPresence,
    pub after: ElementPresence,
}

#[derive(Debug)]
pub struct ComparisonLimit {
    pub side: ComparisonSide,
    pub scope: LimitScope,
    pub pointer: Vec<PointerToken>,
    pub claim: ComparisonClaim,
    pub reason: String,
    pub presence: Option<PresenceComparison>,
    pub evidence: Vec<SchemaSite>,
    pub producer_sites: Vec<ProducerSite>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LimitCounts {
    pub before: usize,
    pub after: usize,
}

#[derive(Debug, Default)]
pub struct SchemaDiff<'a> {
    pub changes: Vec<ElementChange<'a>>,
    pub response_root_pairs: Vec<ResponseRootPair>,
    pub limits: Vec<ComparisonLimit>,
    pub limit_counts: LimitCounts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffError {
    IncompatibleApi,
    IncompatibleAuthority,
    ContradictorySlot,
    NoCommonScope,
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::IncompatibleApi => "comparison APIs differ",
            Self::IncompatibleAuthority => "comparison operation authorities differ",
            Self::ContradictorySlot => "comparison source contradicts its input slot",
            Self::NoCommonScope => "no common admitted comparison scope",
        })
    }
}

impl std::error::Error for DiffError {}

pub fn diff_schemas<'a>(
    before: &'a SchemaIndex,
    after: &'a SchemaIndex,
) -> Result<SchemaDiff<'a>, DiffError> {
    diff_boundary(
        &DiffSide {
            schema: Some(before),
            ..DiffSide::default()
        },
        &DiffSide {
            schema: Some(after),
            ..DiffSide::default()
        },
    )
}

pub fn diff_boundary<'a>(
    _before: &DiffSide<'a>,
    _after: &DiffSide<'a>,
) -> Result<SchemaDiff<'a>, DiffError> {
    // Compile-ready default for the P1c RED tests.
    Ok(SchemaDiff::default())
}
