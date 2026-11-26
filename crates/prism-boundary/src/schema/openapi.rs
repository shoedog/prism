mod parameters;

use self::parameters::labels;
use super::references::{Documents, Field, Located, PathItem};
use super::status::read_statuses;
use super::{servers, Declaration, Diagnostic, Dialect, Inventory, Operation, SchemaError, Site};
use crate::element::{schema_path, OperationId};
use crate::revision::AdmittedSchemaInput;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const METHODS: &[&str] = &["delete", "get", "head", "options", "patch", "post", "put"];

pub(crate) fn read_operations(input: &AdmittedSchemaInput<'_>) -> Result<Inventory, SchemaError> {
    let documents = Documents::new(input, entry_value(input)?);
    let document = &documents.entry.value;
    let dialect = match document["openapi"].as_str() {
        Some(version) if version.starts_with("3.0.") => Dialect::OpenApi30,
        Some(version) if version.starts_with("3.1.") => Dialect::OpenApi31,
        _ if document["swagger"] == "2.0" => Dialect::Swagger2,
        _ => return Err(SchemaError::Dialect),
    };
    let mut inventory = Inventory {
        api: input.api.into(),
        authority: input.authority.into(),
        digest: Sha256::digest(input.entry.bytes).into(),
        info_version: document["info"]["version"].as_str().map(str::to_owned),
        operations: BTreeMap::new(),
        unprojected: Vec::new(),
        diagnostics: Vec::new(),
    };
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
        let Some(item) = PathItem::resolve(
            &documents,
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
                inventory.diagnostics.push(Diagnostic::new(
                    "operation_unreadable",
                    "operation",
                    location.site.clone(),
                ));
                continue;
            }
            index_declaration(
                &documents,
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
    let (responses, success_keys) = read_statuses(value, site, dialect, &mut inventory.diagnostics);
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
        reference_chain: chain.to_vec(),
        deployments: Vec::new(),
        responses,
        success_keys,
        value: (*value).clone(),
    };
    if deployments.is_empty() || label_names.is_none() {
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
                    operation_id: value["operationId"].as_str().map(str::to_owned),
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
        return servers::swagger(
            &entry.value,
            operation.value,
            entry.base.as_ref(),
            &operation.site,
            diagnostics,
        );
    }
    if let Some(servers) = operation
        .child("servers")
        .filter(|list| !empty_list(list.value))
    {
        return servers::openapi(
            servers.value,
            servers.document.base.as_ref(),
            &servers.site,
            diagnostics,
        );
    }
    match item.field("servers") {
        Some(Field::Overlap) => return Vec::new(),
        Some(Field::Declared { location, .. }) if !empty_list(location.value) => {
            return servers::openapi(
                location.value,
                location.document.base.as_ref(),
                &location.site,
                diagnostics,
            );
        }
        _ => {}
    }
    let implicit = serde_json::json!([{"url": "/"}]);
    let servers = entry.value.get("servers").filter(|list| !empty_list(list));
    servers::openapi(
        servers.unwrap_or(&implicit),
        entry.base.as_ref(),
        &entry.site(vec!["servers".into()]),
        diagnostics,
    )
}

fn empty_list(value: &Value) -> bool {
    value.as_array().is_some_and(Vec::is_empty)
}

fn entry_value(input: &AdmittedSchemaInput<'_>) -> Result<Value, SchemaError> {
    if input.entry.tree != input.snapshot.tree() {
        return Err(SchemaError::EntryTree);
    }
    if input.entry.format == crate::revision::Format::Yaml {
        return Err(SchemaError::Yaml);
    }
    serde_json::from_slice(input.entry.bytes).map_err(SchemaError::Json)
}
