use crate::element::CanonicalPath;
use crate::schema::references::{Documents, Field, Located, PathItem};
use crate::schema::{Diagnostic, Dialect};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn labels<'p>(
    documents: &Documents<'_>,
    item: &PathItem<'_, '_>,
    operation: &Located<'_, '_>,
    path: &'p CanonicalPath,
    dialect: Dialect,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<&'p str>> {
    let inherited = match item.field("parameters") {
        Some(Field::Declared { location, .. }) => Some(location),
        Some(Field::Overlap) => return None,
        None => None,
    };
    let local = operation.child("parameters");
    let mut parameters = BTreeMap::new();
    for owner in inherited.into_iter().chain(local.as_ref()) {
        let Some(list) = owner.value.as_array() else {
            diagnostics.push(Diagnostic::new(
                "parameters_unreadable",
                "path_parameters",
                owner.site.clone(),
            ));
            continue;
        };
        for (position, parameter) in list.iter().enumerate() {
            let parameter = match documents.resolve(Located {
                value: parameter,
                document: owner.document,
                site: owner.site.child(&position.to_string()),
            }) {
                Ok(mut chain) => chain.pop().unwrap(),
                Err(failure) => {
                    diagnostics.push(Diagnostic::new(
                        failure.code,
                        "path_parameters",
                        failure.site,
                    ));
                    continue;
                }
            };
            if parameter.value["in"] == "path" {
                if let Some(name) = parameter.value["name"].as_str() {
                    parameters.insert(name, parameter);
                }
            }
        }
    }
    let mut labels = Vec::new();
    for name in path.captures().map(|capture| capture.name.as_str()) {
        let Some(parameter) = parameters.get(name).filter(|p| p.value["style"] == "label") else {
            continue;
        };
        match scalar_schema(documents, parameter, dialect) {
            Some(true) => labels.push(name),
            Some(false) => {}
            None => {
                diagnostics.push(Diagnostic::new(
                    "path_parameter_style_unchecked",
                    "path_parameters",
                    item.site.clone(),
                ));
                return None;
            }
        }
    }
    Some(labels)
}

fn scalar_schema(
    documents: &Documents<'_>,
    parameter: &Located<'_, '_>,
    dialect: Dialect,
) -> Option<bool> {
    let schemas = documents.resolve(parameter.child("schema")?).ok()?;
    if dialect == Dialect::OpenApi30 {
        return scalar_type(schemas.last()?.value["type"].as_str()?);
    }
    let mut scalar = None;
    for schema in schemas {
        let Some(declared) = schema.value.get("type") else {
            continue;
        };
        let kind = match declared {
            Value::String(kind) => scalar_type(kind)?,
            Value::Array(types) => {
                let mut types = types.iter().filter(|kind| *kind != "null");
                let first = scalar_type(types.next()?.as_str()?)?;
                if !types.all(|kind| kind.as_str().and_then(scalar_type) == Some(first)) {
                    return None;
                }
                first
            }
            _ => return None,
        };
        if scalar.is_some_and(|previous| previous != kind) {
            return None;
        }
        scalar = Some(kind);
    }
    scalar
}

fn scalar_type(kind: &str) -> Option<bool> {
    match kind {
        "string" | "number" | "integer" | "boolean" => Some(true),
        "array" | "object" => Some(false),
        _ => None,
    }
}
