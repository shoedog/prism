use super::basis;
use super::limits::Ledger;
use super::*;
use crate::element::{CanonicalPath, OperationId};
use crate::revision::DependencyReceipt;
use crate::Coverage;
use std::collections::BTreeMap;
use std::ops::Range;

#[derive(Debug)]
pub struct ProducerInventory<'a> {
    pub basis: DiffBasis<'a>,
    pub coverage: Coverage,
    pub operations: Vec<ProducerOperation<'a>>,
}

#[derive(Debug)]
pub struct ProducerOperation<'a> {
    pub operation: OperationId,
    pub declaration: ProducerFact<'a, RouteDeclaration>,
    pub validation: ProducerFact<'a, Validator>,
    pub reference: ProducerFact<'a, HandlerReference>,
    pub code: ProducerFact<'a, HandlerBody>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProducerSlot {
    Declaration,
    Validation,
    Reference,
    Code,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProducerSiteKind {
    Declaration,
    Registration,
    Annotation,
    Validation,
    Reference,
    Endpoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducerSite {
    pub kind: ProducerSiteKind,
    pub tree: String,
    pub file: String,
    pub blob: String,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub enum Fact<T> {
    Checked(Option<T>),
    Withheld { reason: String },
}

#[derive(Debug)]
pub struct ProducerFact<'a, T> {
    pub fact: Fact<T>,
    pub sites: Vec<ProducerSite>,
    pub dependencies: Vec<DependencyReceipt<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RouteDeclaration {
    pub original_method: String,
    pub original_path: String,
    pub canonical_path: CanonicalPath,
    pub attributes: BTreeMap<String, AttributeValue>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttributeValue {
    Boolean(bool),
    Integer(i64),
    String(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Validator {
    pub identity: Option<String>,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandlerEndpoint {
    Resolved(String),
    Inline,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HandlerReference {
    pub expression: String,
    pub endpoint: HandlerEndpoint,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HandlerBody {
    pub endpoint: HandlerEndpoint,
    pub digest: String,
}

pub(super) fn path<'a>(op: &'a ProducerOperation<'_>) -> Option<&'a CanonicalPath> {
    match &op.declaration.fact {
        Fact::Checked(Some(declaration)) => Some(&declaration.canonical_path),
        _ => None,
    }
}

pub(super) fn in_scope(op: &ProducerOperation<'_>, scope: &ComparisonScope) -> bool {
    matches!(scope, ComparisonScope::Api)
        || path(op).is_some_and(|path| basis::in_scope(path, scope))
}

pub(super) fn compare<'a>(
    diff: &mut SchemaDiff<'a>,
    ledger: &mut Ledger,
    id: &OperationId,
    sides: [ChangeSide<'a>; 2],
    inventories: [Option<&ProducerInventory<'a>>; 2],
) {
    if inventories.iter().all(Option::is_none) {
        return;
    }
    let operations = sides.each_ref().map(|side| side.producer);
    let mut change = ElementChange::new(id, sides);
    let mut compare = Compare {
        id,
        inventories,
        schema_sites: [&change.before.sites, &change.after.sites],
        ledger,
        kinds: &mut change.kinds,
    };
    compare.fact(
        operations,
        ProducerSlot::Declaration,
        ChangeKind::RouteDeclarationChanged,
        |op| &op.declaration,
        |b, a| Some(b != a),
    );
    compare.fact(
        operations,
        ProducerSlot::Validation,
        ChangeKind::ValidationChanged,
        |op| &op.validation,
        |b, a| {
            let ids = b.identity.as_ref().zip(a.identity.as_ref());
            let digests = b.digest.as_ref().zip(a.digest.as_ref());
            (ids.is_some() || digests.is_some())
                .then(|| ids.is_some_and(|(b, a)| b != a) || digests.is_some_and(|(b, a)| b != a))
        },
    );
    compare.fact(
        operations,
        ProducerSlot::Reference,
        ChangeKind::HandlerReferenceChanged,
        |op| &op.reference,
        |b, a| Some(b.endpoint != a.endpoint),
    );
    compare.fact(
        operations,
        ProducerSlot::Code,
        ChangeKind::HandlerCodeChanged,
        |op| &op.code,
        |b, a| {
            (b.endpoint == a.endpoint && !b.digest.is_empty() && !a.digest.is_empty())
                .then(|| b.digest != a.digest)
        },
    );
    if !change.kinds.is_empty() {
        diff.changes.push(change);
    }
}

struct Compare<'r, 'a> {
    id: &'r OperationId,
    inventories: [Option<&'r ProducerInventory<'a>>; 2],
    schema_sites: [&'r [SchemaSite]; 2],
    ledger: &'r mut Ledger,
    kinds: &'r mut Vec<ChangeKind>,
}

impl<'a> Compare<'_, 'a> {
    fn fact<T: 'a>(
        &mut self,
        operations: [Option<&'a ProducerOperation<'a>>; 2],
        slot: ProducerSlot,
        kind: ChangeKind,
        get: impl Fn(&'a ProducerOperation<'a>) -> &'a ProducerFact<'a, T>,
        differs: impl FnOnce(&T, &T) -> Option<bool>,
    ) {
        let records = operations.map(|op| op.map(&get));
        let mut values = [None, None];
        for i in 0..2 {
            let evidence = records[i].map_or(&[][..], |record| record.sites.as_slice());
            let reason = match (self.inventories[i], records[i]) {
                (None, _) => "producer_unavailable",
                (Some(inventory), _) if inventory.coverage == Coverage::Withheld => {
                    "producer_unavailable"
                }
                (Some(_), None) => "producer_fact_unavailable",
                (Some(inventory), Some(record)) => match &record.fact {
                    Fact::Withheld { reason } => reason,
                    Fact::Checked(_)
                        if record.sites.is_empty()
                            || record
                                .sites
                                .iter()
                                .any(|site| site.tree != inventory.basis.tree) =>
                    {
                        "producer_evidence_unbound"
                    }
                    Fact::Checked(value) => {
                        values[i] = Some(value.as_ref());
                        continue;
                    }
                },
            };
            self.limit(i, slot, reason, evidence);
        }
        if let [Some(b), Some(a)] = values {
            let comparison = match (b, a) {
                (Some(b), Some(a)) => differs(b, a),
                (None, None) => Some(false),
                _ if slot == ProducerSlot::Code => None,
                _ => Some(true),
            };
            match comparison {
                Some(true) => self.kinds.push(kind),
                Some(false) => (),
                None => {
                    let reason = match slot {
                        ProducerSlot::Code => "endpoint_body_incomparable",
                        _ => "validator_incomparable",
                    };
                    for (i, record) in records.iter().enumerate() {
                        self.limit(i, slot, reason, &record.unwrap().sites);
                    }
                }
            }
        }
    }

    fn limit(&mut self, i: usize, slot: ProducerSlot, reason: &str, evidence: &[ProducerSite]) {
        let mut limit = ComparisonLimit::new(
            SIDES[i],
            LimitScope::Operation(Some(self.id.clone())),
            ComparisonClaim::Producer(slot),
            reason,
        );
        limit.producer_sites.extend(
            evidence
                .iter()
                .filter(|site| {
                    self.inventories[i].is_some_and(|inventory| site.tree == inventory.basis.tree)
                })
                .cloned(),
        );
        if limit.producer_sites.is_empty() {
            limit.evidence.extend_from_slice(self.schema_sites[i]);
        }
        self.ledger.record(limit);
    }
}
