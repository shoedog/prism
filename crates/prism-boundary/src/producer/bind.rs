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

pub(super) fn bind(files: &[FileScan<'_>], schema: &SchemaIndex, admitted: bool) -> Outcome {
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
                    let key = typed_id(method, path);
                    if let Some(reason) = found.unattached {
                        record.disposition = D::Unattached;
                        record.reason = Some(reason.into());
                        gap("annotation_unattached", vec![key]);
                    } else {
                        match resolve(schema, admitted, method, path) {
                            Resolution::Without => {
                                record.disposition = D::AnnotationWithoutOperation;
                            }
                            Resolution::NotChecked(reason) => {
                                record.disposition = D::NotChecked;
                                record.reason = Some(reason.into());
                                gap("annotation_not_checked", vec![key]);
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
        if tie {
            for &r in &edge.record_ids {
                records[r].disposition = D::Candidate;
            }
        }
        let mut evidence = BTreeSet::new();
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
            grade: Some(if tie {
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
