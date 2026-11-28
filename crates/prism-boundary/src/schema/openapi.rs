mod parameters;

use self::parameters::labels;
use super::references::{parse_json, Documents, Field, Located, Parsed, PathItem};
use super::status::read_statuses;
use super::{servers, Declaration, Diagnostic, Dialect, Inventory, Operation, SchemaError, Site};
use crate::element::{schema_path, OperationId};
use crate::inventory::OperationGap;
use crate::revision::AdmittedSchemaInput;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const METHODS: &[&str] = &["delete", "get", "head", "options", "patch", "post", "put"];

pub(crate) fn read_operations(input: &AdmittedSchemaInput<'_>) -> Result<Inventory, SchemaError> {
    let documents = Documents::new(input, entry_value(input)?);
    scan_operations(&documents, input)
}

pub(super) fn scan_operations(
    documents: &Documents<'_>,
    input: &AdmittedSchemaInput<'_>,
) -> Result<Inventory, SchemaError> {
    let document = &documents.entry.value;
    let mut inventory = Inventory {
        api: input.api.into(),
        authority: input.authority.into(),
        digest: Sha256::digest(input.entry.bytes).into(),
        info_version: document["info"]["version"].as_str().map(str::to_owned),
        operations: BTreeMap::new(),
        unprojected: Vec::new(),
        operation_gaps: Vec::new(),
        diagnostics: Vec::new(),
    };
    if ["openapi", "swagger", "paths"]
        .iter()
        .any(|key| documents.entry.duplicate(&[], key))
    {
        inventory.diagnostics.push(Diagnostic::new(
            "duplicate_member",
            "document",
            documents.entry.site(Vec::new()),
        ));
        return Ok(inventory);
    }
    let dialect = match document["openapi"].as_str() {
        Some(version) if version.starts_with("3.0.") => Dialect::OpenApi30,
        Some(version) if version.starts_with("3.1.") => Dialect::OpenApi31,
        _ if document["swagger"] == "2.0" => Dialect::Swagger2,
        _ => return Err(SchemaError::Dialect),
    };
    let info = documents.entry.site(vec!["info".into()]);
    if documents.entry.duplicate(&[], "info") || documents.entry.has_duplicates(&info.pointer) {
        inventory.info_version = None;
        inventory
            .diagnostics
            .push(Diagnostic::new("duplicate_member", "info", info));
    }
    let Some(paths) = document["paths"].as_object() else {
        if document.get("paths").is_some() {
            inventory.diagnostics.push(Diagnostic::new(
                "paths_unreadable",
                "paths",
                documents.entry.site(vec!["paths".into()]),
            ));
        }
        return Ok(inventory);
    };
    for (path, raw) in paths.iter().collect::<BTreeMap<_, _>>() {
        if !path.starts_with('/') {
            continue;
        }
        let site = documents.entry.site(vec!["paths".into(), path.clone()]);
        if documents.entry.duplicate(&site.pointer[..1], path) {
            inventory
                .diagnostics
                .push(Diagnostic::new("duplicate_member", "path_item", site));
            continue;
        }
        let Some(item) = PathItem::resolve(
            documents,
            Located {
                value: raw,
                document: &documents.entry,
                site,
            },
            &mut inventory.diagnostics,
        ) else {
            continue;
        };
        for method in METHODS
            .iter()
            .copied()
            .chain((dialect != Dialect::Swagger2).then_some("trace"))
        {
            let Some(Field::Declared { location, chain }) = item.field(method) else {
                continue;
            };
            if !location.value.is_object() {
                let diagnostic =
                    Diagnostic::new("operation_unreadable", "operation", location.site.clone());
                let mut server_diagnostics = Vec::new();
                let deployments =
                    deployments(documents, &item, location, dialect, &mut server_diagnostics);
                let paths = (!deployments.is_empty()
                    && schema_path(path, "", &[]).captures().next().is_none()
                    && server_diagnostics
                        .iter()
                        .all(|entry| entry.scope == "origin"))
                .then(|| {
                    deployments
                        .iter()
                        .map(|base| schema_path(path, &base.path, &[]))
                        .collect()
                });
                inventory.operation_gaps.push(OperationGap {
                    method: Some(method.to_ascii_uppercase()),
                    paths,
                    diagnostic: diagnostic.clone(),
                });
                inventory.diagnostics.push(diagnostic);
                continue;
            }
            index_declaration(
                documents,
                &item,
                path,
                method,
                location,
                chain,
                dialect,
                &mut inventory,
            );
        }
    }
    inventory.diagnostics.sort();
    inventory.diagnostics.dedup();
    Ok(inventory)
}

#[allow(clippy::too_many_arguments)]
fn index_declaration(
    documents: &Documents<'_>,
    item: &PathItem<'_, '_>,
    raw_path: &str,
    method: &str,
    operation: &Located<'_, '_>,
    chain: &[Site],
    dialect: Dialect,
    inventory: &mut Inventory,
) {
    let Located { value, site, .. } = operation;
    let operation_id = if operation.duplicate("operationId") {
        inventory.diagnostics.push(Diagnostic::new(
            "duplicate_member",
            "operation",
            site.child("operationId"),
        ));
        None
    } else {
        value["operationId"].as_str().map(str::to_owned)
    };
    let (responses, success_keys) = if operation.duplicate("responses") {
        inventory.diagnostics.push(Diagnostic::new(
            "duplicate_member",
            "responses",
            site.child("responses"),
        ));
        (BTreeMap::new(), Vec::new())
    } else {
        read_statuses(value, site, dialect, &mut inventory.diagnostics)
    };
    let diagnostic_start = inventory.diagnostics.len();
    let deployments = deployments(
        documents,
        item,
        operation,
        dialect,
        &mut inventory.diagnostics,
    );
    let raw = schema_path(raw_path, "", &[]);
    let label_names = if dialect != Dialect::Swagger2 && raw.captures().next().is_some() {
        labels(
            documents,
            item,
            operation,
            &raw,
            dialect,
            &mut inventory.diagnostics,
        )
    } else {
        Some(Vec::new())
    };
    let labels = label_names.as_deref().unwrap_or_default();
    let declaration = Declaration {
        source_path: raw_path.into(),
        source_method: method.into(),
        path: schema_path(raw_path, "", labels),
        referring_site: chain
            .first()
            .cloned()
            .unwrap_or_else(|| item.site.child(method)),
        site: site.clone(),
        resource: operation
            .document
            .base
            .as_ref()
            .map(super::servers::Uri::key),
        parameters: match item.field("parameters") {
            Some(Field::Overlap) => Err(()),
            Some(Field::Declared { location, .. }) => Ok(Some((
                location
                    .document
                    .base
                    .as_ref()
                    .map(super::servers::Uri::key),
                location.site.clone(),
            ))),
            None => Ok(None),
        },
        reference_chain: chain.to_vec(),
        deployments: Vec::new(),
        responses,
        success_keys,
        value: (*value).clone(),
    };
    if deployments.is_empty() || label_names.is_none() {
        let mut causes: Vec<_> = inventory.diagnostics[diagnostic_start..]
            .iter()
            .filter(|diagnostic| matches!(diagnostic.scope, "path_parameters" | "server"))
            .cloned()
            .collect();
        if causes.is_empty() {
            causes.push(Diagnostic::new(
                "operation_unprojected",
                "operation",
                site.clone(),
            ));
        }
        inventory
            .operation_gaps
            .extend(causes.into_iter().map(|diagnostic| OperationGap {
                method: Some(method.to_ascii_uppercase()),
                paths: None,
                diagnostic,
            }));
        inventory.unprojected.push(Declaration {
            deployments,
            ..declaration
        });
        return;
    }
    let mut paths = BTreeMap::new();
    for deployment in deployments {
        let path = schema_path(raw_path, &deployment.path, labels);
        let id = OperationId::new(&method.to_ascii_uppercase(), &path);
        paths
            .entry(id)
            .or_insert_with(|| (path, BTreeSet::new()))
            .1
            .insert(deployment);
    }
    for (id, (path, deployments)) in paths {
        let declaration = Declaration {
            path,
            deployments: deployments.into_iter().collect(),
            ..declaration.clone()
        };
        match inventory.operations.entry(id.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Operation {
                    id,
                    operation_id: operation_id.clone(),
                    declarations: vec![declaration],
                    dialect,
                });
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                inventory.diagnostics.push(Diagnostic::new(
                    "duplicate_operation",
                    "operation",
                    declaration.referring_site.clone(),
                ));
                entry.get_mut().declarations.push(declaration);
            }
        }
    }
}

fn deployments(
    documents: &Documents<'_>,
    item: &PathItem<'_, '_>,
    operation: &Located<'_, '_>,
    dialect: Dialect,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<servers::Deployment> {
    let entry = &documents.entry;
    if dialect == Dialect::Swagger2 {
        if entry.duplicate(&[], "basePath") {
            diagnostics.push(Diagnostic::new(
                "duplicate_member",
                "server",
                entry.site(vec!["basePath".into()]),
            ));
            return Vec::new();
        }
        if entry.duplicate(&[], "host")
            || operation.duplicate("schemes")
            || operation.value.get("schemes").is_none() && entry.duplicate(&[], "schemes")
        {
            diagnostics.push(Diagnostic::new(
                "duplicate_member",
                "server",
                operation.site.clone(),
            ));
            return Vec::new();
        }
        return servers::swagger(
            &entry.value,
            operation.value,
            entry.base.as_ref(),
            &operation.site,
            diagnostics,
        );
    }
    if let Some(servers) = operation.child("servers") {
        if operation.duplicate("servers") || !empty_list(servers.value) {
            return openapi_deployments(servers, diagnostics);
        }
    }
    match item.field("servers") {
        Some(Field::Overlap) => return Vec::new(),
        Some(Field::Declared { location, .. })
            if !empty_list(location.value)
                || location.document.ambiguous_path(&location.site.pointer) =>
        {
            return openapi_deployments(
                Located {
                    site: location.site.clone(),
                    ..*location
                },
                diagnostics,
            );
        }
        _ => {}
    }
    let implicit = serde_json::json!([{"url": "/"}]);
    let servers = entry.value.get("servers").filter(|list| !empty_list(list));
    openapi_deployments(
        Located {
            value: servers.unwrap_or(&implicit),
            document: entry,
            site: entry.site(vec!["servers".into()]),
        },
        diagnostics,
    )
}

fn openapi_deployments(
    servers: Located<'_, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<servers::Deployment> {
    if servers.document.ambiguous_path(&servers.site.pointer)
        || servers.document.duplicates_under(&servers.site.pointer)
    {
        diagnostics.push(Diagnostic::new("duplicate_member", "server", servers.site));
        return Vec::new();
    }
    servers::openapi(
        servers.value,
        servers.document.base.as_ref(),
        &servers.site,
        diagnostics,
    )
}

fn empty_list(value: &Value) -> bool {
    value.as_array().is_some_and(Vec::is_empty)
}

pub(super) fn entry_value(input: &AdmittedSchemaInput<'_>) -> Result<Parsed, SchemaError> {
    if input.entry.tree != input.snapshot.tree() {
        return Err(SchemaError::EntryTree);
    }
    if input.entry.format == crate::revision::Format::Yaml {
        return Err(SchemaError::Yaml);
    }
    parse_json(input.entry.bytes).map_err(SchemaError::Json)
}
