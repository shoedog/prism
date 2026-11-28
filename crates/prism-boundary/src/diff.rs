//! P1c structural diff claims.
//! Compare admitted revisions with a common API, operation authority, and logical scope.
//! Report whole-operation edits with their own statuses and observed field detail.
//! Report explicit success-key edits independently of response-body eligibility.
//! Pair unique proved JSON replacements and preserve both original root and field IDs.
//! Keep schema-bearing responses with unproved media eligible for pairing limits even when their field inventories are empty.
//! Require proved media or checked body absence for a retained root to establish named field absence.
//! Record side-local coverage for authored requests excluded by their dialect.
//! Compare named JSON field presence on checked evidence and compare all five signed shape slots.
//! Claim a type or item-type change only for slots with no reader limit at their typed pointers.
//! Withhold every other type or item-type comparison and count a side-local slot limit.
//! Whole-message refusals and opaque ancestors can leave descendants without signed slots.
//! Keep declaration and member deltas independently for readable enum evidence.
//! Preserve side-local unknowns, scope, reasons, evidence, and deduplicated limit counts.
//! Report independently proved producer fact changes through typed revision-local inputs.
//! Establish producer-only operation presence from revision-bound declaration evidence.
//! Treat a producer declaration with an unavailable path as a method-wide coverage gap under a path prefix.
//! Preserve unchanged structural facts through formatting, definition renames, and equivalent projections.
//! Stay silent on value-only, unsigned union const, and unsigned item/map facts within P1b's known limits.

mod basis;
mod fields;
pub(crate) use fields::field_detail_presence;
mod limits;
mod operations;
mod producer;
mod status;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComparisonSide {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShapeSlot {
    Type,
    Format,
    Required,
    Enum,
    ItemType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComparisonClaim {
    OperationPresence,
    StatusMembership,
    RootPairing,
    FieldPresence,
    Property(ShapeSlot),
    Producer(ProducerSlot),
    Coverage,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LimitScope {
    Schema,
    Operation(Option<OperationId>),
    Status(OperationId, Option<SuccessKey>),
    Root(OperationId, RootKey),
}

#[derive(Clone, Debug)]
pub struct PresenceComparison {
    pub before: ElementPresence,
    pub after: ElementPresence,
}

#[derive(Debug)]
pub struct ComparisonLimit {
    pub origin: LimitOrigin,
    pub side: ComparisonSide,
    pub scope: LimitScope,
    pub pointer: Vec<PointerToken>,
    pub claim: ComparisonClaim,
    pub reason: String,
    pub presence: Option<PresenceComparison>,
    pub evidence: Vec<SchemaSite>,
    pub producer_sites: Vec<ProducerSite>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitOrigin {
    InputInventory,
    Comparison,
    Both,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LimitCounts {
    pub before: usize,
    pub after: usize,
}

#[derive(Debug, Default)]
pub struct SchemaDiff<'a> {
    pub(crate) bases: Option<[DiffBasis<'a>; 2]>,
    pub(crate) schema_sources: [Option<&'a crate::inventory::SchemaSource>; 2],
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
    before: &DiffSide<'a>,
    after: &DiffSide<'a>,
) -> Result<SchemaDiff<'a>, DiffError> {
    let before = basis::BoundSide::admit(before)?;
    let after = basis::BoundSide::admit(after)?;
    let scope = basis::common_scope(&before.basis, &after.basis)?;
    Ok(operations::compare([before, after], &scope))
}

const SIDES: [ComparisonSide; 2] = [ComparisonSide::Before, ComparisonSide::After];

impl ElementPresence {
    fn present(id: ElementId) -> Self {
        Self {
            id: Some(id),
            presence: Presence::Present,
        }
    }

    fn without_id(presence: Presence) -> Self {
        Self { id: None, presence }
    }
}

impl<'a> ChangeSide<'a> {
    fn new(element: ElementPresence) -> Self {
        Self {
            element,
            declaration_id: None,
            shape: None,
            sites: Vec::new(),
            success_keys: Vec::new(),
            fields: Vec::new(),
            producer: None,
        }
    }
}

impl<'a> ElementChange<'a> {
    fn new(operation: &OperationId, [before, after]: [ChangeSide<'a>; 2]) -> Self {
        Self {
            operation: operation.clone(),
            kinds: Vec::new(),
            before,
            after,
            enum_added: Vec::new(),
            enum_removed: Vec::new(),
        }
    }
}
