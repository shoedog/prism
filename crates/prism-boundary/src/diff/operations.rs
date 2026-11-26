use super::basis::{in_scope, same_path, BoundSide};
use super::fields::OperationFields;
use super::limits::Ledger;
use super::{producer, status, *};
use crate::element::MessageRoot;
use crate::{Coverage, Operation};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Entry<'a> {
    schema: Option<&'a Operation>,
    producers: Vec<&'a ProducerOperation<'a>>,
    fields: OperationFields<'a>,
}

struct Side<'a> {
    bound: BoundSide<'a>,
    entries: BTreeMap<OperationId, Entry<'a>>,
    gaps: Vec<Gap>,
}

struct Gap {
    method: Option<String>,
    paths: Option<Vec<CanonicalPath>>,
    sites: Vec<SchemaSite>,
    producer_sites: Vec<ProducerSite>,
    reason: &'static str,
}

impl<'a> Side<'a> {
    fn new(bound: BoundSide<'a>, scope: &ComparisonScope) -> Self {
        let mut entries = BTreeMap::<OperationId, Entry<'_>>::new();
        let mut gaps = Vec::new();
        if let Some(index) = bound.schema {
            for op in &index.operations {
                if op
                    .declarations
                    .iter()
                    .any(|site| in_scope(&site.canonical_path, scope))
                {
                    entries.entry(op.id.clone()).or_default().schema = Some(op);
                }
            }
            for (fields, comparable) in [(&index.fields, true), (&index.observed_fields, false)] {
                for field in fields {
                    let (op, root) = match &field.id.root {
                        MessageRoot::Request(op) => (op, RootKey::Request),
                        MessageRoot::Response(op, key) => (op, RootKey::Response(*key)),
                    };
                    if let Some(entry) = entries.get_mut(op) {
                        entry.fields.insert(root, field, comparable);
                    }
                }
            }
            for gap in &index.operation_gaps {
                if gap
                    .paths
                    .as_ref()
                    .is_none_or(|paths| paths.iter().any(|path| in_scope(path, scope)))
                {
                    gaps.push(Gap {
                        method: gap.method.clone(),
                        paths: gap.paths.clone(),
                        sites: diagnostic_sites(&gap.diagnostic),
                        producer_sites: Vec::new(),
                        reason: gap.diagnostic.code,
                    });
                }
            }
        }
        if let Some(inventory) = bound.producer {
            for op in &inventory.operations {
                if let Some(path) = producer::bound_path(op, inventory.basis.tree) {
                    if !in_scope(path, scope) || !in_scope(path, &inventory.basis.scope) {
                        continue;
                    }
                } else if !matches!(scope, ComparisonScope::Api)
                    || !matches!(inventory.basis.scope, ComparisonScope::Api)
                {
                    let reason = if producer::path(op).is_none() {
                        "producer_operation_unscoped"
                    } else {
                        "producer_evidence_unbound"
                    };
                    gaps.push(Gap {
                        method: Some(op.operation.method().into()),
                        paths: None,
                        sites: Vec::new(),
                        producer_sites: op
                            .declaration
                            .sites
                            .iter()
                            .filter(|site| site.tree == inventory.basis.tree)
                            .cloned()
                            .collect(),
                        reason,
                    });
                    continue;
                }
                entries
                    .entry(op.operation.clone())
                    .or_default()
                    .producers
                    .push(op);
            }
        }
        Self {
            bound,
            entries,
            gaps,
        }
    }

    fn standing_limits(&mut self, side: ComparisonSide, ledger: &mut Ledger) {
        let Some(index) = self.bound.schema else {
            ledger.record(ComparisonLimit::new(
                side,
                LimitScope::Schema,
                ComparisonClaim::Coverage,
                "schema_unavailable",
            ));
            return;
        };
        for diagnostic in &index.diagnostics {
            if index
                .operation_gaps
                .iter()
                .any(|gap| &gap.diagnostic == diagnostic)
            {
                continue;
            }
            match diagnostic.scope {
                "responses" | "response_body" | "origin" => continue,
                "operation"
                    if matches!(
                        diagnostic.code,
                        "duplicate_operation" | "operation_unreadable"
                    ) =>
                {
                    continue
                }
                "document" | "paths" | "path_item" | "server" => {
                    let method = diagnostic_method(diagnostic);
                    self.gaps.push(Gap {
                        sites: diagnostic_sites(diagnostic),
                        producer_sites: Vec::new(),
                        reason: diagnostic.code,
                        method,
                        paths: None,
                    });
                }
                _ => {
                    let mut limit = ComparisonLimit::new(
                        side,
                        LimitScope::Schema,
                        ComparisonClaim::Coverage,
                        diagnostic.code,
                    );
                    limit.evidence = diagnostic_sites(diagnostic);
                    ledger.record(limit);
                }
            }
        }
    }
}

fn diagnostic_method(diagnostic: &crate::SchemaDiagnostic) -> Option<String> {
    match diagnostic.site.pointer.as_slice() {
        [paths, _, method, ..]
            if paths == "paths"
                && matches!(
                    method.as_str(),
                    "get" | "put" | "post" | "delete" | "options" | "head" | "patch" | "trace"
                ) =>
        {
            Some(method.to_ascii_uppercase())
        }
        _ => None,
    }
}

fn diagnostic_sites(diagnostic: &crate::SchemaDiagnostic) -> Vec<SchemaSite> {
    let mut sites = diagnostic.reference_chain.clone();
    sites.push(diagnostic.site.clone());
    sites.sort();
    sites.dedup();
    sites
}

pub(super) fn operation_sites(op: &Operation) -> Vec<SchemaSite> {
    let mut sites: Vec<_> = op
        .declarations
        .iter()
        .flat_map(|declaration| {
            [&declaration.site, &declaration.referring_site]
                .into_iter()
                .chain(&declaration.reference_chain)
        })
        .cloned()
        .collect();
    sites.sort();
    sites.dedup();
    sites
}

pub(super) fn compare<'a>(bounds: [BoundSide<'a>; 2], scope: &ComparisonScope) -> SchemaDiff<'a> {
    let mut sides = bounds.map(|bound| Side::new(bound, scope));
    let mut ledger = Ledger::default();
    let mut diff = SchemaDiff::default();
    for (side, label) in sides.iter_mut().zip(SIDES) {
        side.standing_limits(label, &mut ledger);
    }
    let ids: BTreeSet<_> = sides.iter().flat_map(|side| side.entries.keys()).collect();
    let mut used_gaps = BTreeSet::new();
    for id in ids {
        compare_operation(&sides, id, &mut diff, &mut ledger, &mut used_gaps);
    }
    for (i, side) in sides.iter().enumerate() {
        for (_, gap) in side
            .gaps
            .iter()
            .enumerate()
            .filter(|(j, _)| !used_gaps.contains(&(i, *j)))
        {
            let mut limit = ComparisonLimit::new(
                SIDES[i],
                LimitScope::Operation(None),
                ComparisonClaim::OperationPresence,
                gap.reason,
            );
            limit.evidence.clone_from(&gap.sites);
            limit.producer_sites.clone_from(&gap.producer_sites);
            ledger.record(limit);
        }
    }
    ledger.finish(&mut diff);
    diff
}

fn compare_operation<'a>(
    sides: &[Side<'a>; 2],
    id: &OperationId,
    diff: &mut SchemaDiff<'a>,
    ledger: &mut Ledger,
    used_gaps: &mut BTreeSet<(usize, usize)>,
) {
    let empty = Entry::default();
    let entries = sides
        .each_ref()
        .map(|side| side.entries.get(id).unwrap_or(&empty));
    let schemas = entries.map(|entry| entry.schema);
    let paths = [0, 1].map(|i| {
        let entry = entries[i];
        entry
            .schema
            .and_then(|op| op.declarations.first())
            .map(|site| &site.canonical_path)
            .or_else(|| {
                entry
                    .producers
                    .first()
                    .and_then(|op| producer::bound_path(op, sides[i].bound.basis.tree))
            })
    });
    let mismatched = matches!(paths, [Some(b), Some(a)] if !same_path(b, a));
    let typed = entries.map(|entry| {
        entry.schema.is_none_or(|op| {
            op.declarations.first().is_some_and(|first| {
                op.declarations
                    .iter()
                    .all(|site| same_path(&first.canonical_path, &site.canonical_path))
            })
        })
    });
    let gaps = [0, 1].map(|i| {
        sides[i]
            .gaps
            .iter()
            .enumerate()
            .filter(|(_, gap)| {
                gap.method
                    .as_ref()
                    .is_none_or(|method| method == id.method())
                    && gap.paths.as_ref().is_none_or(|possible| {
                        paths[i].or(paths[1 - i]).is_none_or(|path| {
                            possible.iter().any(|candidate| same_path(path, candidate))
                        })
                    })
            })
            .collect::<Vec<_>>()
    });
    let unbound = [0, 1].map(|i| {
        matches!(entries[i].producers.as_slice(), [op]
            if !producer::evidence_bound(&op.declaration.sites, sides[i].bound.basis.tree))
    });
    let presence = [0, 1].map(|i| {
        let side = &sides[i];
        let present = if side.bound.schema.is_some() {
            schemas[i].is_some()
        } else {
            entries[i].producers.len() == 1
                && !unbound[i]
                && side
                    .bound
                    .producer
                    .is_some_and(|inventory| inventory.coverage != Coverage::Withheld)
        };
        let checked = (side.bound.schema.is_some()
            || side.bound.producer.is_some_and(|inventory| {
                entries[i].producers.is_empty()
                    && inventory.coverage == Coverage::Checked
                    && (matches!(inventory.basis.scope, ComparisonScope::Api)
                        || paths[1 - i].is_some_and(|path| in_scope(path, &inventory.basis.scope)))
            }))
            && gaps[i].is_empty()
            && entries[i].producers.len() <= 1;
        if present && typed[i] && !mismatched {
            ElementPresence::present(ElementId::Operation(id.clone()))
        } else if !present && checked {
            ElementPresence::without_id(Presence::Absent)
        } else {
            ElementPresence::without_id(Presence::Unknown)
        }
    });
    for i in 0..2 {
        for &(j, gap) in &gaps[i] {
            used_gaps.insert((i, j));
            let claim = if presence[i].presence == Presence::Present {
                ComparisonClaim::Coverage
            } else {
                ComparisonClaim::OperationPresence
            };
            let mut limit = ComparisonLimit::new(
                SIDES[i],
                LimitScope::Operation(Some(id.clone())),
                claim,
                gap.reason,
            );
            if presence[i].presence == Presence::Unknown {
                limit = limit.with_presence(&presence);
            }
            limit.evidence.clone_from(&gap.sites);
            limit.producer_sites.clone_from(&gap.producer_sites);
            ledger.record(limit);
        }
        if let Some(op) = schemas[i] {
            if op.declarations.len() > 1 || mismatched || !typed[i] {
                let claim = if presence[i].presence == Presence::Present {
                    ComparisonClaim::Coverage
                } else {
                    ComparisonClaim::OperationPresence
                };
                let reason = if op.declarations.len() > 1 || !typed[i] {
                    "duplicate_operation"
                } else {
                    "typed_operation_mismatch"
                };
                let mut limit = ComparisonLimit::new(
                    SIDES[i],
                    LimitScope::Operation(Some(id.clone())),
                    claim,
                    reason,
                );
                limit.evidence = operation_sites(op);
                ledger.record(limit.with_presence(&presence));
            }
            status::standing(ledger, SIDES[i], op, &entries[i].fields);
        } else if sides[i].bound.schema.is_none()
            && presence[i].presence == Presence::Unknown
            && (gaps[i].is_empty() || !entries[i].producers.is_empty())
        {
            let reason = match sides[i].bound.producer {
                None => "operation_inventory_unavailable",
                Some(_) if entries[i].producers.len() > 1 => "ambiguous_producer_operation",
                Some(_) if unbound[i] => "producer_evidence_unbound",
                Some(_) if mismatched => "typed_operation_mismatch",
                Some(_) => "producer_operation_unchecked",
            };
            let mut limit = ComparisonLimit::new(
                SIDES[i],
                LimitScope::Operation(Some(id.clone())),
                ComparisonClaim::OperationPresence,
                reason,
            )
            .with_presence(&presence);
            limit.producer_sites.extend(
                entries[i]
                    .producers
                    .iter()
                    .flat_map(|op| &op.declaration.sites)
                    .filter(|site| site.tree == sides[i].bound.basis.tree)
                    .cloned(),
            );
            ledger.record(limit);
        }
    }

    let mut producer_mismatch = false;
    let evidence = [0, 1].map(|i| {
        let mut side = ChangeSide::new(presence[i].clone());
        side.producer = entries[i].producers.first().copied().filter(|_| {
            entries[i].producers.len() == 1
                && sides[i]
                    .bound
                    .producer
                    .is_some_and(|inventory| inventory.coverage != Coverage::Withheld)
        });
        if let Some(op) = schemas[i] {
            side.sites = operation_sites(op);
            if let Some(producer) = side.producer.filter(|producer| {
                producer::path(producer).is_some_and(|path| {
                    !typed[i] || !paths[i].is_some_and(|schema_path| same_path(path, schema_path))
                })
            }) {
                let mut limit = ComparisonLimit::new(
                    SIDES[i],
                    LimitScope::Operation(Some(id.clone())),
                    ComparisonClaim::Coverage,
                    "producer_operation_mismatch",
                );
                limit.evidence.clone_from(&side.sites);
                limit.producer_sites.extend(
                    producer
                        .declaration
                        .sites
                        .iter()
                        .filter(|site| site.tree == sides[i].bound.basis.tree)
                        .cloned(),
                );
                ledger.record(limit);
                side.producer = None;
                producer_mismatch = true;
            }
        }
        side
    });
    let whole = match presence.each_ref().map(|side| side.presence) {
        [Presence::Absent, Presence::Present] => Some((1, ChangeKind::OpAdded)),
        [Presence::Present, Presence::Absent] => Some((0, ChangeKind::OpRemoved)),
        _ => None,
    };
    if let Some((i, kind)) = whole {
        let mut evidence = evidence;
        if let Some(op) = schemas[i].filter(|_| gaps[i].is_empty()) {
            evidence[i].success_keys = status::established_keys(op);
            evidence[i].fields = entries[i].fields.details(op, None);
        }
        let mut change = ElementChange::new(id, evidence);
        change.kinds.push(kind);
        diff.changes.push(change);
    } else if presence
        .iter()
        .all(|side| side.presence == Presence::Present)
    {
        if let [Some(b), Some(a)] = schemas {
            if gaps.iter().all(Vec::is_empty) {
                status::compare(diff, ledger, [b, a], entries.map(|entry| &entry.fields));
            }
        }
        if !producer_mismatch {
            producer::compare(
                diff,
                ledger,
                id,
                evidence,
                sides.each_ref().map(|side| side.bound.producer),
            );
        }
    }
}
