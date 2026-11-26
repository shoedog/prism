use super::shape::shape_limits;
use crate::diff::limits::Ledger;
use crate::diff::operations::operation_sites;
use crate::diff::*;
use crate::inventory::{BlockEffect, FieldBlock, WireEvidence};
use crate::{Coverage, Field, MessageCoverage, Operation};
use std::collections::BTreeMap;

#[derive(Default)]
pub(in crate::diff) struct OperationFields<'a>(BTreeMap<RootKey, FieldPaths<'a>>);

pub(super) type FieldPaths<'a> = BTreeMap<Vec<PointerToken>, FieldEvidence<'a>>;

#[derive(Clone, Copy)]
pub(super) struct FieldEvidence<'a> {
    pub(super) field: &'a Field,
    pub(super) comparable: bool,
}

impl<'a> OperationFields<'a> {
    pub fn insert(&mut self, root: RootKey, field: &'a Field, comparable: bool) {
        self.0
            .entry(root)
            .or_default()
            .entry(field.id.pointer.clone())
            .or_insert(FieldEvidence { field, comparable });
    }

    pub fn details(&self, op: &'a Operation, root: Option<&RootKey>) -> Vec<FieldDetail> {
        self.0
            .iter()
            .filter(|(key, _)| root.is_none_or(|root| root == *key))
            .flat_map(|(key, fields)| {
                let root = Root::new(op, self, key.clone());
                fields.values().map(move |entry| FieldDetail {
                    id: entry.field.id.clone(),
                    presence: if entry.comparable && root.authority_reason().is_none() {
                        Presence::Present
                    } else if root.declaration_only(&entry.field.id.pointer)
                        && root.presence_reason(&entry.field.id.pointer).is_none()
                    {
                        Presence::Absent
                    } else {
                        Presence::Unknown
                    },
                    shape: entry.field.shape.clone(),
                    sites: entry.field.sites.clone(),
                })
            })
            .collect()
    }
}

pub(in crate::diff) struct Root<'r, 'a> {
    pub op: &'a Operation,
    pub key: RootKey,
    pub message: Option<&'a MessageCoverage>,
    pub(super) paths: Option<&'r FieldPaths<'a>>,
    wire: Option<&'a WireEvidence>,
}

impl<'r, 'a> Root<'r, 'a> {
    pub fn new(op: &'a Operation, fields: &'r OperationFields<'a>, key: RootKey) -> Self {
        Self {
            op,
            paths: fields.0.get(&key),
            message: op.messages.iter().find(|message| message.key == key),
            wire: op.wire_evidence.get(&key),
            key,
        }
    }

    pub fn scope(&self) -> LimitScope {
        LimitScope::Root(self.op.id.clone(), self.key.clone())
    }

    pub fn sites(&self) -> Vec<SchemaSite> {
        operation_sites(self.op)
    }

    pub(super) fn lookup(&self, pointer: &[PointerToken]) -> Option<FieldEvidence<'a>> {
        self.paths.and_then(|paths| paths.get(pointer).copied())
    }

    fn block(&self, pointer: &[PointerToken]) -> Option<&FieldBlock> {
        self.blocks()
            .iter()
            .filter(|block| {
                pointer.starts_with(&block.pointer)
                    && (block.effect == BlockEffect::Field || pointer.len() > block.pointer.len())
            })
            .max_by_key(|block| block.pointer.len())
    }

    pub(super) fn blocks(&self) -> &[FieldBlock] {
        self.wire.map_or(&[], |wire| &wire.blocks)
    }

    pub(super) fn declaration_only(&self, pointer: &[PointerToken]) -> bool {
        self.wire
            .is_some_and(|wire| wire.declaration_only.contains(pointer))
    }

    pub(super) fn authority_reason(&self) -> Option<&str> {
        if self.key == RootKey::Request && !self.op.request_eligible {
            return Some("request_dialect_excluded");
        }
        if let Some(message) = self.message {
            if message.coverage == Coverage::Withheld
                || message.json_source.is_none() && message.coverage != Coverage::Checked
            {
                return message
                    .limits
                    .first()
                    .map(|limit| limit.reason.as_str())
                    .or(Some("field_presence_unchecked"));
            }
        } else if self.key != RootKey::Request || self.paths.is_some() {
            return Some("root_inventory_unavailable");
        }
        None
    }

    pub(super) fn presence_reason(&self, pointer: &[PointerToken]) -> Option<&str> {
        self.authority_reason()
            .or_else(|| self.block(pointer).map(|block| block.reason.as_str()))
    }

    fn standing(&self, side: ComparisonSide, ledger: &mut Ledger) {
        let Some(message) = self.message else { return };
        for region in &message.limits {
            let blocked = self
                .blocks()
                .iter()
                .any(|block| block.pointer == region.pointer && block.reason == region.reason);
            let claim = if blocked
                && message.json_source.is_some()
                && message.coverage != Coverage::Withheld
            {
                ComparisonClaim::FieldPresence
            } else {
                ComparisonClaim::Coverage
            };
            self.limit(side, ledger, &region.pointer, claim, &region.reason, None);
            if region.reason == "enum_unreadable" && self.lookup(&region.pointer).is_some() {
                self.limit(
                    side,
                    ledger,
                    &region.pointer,
                    ComparisonClaim::Property(ShapeSlot::Enum),
                    &region.reason,
                    None,
                );
            }
        }
        for entry in self
            .paths
            .into_iter()
            .flat_map(BTreeMap::values)
            .filter(|entry| entry.comparable && self.authority_reason().is_none())
        {
            shape_limits(self, entry.field, side, ledger);
        }
    }

    pub(super) fn limit(
        &self,
        side: ComparisonSide,
        ledger: &mut Ledger,
        pointer: &[PointerToken],
        claim: ComparisonClaim,
        reason: &str,
        presence: Option<&[ElementPresence; 2]>,
    ) {
        let mut limit = ComparisonLimit::new(side, self.scope(), claim, reason);
        limit.pointer = pointer.to_vec();
        limit.evidence = self
            .lookup(pointer)
            .map(|entry| entry.field.sites.clone())
            .filter(|sites| !sites.is_empty())
            .unwrap_or_else(|| self.sites());
        if let Some(presence) = presence {
            limit = limit.with_presence(presence);
        }
        ledger.record(limit);
    }
}

pub(in crate::diff) fn standing(
    op: &Operation,
    fields: &OperationFields<'_>,
    side: ComparisonSide,
    ledger: &mut Ledger,
) {
    for message in &op.messages {
        Root::new(op, fields, message.key.clone()).standing(side, ledger);
    }
}
