use crate::schema::references::{Documents, Resolved};
use crate::schema::{Diagnostic, Site};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn labels(
    documents: &Documents<'_>,
    item: &Resolved<'_, '_>,
    operation: &Value,
    site: &Site,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeSet<String> {
    let mut parameters = BTreeMap::new();
    for (owner, owner_site) in [(item.value, &item.site), (operation, site)] {
        let Some(list) = owner.get("parameters") else {
            continue;
        };
        let base = owner_site.child("parameters");
        let Some(list) = list.as_array() else {
            diagnostics.push(Diagnostic {
                code: "parameters_unreadable",
                scope: "path_parameters",
                site: base,
            });
            continue;
        };
        for (position, parameter) in list.iter().enumerate() {
            let Some(parameter) = documents.resolve(
                item.document,
                parameter,
                base.child(&position.to_string()),
                "path_parameters",
                diagnostics,
            ) else {
                continue;
            };
            if parameter.value["in"] == "path" {
                if let Some(name) = parameter.value["name"].as_str() {
                    parameters.insert(name.to_owned(), parameter.value);
                }
            }
        }
    }
    parameters
        .into_iter()
        .filter_map(|(name, parameter)| {
            let scalar = matches!(
                parameter["schema"]["type"].as_str(),
                Some("string" | "number" | "integer" | "boolean")
            );
            (parameter["style"] == "label" && scalar).then_some(name)
        })
        .collect()
}
