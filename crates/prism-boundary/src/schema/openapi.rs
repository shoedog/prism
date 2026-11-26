mod parameters;

use self::parameters::labels;
use super::references::{Documents, Resolved};
use super::status::{responses, success_key};
use super::{servers, Declaration, Diagnostic, Dialect, Inventory, Operation, SchemaError, Site};
use crate::element::{schema_path, OperationId, SuccessKey};
use crate::revision::AdmittedSchemaInput;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const METHODS: &[&str] = &["delete", "get", "head", "options", "patch", "post", "put"];

pub(crate) fn read_operations(input: &AdmittedSchemaInput<'_>) -> Result<Inventory, SchemaError> {
    let documents = Documents::new(input, entry_value(input)?);
    let document = &documents.entry.value;
    let dialect = match document["openapi"].as_str() {
        Some(version) if version.starts_with("3.0.") || version.starts_with("3.1.") => {
            Dialect::OpenApi3
        }
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
            inventory.diagnostics.push(Diagnostic {
                code: "paths_unreadable",
                scope: "paths",
                site: documents.entry.site(vec!["paths".into()]),
            });
        }
        return Ok(inventory);
    };
    for (path, raw) in paths.iter().collect::<BTreeMap<_, _>>() {
        if !path.starts_with('/') {
            continue;
        }
        let site = documents.entry.site(vec!["paths".into(), path.clone()]);
        let Some(item) = documents.resolve(
            &documents.entry,
            raw,
            site.clone(),
            "path_item",
            &mut inventory.diagnostics,
        ) else {
            continue;
        };
        for method in METHODS
            .iter()
            .copied()
            .chain((dialect == Dialect::OpenApi3).then_some("trace"))
        {
            let Some(value) = item.value.get(method) else {
                continue;
            };
            let operation_site = item.site.child(method);
            if !value.is_object() {
                inventory.diagnostics.push(Diagnostic {
                    code: "operation_unreadable",
                    scope: "operation",
                    site: operation_site,
                });
                continue;
            }
            index_declaration(
                &documents,
                &item,
                &site,
                path,
                method,
                value,
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
    item: &Resolved<'_, '_>,
    referring: &Site,
    raw_path: &str,
    method: &str,
    value: &Value,
    dialect: Dialect,
    inventory: &mut Inventory,
) {
    let site = item.site.child(method);
    let deployments = deployments(
        documents,
        item,
        value,
        &site,
        dialect,
        &mut inventory.diagnostics,
    );
    let label_names = if dialect == Dialect::OpenApi3 && raw_path.contains('{') {
        labels(documents, item, value, &site, &mut inventory.diagnostics)
    } else {
        BTreeSet::new()
    };
    let label_names = label_names.iter().map(String::as_str).collect::<Vec<_>>();
    let declaration = Declaration {
        source_path: raw_path.into(),
        source_method: method.into(),
        path: schema_path(raw_path, "", &label_names),
        referring_site: item
            .chain
            .first()
            .cloned()
            .unwrap_or_else(|| referring.child(method)),
        site: site.clone(),
        reference_chain: item.chain.clone(),
        deployments: Vec::new(),
        value: value.clone(),
    };
    if deployments.is_empty() {
        inventory.unprojected.push(declaration);
        return;
    }
    let mut paths = BTreeMap::new();
    for deployment in deployments {
        let path = schema_path(raw_path, &deployment.path, &label_names);
        let id = OperationId::new(&method.to_ascii_uppercase(), &path);
        paths
            .entry(id)
            .or_insert_with(|| (path, BTreeSet::new()))
            .1
            .insert(deployment);
    }
    let responses = responses(value, &site, &mut inventory.diagnostics);
    let success_keys = responses
        .keys()
        .filter_map(|key| success_key(key, dialect))
        .collect::<Vec<_>>();
    if responses.contains_key("default")
        && !success_keys.contains(&SuccessKey::Range)
        && success_keys.len() < 100
    {
        inventory.diagnostics.push(Diagnostic {
            code: "fallback_success",
            scope: "response_body",
            site: responses["default"].site.clone(),
        });
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
                    responses: responses.clone(),
                    success_keys: success_keys.clone(),
                    dialect,
                });
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                inventory.diagnostics.push(Diagnostic {
                    code: "duplicate_operation",
                    scope: "operation",
                    site: declaration.referring_site.clone(),
                });
                entry.get_mut().declarations.push(declaration);
            }
        }
    }
}

fn deployments(
    documents: &Documents<'_>,
    item: &Resolved<'_, '_>,
    operation: &Value,
    site: &Site,
    dialect: Dialect,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<servers::Deployment> {
    let entry = &documents.entry;
    if dialect == Dialect::Swagger2 {
        return servers::swagger(
            &entry.value,
            operation,
            entry.base.as_ref(),
            site,
            diagnostics,
        );
    }
    if let Some(servers) = operation.get("servers") {
        return servers::openapi(
            servers,
            item.document.base.as_ref(),
            &site.child("servers"),
            diagnostics,
        );
    }
    if let Some(servers) = item.value.get("servers") {
        return servers::openapi(
            servers,
            item.document.base.as_ref(),
            &item.site.child("servers"),
            diagnostics,
        );
    }
    let implicit = serde_json::json!([{"url": "/"}]);
    let servers = entry
        .value
        .get("servers")
        .filter(|list| list.as_array().is_none_or(|list| !list.is_empty()));
    servers::openapi(
        servers.unwrap_or(&implicit),
        entry.base.as_ref(),
        &entry.site(vec!["servers".into()]),
        diagnostics,
    )
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
