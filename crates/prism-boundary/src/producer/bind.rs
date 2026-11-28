//! Resolves directives against the admitted schema index and builds the binding rows.

use super::directive::Directive;
use super::scan::{Decl, Found, Kind, Scan};
use crate::element::{schema_path, OperationId};
use crate::model::{
    AnnotationDisposition as D, AnnotationRecord, Binding, EdgeGrade, HandlerSite, SourceSite,
};
use crate::SchemaIndex;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct FileScan<'a> {
    pub tree: &'a str,
    pub path: &'a str,
    pub blob: &'a str,
    pub scan: Scan,
}

pub(super) enum Resolution {
    Operations(Vec<String>),
    Without,
    NotChecked(&'static str),
}

pub(super) fn typed_id(method: &str, path: &str) -> String {
    OperationId::new(&method.to_ascii_uppercase(), &schema_path(path, "", &[])).to_string()
}

/// The ids `resolve` could return for `method path` across every projection of the schema's
/// servers and base path, or `None` when they cannot be enumerated (an incomplete operation
/// domain, or no declaration to learn a projection from).
pub(super) fn projected_ids(schema: &SchemaIndex, method: &str, path: &str) -> Option<Vec<String>> {
    let upper = method.to_ascii_uppercase();
    if schema.operation_coverage_incomplete
        || schema
            .operation_gaps
            .iter()
            .any(|gap| gap.method.as_deref().is_none_or(|m| m == upper))
    {
        return None;
    }
    let mut prefixes = BTreeSet::new();
    for op in &schema.operations {
        for declaration in &op.declarations {
            let authored = schema_path(&declaration.path, "", &[]).to_string();
            let prefix = op.id.path().strip_suffix(authored.as_str())?;
            let rebuilt = schema_path(&declaration.path, prefix, &[]).to_string();
            if rebuilt != op.id.path() {
                return None;
            }
            prefixes.insert(prefix.to_string());
        }
    }
    let mut ids: Vec<String> = prefixes
        .iter()
        .map(|prefix| OperationId::new(&upper, &schema_path(path, prefix, &[])).to_string())
        .collect();
    ids.sort();
    ids.dedup();
    (!ids.is_empty()).then_some(ids)
}

/// Which operations the annotation `method path` names. The typed authored path is compared,
/// never a collapsed or base-adjusted one.
pub(super) fn resolve(
    schema: &SchemaIndex,
    admitted: bool,
    method: &str,
    path: &str,
) -> Resolution {
    if !admitted {
        return Resolution::NotChecked("schema_not_admitted");
    }
    let wanted = schema_path(path, "", &[]);
    let printed = wanted.to_string();
    let same_method = |m: &str| m.eq_ignore_ascii_case(method);
    let mut hits = Vec::new();
    let mut printed_only = false;
    let mut includes_base = false;
    for op in &schema.operations {
        let declared = op.declarations.iter().filter(|d| same_method(&d.method));
        let mut matched = false;
        for d in declared {
            let authored = schema_path(&d.path, "", &[]);
            matched |= authored == wanted;
            printed_only |= authored != wanted && authored.to_string() == printed;
        }
        includes_base |= same_method(op.id.method()) && op.id.path() == printed;
        if matched {
            hits.push(op.id.to_string());
        }
    }
    if !hits.is_empty() {
        hits.sort();
        hits.dedup();
        return Resolution::Operations(hits);
    }
    if printed_only {
        return Resolution::NotChecked("typed_path_differs");
    }
    if includes_base {
        return Resolution::NotChecked("annotation_path_includes_base");
    }
    let upper = method.to_ascii_uppercase();
    let gap = schema.operation_coverage_incomplete
        || schema
            .operation_gaps
            .iter()
            .any(|gap| gap.method.as_deref().is_none_or(|m| m == upper));
    if gap {
        Resolution::NotChecked("schema_operations_incomplete")
    } else {
        Resolution::Without
    }
}

fn site(file: &FileScan<'_>, found: &Found) -> SourceSite {
    SourceSite {
        tree: file.tree.into(),
        file: file.path.into(),
        blob: file.blob.into(),
        line: found.line,
        span: Some([found.start, found.end]),
    }
}

fn handler_site(file: &FileScan<'_>, decl: &Decl) -> HandlerSite {
    HandlerSite {
        package: file.scan.package.clone(),
        name: decl.name.clone(),
        receiver: decl.receiver.clone(),
        receiver_type: decl.receiver_type.clone().filter(|name| !name.is_empty()),
        site: SourceSite {
            tree: file.tree.into(),
            file: file.path.into(),
            blob: file.blob.into(),
            line: decl.line,
            span: Some([decl.start, decl.end]),
        },
    }
}

pub(super) struct Outcome {
    pub rows: Vec<Binding>,
    pub records: Vec<AnnotationRecord>,
    /// (file index, record index, reason, possible keys) for every entry-level gap.
    pub gaps: Vec<Gap>,
}

pub(super) struct Gap {
    pub file: usize,
    pub reason: &'static str,
    pub keys: Vec<String>,
    pub span: [usize; 2],
}

struct Edge {
    handler: HandlerSite,
    generated: bool,
    bodyless: bool,
    sites: Vec<SourceSite>,
    method: String,
    path: String,
    record_ids: Vec<usize>,
}

/// Entry-level gap about operations the annotation could also name under deployments the schema
/// did not project. It does not hide a competitor handler for the known projection.
const DEPLOYMENTS_UNPROJECTED: &str = "annotation_deployments_unprojected";

/// A gap key list for a directive the schema could not decide: the projected ids, or keyless
/// (it blocks everything) when they cannot be enumerated or the schema was not admitted.
fn gap_keys(schema: &SchemaIndex, admitted: bool, method: &str, path: &str) -> Vec<String> {
    if !admitted {
        return Vec::new();
    }
    projected_ids(schema, method, path).unwrap_or_default()
}

/// `unread`: an expected file's text was never read, so a competitor could hide there.
pub(super) fn bind(
    files: &[FileScan<'_>],
    schema: &SchemaIndex,
    admitted: bool,
    unread: bool,
) -> Outcome {
    let mut records = Vec::new();
    let mut gaps = Vec::new();
    let mut edges: BTreeMap<(String, String, usize), Edge> = BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        for found in &file.scan.found {
            let mut record = AnnotationRecord {
                site: site(file, found),
                raw: found.raw.clone(),
                raw_method: None,
                raw_path: None,
                disposition: D::Malformed,
                reason: None,
                handler: found
                    .handler
                    .map(|h| handler_site(file, &file.scan.decls[h])),
                operation_ids: Vec::new(),
                notes: Vec::new(),
            };
            if file.scan.generated {
                record.notes.push("generated_file".into());
            }
            let span = [found.start, found.end];
            let mut gap = |reason, keys| {
                gaps.push(Gap {
                    file: index,
                    reason,
                    keys,
                    span,
                })
            };
            match &found.kind {
                Kind::Block => {
                    record.disposition = D::UnsupportedForm;
                    record.reason = Some("block_comment".into());
                    gap("annotation_unsupported_form", Vec::new());
                }
                Kind::Directive(Directive::Invalid {
                    reason,
                    method,
                    path,
                }) => {
                    record.raw_method.clone_from(method);
                    record.raw_path.clone_from(path);
                    record.reason = Some((*reason).into());
                    gap("annotation_malformed", Vec::new());
                }
                Kind::Directive(Directive::Canonical { method, path }) => {
                    record.raw_method = Some(method.clone());
                    record.raw_path = Some(path.clone());
                    if let Some(reason) = found.unattached {
                        record.disposition = D::Unattached;
                        record.reason = Some(reason.into());
                        gap(
                            "annotation_unattached",
                            gap_keys(schema, admitted, method, path),
                        );
                    } else {
                        match resolve(schema, admitted, method, path) {
                            Resolution::Without => {
                                record.disposition = D::AnnotationWithoutOperation;
                            }
                            Resolution::NotChecked(reason) => {
                                record.disposition = D::NotChecked;
                                record.reason = Some(reason.into());
                                let keys = match reason {
                                    // The annotation already spells the projected id.
                                    "annotation_path_includes_base" => vec![typed_id(method, path)],
                                    "schema_not_admitted" => Vec::new(),
                                    _ => gap_keys(schema, admitted, method, path),
                                };
                                gap("annotation_not_checked", keys);
                            }
                            Resolution::Operations(ids) => {
                                let decl = found.handler.expect("attached");
                                for id in &ids {
                                    let slot = edges
                                        .entry((id.clone(), file.path.into(), decl))
                                        .or_insert_with(|| Edge {
                                            handler: handler_site(file, &file.scan.decls[decl]),
                                            generated: file.scan.generated,
                                            bodyless: file.scan.decls[decl].bodyless,
                                            sites: Vec::new(),
                                            method: method.clone(),
                                            path: path.clone(),
                                            record_ids: Vec::new(),
                                        });
                                    slot.sites.push(record.site.clone());
                                    slot.record_ids.push(records.len());
                                }
                                record.disposition = D::Bound;
                                record.operation_ids = ids;
                                if schema.diagnostics.iter().any(|d| d.scope == "server") {
                                    gap(DEPLOYMENTS_UNPROJECTED, Vec::new());
                                }
                            }
                        }
                    }
                }
            }
            records.push(record);
        }
    }
    let mut handlers: BTreeMap<&str, BTreeSet<(&str, usize)>> = BTreeMap::new();
    for (id, file, decl) in edges.keys() {
        handlers.entry(id).or_default().insert((file, *decl));
    }
    let mut rows = Vec::new();
    for ((id, _, _), edge) in &edges {
        let tie = handlers[id.as_str()].len() > 1;
        // Uniqueness needs every possible competitor accounted for: an expected file never
        // read, or a gap that is keyless or names this operation, could hide one.
        let hidden = unread
            || gaps.iter().any(|gap| {
                gap.reason != DEPLOYMENTS_UNPROJECTED
                    && (gap.keys.is_empty() || gap.keys.contains(id))
            });
        if tie || hidden {
            for &r in &edge.record_ids {
                records[r].disposition = D::Candidate;
            }
        }
        let mut evidence = BTreeSet::new();
        evidence.extend(hidden.then_some("competitor_unread"));
        evidence.extend(tie.then_some("competing_handlers"));
        evidence.extend(edge.generated.then_some("generated_file"));
        evidence.extend(edge.bodyless.then_some("bodyless_declaration"));
        rows.push(Binding {
            operation_id: id.clone(),
            original_method: edge.method.clone(),
            original_path: edge.path.clone(),
            provenance: "proven".into(),
            sites: edge.sites.clone(),
            config_source: None,
            suffix: None,
            plugin: None,
            success_statuses: Vec::new(),
            message_roots: Vec::new(),
            fields: Vec::new(),
            observed_fields: Vec::new(),
            grade: Some(if tie || hidden {
                EdgeGrade::Candidate
            } else {
                EdgeGrade::Bound
            }),
            handler: Some(edge.handler.clone()),
            evidence: evidence.into_iter().map(String::from).collect(),
        });
    }
    Outcome {
        rows,
        records,
        gaps,
    }
}
