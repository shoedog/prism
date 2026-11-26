mod uri;

pub(super) use self::uri::Uri;
use self::uri::{valid_scheme, Origin};
use super::{Diagnostic, Site};
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Deployment {
    pub path: String,
    pub origin: Origin,
    pub site: Site,
    pub declared_url: String,
    pub expanded_url: String,
}

pub(super) fn swagger(
    document: &Value,
    operation: &Value,
    base: Option<&Uri>,
    site: &Site,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Deployment> {
    let inherited = base.map(|uri| &uri.origin);
    let host = match document.get("host") {
        Some(host) => host
            .as_str()
            .filter(|h| !h.is_empty())
            .map(str::to_ascii_lowercase),
        None => inherited.and_then(|origin| origin.authority.clone()),
    };
    let schemes = operation.get("schemes").or_else(|| document.get("schemes"));
    let mut origins = match schemes {
        Some(schemes) => schemes
            .as_array()
            .into_iter()
            .flatten()
            .map(|scheme| {
                scheme
                    .as_str()
                    .filter(|s| valid_scheme(s))
                    .map(str::to_ascii_lowercase)
            })
            .collect::<Vec<_>>(),
        None => vec![inherited.and_then(|origin| origin.scheme.clone())],
    };
    if origins.is_empty() {
        origins.push(None);
    }
    if host.is_none() || origins.contains(&None) {
        diagnostics.push(Diagnostic::new(
            "server_origin_unchecked",
            "origin",
            site.clone(),
        ));
    }
    origins
        .into_iter()
        .map(|scheme| Deployment {
            path: document["basePath"]
                .as_str()
                .filter(|path| !path.is_empty())
                .unwrap_or("/")
                .into(),
            origin: Origin {
                scheme,
                authority: host.clone(),
            },
            site: site.clone(),
            declared_url: String::new(),
            expanded_url: String::new(),
        })
        .collect()
}

pub(super) fn openapi(
    list: &Value,
    base: Option<&Uri>,
    site: &Site,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Deployment> {
    let mut deployments = Vec::new();
    let Some(servers) = list.as_array() else {
        diagnostics.push(Diagnostic::new(
            "server_list_unchecked",
            "server",
            site.clone(),
        ));
        return deployments;
    };
    for (position, server) in servers.iter().enumerate() {
        let mut site = site.clone();
        site.pointer.push(position.to_string());
        let mut deployments_for_server =
            |code, scope| diagnostics.push(Diagnostic::new(code, scope, site.clone()));
        let Some(raw) = server["url"].as_str() else {
            deployments_for_server("server_url_unchecked", "server");
            continue;
        };
        for expanded in expand(raw, &server["variables"], &mut deployments_for_server) {
            let Some(uri) = Uri::parse(&expanded) else {
                deployments_for_server("server_url_unchecked", "server");
                continue;
            };
            let Some(resolved) = uri.resolve(base) else {
                deployments_for_server("retrieval_base_unchecked", "server");
                continue;
            };
            if resolved.origin.scheme.is_none() || resolved.origin.authority.is_none() {
                deployments_for_server("server_origin_unchecked", "origin");
            }
            deployments.push(Deployment {
                path: resolved.path,
                origin: resolved.origin,
                site: site.clone(),
                declared_url: raw.into(),
                expanded_url: expanded,
            });
        }
    }
    if deployments.is_empty() && !servers.is_empty() {
        diagnostics.push(Diagnostic::new(
            "server_deployments_unchecked",
            "server",
            site.clone(),
        ));
    }
    deployments
}

fn expand(
    template: &str,
    variables: &Value,
    limit: &mut impl FnMut(&'static str, &'static str),
) -> Vec<String> {
    static VARIABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^{}]+)\}").unwrap());
    let mut values = BTreeMap::new();
    for matched in VARIABLE.captures_iter(template) {
        let name = matched.get(1).unwrap().as_str();
        if values.contains_key(name) {
            continue;
        }
        let variable = &variables[name];
        let mut choices = Vec::new();
        if let Some(default) = variable["default"].as_str() {
            choices.push(default);
        }
        if let Some(finite) = variable["enum"].as_array().filter(|list| !list.is_empty()) {
            choices.extend(finite.iter().filter_map(Value::as_str));
            if finite.iter().any(|value| !value.is_string()) {
                limit("server_variable_unchecked", "server");
            }
        } else {
            limit("server_variable_unchecked", "server");
        }
        choices.sort_unstable();
        choices.dedup();
        values.insert(name, choices);
    }
    let mut selections = vec![BTreeMap::new()];
    for (name, choices) in values {
        selections = selections
            .into_iter()
            .flat_map(|selection| {
                choices.iter().map(move |value| {
                    let mut choice = selection.clone();
                    choice.insert(name, *value);
                    choice
                })
            })
            .collect();
    }
    selections
        .into_iter()
        .map(|selection| {
            VARIABLE
                .replace_all(template, |m: &regex::Captures<'_>| selection[&m[1]])
                .into_owned()
        })
        .collect()
}
