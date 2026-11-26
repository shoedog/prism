use super::*;
use crate::element::PointerToken;
use crate::{
    ChangeKind, ComparisonClaim, ComparisonSide, ElementId, LimitScope, Presence, ProducerSlot,
    RootKey, ShapeSlot,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

impl ImpactDocument {
    pub fn with_schema_diff(mut self, diff: &SchemaDiff<'_>) -> Self {
        if !diff_matches_slots(&self, diff) {
            return self;
        }
        self.change.elements.clear();
        self.change.status_additions.clear();
        self.change.status_removals.clear();
        for change in &diff.changes {
            let row = element_row(change);
            for (expected, side, edits) in [
                (
                    ChangeKind::SuccessStatusAdded,
                    &change.after,
                    &mut self.change.status_additions,
                ),
                (
                    ChangeKind::SuccessStatusRemoved,
                    &change.before,
                    &mut self.change.status_removals,
                ),
            ] {
                if change.kinds.contains(&expected) {
                    if let Some(ElementId::Status(_, status)) = &side.element.id {
                        edits.push(StatusEdit {
                            operation_id: row.operation_id.clone(),
                            status: status.to_string(),
                        });
                    }
                }
            }
            self.change.elements.push(row);
        }
        self.change.response_root_pairs = diff
            .response_root_pairs
            .iter()
            .map(|pair| RootPair {
                operation_id: pair.operation.to_string(),
                before_status: pair.before.to_string(),
                after_status: pair.after.to_string(),
                fields: pair
                    .fields
                    .iter()
                    .map(|field| FieldPair {
                        before_element_id: field.before.to_string(),
                        after_element_id: field.after.to_string(),
                    })
                    .collect(),
            })
            .collect();

        let mut limits = std::mem::take(&mut self.coverage);
        for limit in &diff.limits {
            if let Some(record) = self.project_limit(limit) {
                limits.push(record);
            }
        }
        self.with_coverage_limits(&limits)
    }

    pub fn with_coverage_limits(mut self, limits: &[CoverageLimit]) -> Self {
        let mut positions = BTreeMap::new();
        let mut aliases = BTreeMap::new();
        let mut coverage: Vec<CoverageLimit> = Vec::new();
        for limit in limits {
            let key = limit.key();
            if let Some(previous) = positions.get(&key).map(|&i: &usize| &mut coverage[i]) {
                aliases.insert(limit.limit_id.clone(), previous.limit_id.clone());
                extend_unique(&mut previous.evidence, &limit.evidence);
                extend_unique(&mut previous.affected_claims, &limit.affected_claims);
                if previous.presence.is_none() {
                    previous.presence.clone_from(&limit.presence);
                }
                if previous.source != limit.source {
                    previous.source = "both".into();
                }
            } else {
                positions.insert(key, coverage.len());
                coverage.push(limit.clone());
            }
        }
        for (row_id, links) in self
            .change
            .elements
            .iter_mut()
            .chain(&mut self.impacts)
            .map(|row| (&row.row_id, &mut row.coverage_limits))
            .chain(
                self.warnings
                    .iter_mut()
                    .map(|row| (&row.claim_id, &mut row.coverage_limits)),
            )
        {
            for link in links.iter_mut() {
                if let Some(id) = aliases.get(link) {
                    *link = id.clone();
                }
            }
            let mut seen = BTreeSet::new();
            links.retain(|id| seen.insert(id.clone()));
            for limit in &mut coverage {
                if links.contains(&limit.limit_id) {
                    extend_unique(&mut limit.affected_claims, std::slice::from_ref(row_id));
                }
                if limit.affected_claims.contains(row_id) {
                    extend_unique(links, std::slice::from_ref(&limit.limit_id));
                }
            }
        }
        self.coverage = coverage;
        self
    }

    fn project_limit(&mut self, limit: &crate::ComparisonLimit) -> Option<CoverageLimit> {
        let (side, basis, id) = match (limit.side, &self.change.producer) {
            (ComparisonSide::Before, Some(slots)) => {
                ("before", "actual_before", slots.before_input_id.as_deref()?)
            }
            (ComparisonSide::After, Some(slots)) => {
                ("after", "actual_after", slots.after_input_id.as_deref()?)
            }
            _ => return None,
        };
        let input = self.inputs.iter().find(|input| input.input_id == id)?;
        let scope = match &limit.scope {
            LimitScope::Schema => LimitRegion::Schema {
                path_scope: Some(input.scope.clone()),
            },
            LimitScope::Operation(op) => LimitRegion::Operation {
                operation_id: op.as_ref().map(ToString::to_string),
                path_scope: op.is_none().then(|| input.scope.clone()),
            },
            LimitScope::Status(op, status) => LimitRegion::Status {
                operation_id: op.to_string(),
                status: status.map(|status| status.to_string()),
            },
            LimitScope::Root(op, key) => {
                let (direction, status) = match key {
                    RootKey::Request => ("request", None),
                    RootKey::Response(status) => ("response", Some(status.to_string())),
                    RootKey::Fallback => ("response", Some("default".into())),
                };
                LimitRegion::Root {
                    operation_id: op.to_string(),
                    direction: direction.into(),
                    status,
                }
            }
        };
        let mut record = CoverageLimit {
            limit_id: String::new(),
            comparison_id: basis.into(),
            input_id: id.into(),
            side: side.into(),
            authority: input.operation_authority.clone(),
            scope,
            pointer: limit
                .pointer
                .iter()
                .map(|token| match token {
                    PointerToken::Property(name) => WirePointer::Property { name: name.clone() },
                    PointerToken::ListItem => WirePointer::ListItem,
                    PointerToken::MapValue => WirePointer::MapValue,
                })
                .collect(),
            claim: limit_claim(&limit.claim).into(),
            reason: limit.reason.clone(),
            source: match limit.origin {
                crate::LimitOrigin::InputInventory => "input_inventory",
                crate::LimitOrigin::Comparison => "comparison",
                crate::LimitOrigin::Both => "both",
            }.into(),
            evidence: limit
                .evidence
                .iter()
                .map(|site| {
                    json!({
                        "tree":site.tree,"file":site.file,"blob":site.blob,"pointer":site.pointer
                    })
                })
                .chain(limit.producer_sites.iter().map(|site| json!({
                    "tree":site.tree,"file":site.file,"blob":site.blob,"span":[site.span.start,site.span.end],
                }))).collect(),
            affected_claims: affected_changes(limit, &self.change.elements),
            presence: limit.presence.as_ref().map(|sides| {
                json!({
                    "before_element_id":sides.before.id.as_ref().map(element_id),
                    "before_presence":presence(sides.before.presence),
                    "before_shape":null,
                    "after_element_id":sides.after.id.as_ref().map(element_id),
                    "after_presence":presence(sides.after.presence),
                    "after_shape":null
                })
            }),
        };
        record.limit_id = format!("limit:{:x}", Sha256::digest(record.key().as_bytes()));
        if limit.origin != crate::LimitOrigin::InputInventory {
            let claim_id = format!("unchecked:{}", record.limit_id);
            record.affected_claims.push(claim_id.clone());
            if !self
                .warnings
                .iter()
                .any(|warning| warning.claim_id == claim_id)
            {
                self.warnings.push(UncheckedClaim {
                    claim_id,
                    status: "unchecked".into(),
                    coverage_limits: vec![record.limit_id.clone()],
                });
            }
        }
        Some(record)
    }
}

fn diff_matches_slots(doc: &ImpactDocument, diff: &SchemaDiff<'_>) -> bool {
    let Some((slots, bases)) = doc.change.producer.as_ref().zip(diff.bases.as_ref()) else {
        return false;
    };
    [&slots.before_input_id, &slots.after_input_id]
        .into_iter()
        .zip(bases)
        .all(|(id, basis)| {
            let mut inputs = doc
                .inputs
                .iter()
                .filter(|input| id.as_ref() == Some(&input.input_id));
            inputs
                .next()
                .filter(|_| inputs.next().is_none())
                .is_some_and(|input| {
                    input.role == crate::revision::Role::Producer
                        && input.api == basis.api
                        && input.operation_authority == basis.authority
                        && input
                            .snapshot
                            .identity()
                            .is_some_and(|(tree, _)| tree == basis.tree)
                })
        })
}

fn affected_changes(
    limit: &crate::ComparisonLimit,
    changes: &[ElementChangeRecord],
) -> Vec<String> {
    let LimitScope::Root(operation, root) = &limit.scope else {
        return Vec::new();
    };
    if limit.pointer.is_empty()
        || !matches!(
            limit.claim,
            ComparisonClaim::Property(_) | ComparisonClaim::FieldPresence
        )
    {
        return Vec::new();
    }
    let root = match root {
        RootKey::Request => crate::element::MessageRoot::Request(operation.clone()),
        RootKey::Response(status) => {
            crate::element::MessageRoot::Response(operation.clone(), *status)
        }
        RootKey::Fallback => return Vec::new(),
    };
    let target = crate::element::FieldId::new(root, limit.pointer.clone()).to_string();
    changes
        .iter()
        .filter(|row| {
            let (element, declaration) = match limit.side {
                ComparisonSide::Before => (&row.before_element_id, &row.before_declaration_id),
                ComparisonSide::After => (&row.after_element_id, &row.after_declaration_id),
            };
            element.as_ref().or(declaration.as_ref()) == Some(&target)
        })
        .map(|row| row.row_id.clone())
        .collect()
}

impl CoverageLimit {
    pub(super) fn key(&self) -> String {
        json!([
            self.comparison_id,
            self.side,
            self.scope,
            self.pointer,
            self.claim,
            self.reason
        ])
        .to_string()
    }
}

fn extend_unique<T: PartialEq + Clone>(items: &mut Vec<T>, other: &[T]) {
    for item in other {
        if !items.contains(item) {
            items.push(item.clone());
        }
    }
}

fn element_row(change: &crate::ElementChange<'_>) -> ElementChangeRecord {
    let before = &change.before;
    let after = &change.after;
    let before_element_id = before.element.id.as_ref().map(element_id);
    let after_element_id = after.element.id.as_ref().map(element_id);
    let change_kinds: Vec<String> = change
        .kinds
        .iter()
        .map(|kind| {
            serde_json::to_value(kind)
                .expect("serializable kind")
                .as_str()
                .expect("kind name")
                .to_owned()
        })
        .collect();
    let key = json!([
        change.operation.to_string(),
        before_element_id,
        after_element_id,
        before.declaration_id.as_ref().map(ToString::to_string),
        after.declaration_id.as_ref().map(ToString::to_string),
        change_kinds
    ]);
    ElementChangeRecord {
        row_id: format!("change:{:x}", Sha256::digest(key.to_string().as_bytes())),
        operation_id: change.operation.to_string(),
        before_element_id,
        after_element_id,
        change_kinds,
        before_presence: presence(before.element.presence).into(),
        after_presence: presence(after.element.presence).into(),
        before_shape: before.shape.as_ref().map(WireSignature::from_shape),
        after_shape: after.shape.as_ref().map(WireSignature::from_shape),
        before_declaration_id: before.declaration_id.as_ref().map(ToString::to_string),
        after_declaration_id: after.declaration_id.as_ref().map(ToString::to_string),
        before_fields: details(&before.fields),
        after_fields: details(&after.fields),
        enum_added: change.enum_added.iter().map(scalar).collect(),
        enum_removed: change.enum_removed.iter().map(scalar).collect(),
        coverage_limits: Vec::new(),
    }
}

fn details(fields: &[crate::FieldDetail]) -> Vec<FieldRecord> {
    fields
        .iter()
        .map(|field| FieldRecord {
            field_id: field.id.to_string(),
            signature: Some(WireSignature::from_shape(&field.shape)),
        })
        .collect()
}

fn scalar(value: &crate::EnumValue) -> Scalar {
    match value {
        crate::EnumValue::Null => Scalar::Null(()),
        crate::EnumValue::Boolean(v) => Scalar::Boolean(*v),
        crate::EnumValue::Number(v) => Scalar::Number(v.parse().expect("canonical number")),
        crate::EnumValue::String(v) => Scalar::String(v.clone()),
    }
}

fn element_id(id: &ElementId) -> String {
    match id {
        ElementId::Operation(op) => op.to_string(),
        ElementId::Status(op, key) => format!("status:{op}:{key}"),
        ElementId::Field(field) => field.to_string(),
    }
}

fn presence(state: Presence) -> &'static str {
    match state {
        Presence::Present => "present",
        Presence::Absent => "absent",
        Presence::Unknown => "unknown",
    }
}

fn limit_claim(claim: &ComparisonClaim) -> &'static str {
    match claim {
        ComparisonClaim::OperationPresence => "operation_presence",
        ComparisonClaim::StatusMembership => "status_membership",
        ComparisonClaim::RootPairing => "root_pairing",
        ComparisonClaim::FieldPresence => "field_presence",
        ComparisonClaim::Coverage => "coverage",
        ComparisonClaim::Property(slot) => match slot {
            ShapeSlot::Type => "type",
            ShapeSlot::Format => "format",
            ShapeSlot::Required => "required",
            ShapeSlot::Enum => "enum",
            ShapeSlot::ItemType => "item_type",
        },
        ComparisonClaim::Producer(slot) => match slot {
            ProducerSlot::Declaration => "declaration",
            ProducerSlot::Validation => "validation",
            ProducerSlot::Reference => "reference",
            ProducerSlot::Code => "code",
        },
    }
}
