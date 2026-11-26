use crate::element::CanonicalPath;
use crate::schema::references::{contains_member, Documents, Field, Located, PathItem};
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
        Some(Field::Declared { location, .. }) => {
            ParameterList::read(documents, Some(location), diagnostics)
        }
        Some(Field::Overlap) => ParameterList {
            entries: BTreeMap::new(),
            complete: false,
        },
        None => ParameterList::read(documents, None, diagnostics),
    };
    let local = ParameterList::read(
        documents,
        operation.child("parameters").as_ref(),
        diagnostics,
    );
    let both_complete = inherited.complete && local.complete;
    let mut labels = Vec::new();
    let mut projectable = true;
    for name in path.captures().map(|capture| capture.name.as_str()) {
        let key = (name, "path");
        let (parameter, readable) = match local.entries.get(&key) {
            Some(parameter) => (Some(parameter), local.complete),
            None => (inherited.entries.get(&key), both_complete),
        };
        // An unread list at or above the effective precedence can hide a style override.
        let serialization = match (parameter, readable) {
            (_, false) => PathSerialization::Unchecked,
            (None, true) => PathSerialization::Simple,
            (Some(parameter), true) => {
                path_serialization(parameter, operation, dialect, both_complete)
            }
        };
        let code = match serialization {
            PathSerialization::Simple => continue,
            PathSerialization::ScalarLabel => {
                labels.push(name);
                continue;
            }
            PathSerialization::Unsupported => "path_style_unsupported",
            PathSerialization::Unchecked => "path_parameter_style_unchecked",
        };
        diagnostics.push(Diagnostic {
            capture: Some(name.into()),
            ..Diagnostic::new(code, "path_parameters", item.site.clone())
        });
        projectable = false;
    }
    projectable.then_some(labels)
}

struct ParameterList<'r, 'a> {
    entries: BTreeMap<(&'r str, &'r str), Located<'r, 'a>>,
    complete: bool,
}

impl<'r, 'a> ParameterList<'r, 'a> {
    fn read(
        documents: &'r Documents<'a>,
        owner: Option<&Located<'r, 'a>>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut result = Self {
            entries: BTreeMap::new(),
            complete: true,
        };
        let Some(owner) = owner else {
            return result;
        };
        let Some(list) = owner.value.as_array() else {
            diagnostics.push(Diagnostic::new(
                "parameters_unreadable",
                "path_parameters",
                owner.site.clone(),
            ));
            result.complete = false;
            return result;
        };
        for (position, value) in list.iter().enumerate() {
            let declaration = Located {
                value,
                document: owner.document,
                site: owner.site.child(&position.to_string()),
            };
            // Resolve the entry only to check its admission and read its override key.
            // Keep the original declaration so a resolved reference cannot qualify as inline.
            let resolved = if value.get("$ref").is_some() {
                match documents.resolve(Located {
                    site: declaration.site.clone(),
                    ..declaration
                }) {
                    Ok(chain) => chain.last().unwrap().value,
                    Err(failure) => {
                        diagnostics.push(Diagnostic::new(
                            failure.code,
                            "path_parameters",
                            failure.site,
                        ));
                        result.complete = false;
                        continue;
                    }
                }
            } else {
                value
            };
            let key = match (resolved["name"].as_str(), resolved["in"].as_str()) {
                (Some(name), Some(location @ ("path" | "query" | "header" | "cookie"))) => {
                    (name, location)
                }
                _ => {
                    diagnostics.push(Diagnostic::new(
                        "parameter_unreadable",
                        "path_parameters",
                        declaration.site,
                    ));
                    result.complete = false;
                    continue;
                }
            };
            if result.entries.insert(key, declaration).is_some() {
                result.complete = false;
            }
        }
        result
    }
}

enum PathSerialization {
    Simple,
    ScalarLabel,
    Unsupported,
    Unchecked,
}

fn path_serialization(
    parameter: &Located<'_, '_>,
    operation: &Located<'_, '_>,
    dialect: Dialect,
    both_lists_complete: bool,
) -> PathSerialization {
    if parameter.value.get("$ref").is_some() {
        return PathSerialization::Unchecked;
    }
    let serialization = match parameter.value.get("style").map(Value::as_str) {
        None | Some(Some("simple")) => PathSerialization::Simple,
        Some(Some("label")) => match label_type(parameter, operation, dialect, both_lists_complete)
        {
            Some("string" | "number" | "integer" | "boolean") => PathSerialization::ScalarLabel,
            Some("array" | "object") => return PathSerialization::Unsupported,
            _ => return PathSerialization::Unchecked,
        },
        _ => return PathSerialization::Unsupported,
    };
    // Either boolean explode value preserves the capture spelling for these styles.
    if parameter
        .value
        .get("explode")
        .is_some_and(|value| !value.is_boolean())
    {
        return PathSerialization::Unsupported;
    }
    serialization
}

fn label_type<'r>(
    parameter: &Located<'r, '_>,
    operation: &Located<'_, '_>,
    dialect: Dialect,
    both_lists_complete: bool,
) -> Option<&'r str> {
    // Labels require both lists to be readable, including shadowed entries, and
    // a parameter in the operation's document. Withhold if that document has a
    // resolution or dialect keyword, the schema contains a reference, or its type
    // is absent or malformed. The caller checks the resulting non-null type name.
    if !both_lists_complete
        || !std::ptr::eq(parameter.document, operation.document)
        || parameter
            .document
            .has_schema_resolution_or_dialect_keywords()
    {
        return None;
    }
    let schema = parameter.value.get("schema")?;
    if contains_member(schema, &["$ref"]) {
        return None;
    }
    let kind = match schema.get("type")? {
        Value::Array(types) if dialect == Dialect::OpenApi31 => match types.as_slice() {
            [kind] => kind,
            [first, kind] if first == "null" => kind,
            [kind, last] if last == "null" => kind,
            _ => return None,
        },
        kind => kind,
    };
    kind.as_str()
}
