//! Static listing checks, independent of the executed-truth projection.
use super::{
    compare_pins,
    pins::{files_and_spans, resolve_pins},
};
use prism::framework_entries::routes::{MethodSet, Mount, Route, RouterId, Summary};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::Hash;
use std::path::Path;

fn check_wire<T: Serialize + DeserializeOwned>(
    value: &Value,
    field: &str,
    errors: &mut Vec<String>,
) {
    match serde_json::from_value::<T>(value.clone()) {
        Ok(decoded) if serde_json::to_value(&decoded).is_ok_and(|encoded| encoded == *value) => {}
        Ok(_) => errors.push(format!(
            "{field}: noncanonical fields or presence in {value}"
        )),
        Err(error) => errors.push(format!("{field}: {error}")),
    }
}

pub(super) fn items(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn site_key(site: &Value) -> (&str, Option<u64>, Option<u64>, Option<u64>) {
    (
        site["file"].as_str().unwrap_or_default(),
        site["start_byte"].as_u64(),
        site["end_byte"].as_u64(),
        site["line"].as_u64(),
    )
}

fn methods(value: &Value) -> Option<MethodSet> {
    serde_json::from_value(value.clone()).ok()
}

pub(super) fn router(value: &Value) -> Option<RouterId> {
    serde_json::from_value(value.clone()).ok()
}

fn path_key(path: &Value) -> impl Ord + '_ {
    let rank = match path["status"].as_str() {
        Some("full") => 0,
        Some("suffix") => 1,
        _ => 2,
    };
    (rank, path["value"].as_str(), router(&path["root"]))
}

fn route_key(route: &Value) -> impl Ord + '_ {
    let paths = items(&route["paths"]);
    let raw = route["path"]["raw"].as_str();
    let spelling = match paths.first() {
        Some(path) => (
            path["normalized"]
                .as_str()
                .or(path["value"].as_str())
                .or(raw),
            None,
        ),
        None => (route["path"]["literal"].as_str().or(raw), raw),
    };
    let plugin = &route["origin_plugin"];
    (
        spelling,
        methods(&route["methods"]),
        site_key(&route["site"]),
        route.get("install_site").map(site_key),
        route["element_ordinal"].as_u64(),
        plugin["file"].as_str().zip(plugin["object_byte"].as_u64()),
        route["framework"].as_str(),
    )
}

fn mount_key(mount: &Value) -> impl Ord + Hash + '_ {
    (
        site_key(&mount["site"]),
        mount["alternative_ordinal"].as_u64(),
        mount["child_ordinal"].as_u64(),
    )
}

fn mount_identity(mount: &Value) -> impl Eq + Hash + '_ {
    let endpoint = |key| {
        let receiver = &mount[key];
        (
            [
                &receiver["file"],
                &receiver["binding"],
                &receiver["construction_byte"],
            ],
            items(&receiver["invocations"]),
            items(&receiver["realm"]),
        )
    };
    (mount_key(mount), endpoint("parent"), endpoint("child"))
}

pub(super) fn validate_mount_pins(mounts: &[Value], errors: &mut Vec<String>) {
    let mut counts = BTreeMap::new();
    for mount in mounts {
        *counts.entry(site_key(&mount["site"])).or_insert(0) += 1;
    }
    let mut seen = HashSet::new();
    for (index, mount) in mounts.iter().enumerate() {
        if counts[&site_key(&mount["site"])] > 1
            && (mount["alternative_ordinal"].as_u64().is_none()
                || mount["child_ordinal"].as_u64().is_none())
        {
            errors.push(format!(
                "expect_mounts[{index}]: shared site requires both source ordinals"
            ));
        } else if !seen.insert(mount_identity(mount)) {
            errors.push(format!(
                "expect_mounts[{index}]: repeated endpoints at the same site require distinct source ordinals"
            ));
        }
    }
}

pub(super) fn validate(
    routes: &[Value],
    mounts: &[Value],
    summary: &Value,
    dir: Option<&Path>,
    errors: &mut Vec<String>,
) {
    check_wire::<Summary>(summary, "summary", errors);
    files_and_spans(summary, dir, "summary", errors);
    if routes
        .windows(2)
        .any(|pair| route_key(&pair[0]) > route_key(&pair[1]))
    {
        errors.push("routes are out of wire order".into());
    }
    if mounts
        .windows(2)
        .any(|pair| mount_key(&pair[0]) > mount_key(&pair[1]))
    {
        errors.push("mounts are out of wire order".into());
    }
    let mut seen_mounts = HashSet::new();
    for (index, mount) in mounts.iter().enumerate() {
        let field = format!("mount[{index}]");
        if !seen_mounts.insert(mount_identity(mount)) {
            errors.push(format!("{field}: duplicate mount identity"));
        }
        check_wire::<Mount>(mount, &field, errors);
        files_and_spans(mount, dir, &field, errors);
        if mount["child"].get("realm").is_some() && mount["child"]["status"] != "router" {
            errors.push(format!("{field}.child.realm: requires a grounded child"));
        }
        if ["parent", "child"]
            .iter()
            .any(|&endpoint| mount[endpoint]["proof"] == "plugin_contract")
        {
            errors.push(format!("{field}: plugin_contract cannot ground a mount"));
        }
        if mount.get("end_guard").is_some() && mount.get("guard_pattern").is_none() {
            errors.push(format!("{field}.end_guard: requires guard_pattern"));
        }
    }
    let failed = items(&summary["failed_roots"]);
    for (index, route) in routes.iter().enumerate() {
        let field = format!("route[{index}]");
        check_wire::<Route>(route, &field, errors);
        files_and_spans(route, dir, &field, errors);
        let receiver = &route["receiver"];
        let candidate = route["candidate"] == true;
        let hapi = route["framework"] == "hapi";
        let flask = route["framework"] == "flask";
        let weak = receiver["proof"] == "plugin_contract";
        if receiver["status"] == "unproven" && (!candidate || !items(&route["paths"]).is_empty()) {
            errors.push(format!(
                "{field}: unproven receiver needs a candidate and empty paths"
            ));
        }
        if weak && (!hapi || route.get("origin_plugin").is_none()) {
            errors.push(format!(
                "{field}: plugin proof requires an identified Hapi object"
            ));
        }
        if route.get("path").is_none()
            && (!candidate
                || !items(&route["paths"]).is_empty()
                || !items(&route["callbacks"]).is_empty())
        {
            errors.push(format!(
                "{field}: missing declaration requires an opaque candidate with no exposures or callbacks"
            ));
        }
        if route["protocol"] == "websocket"
            && (!route["methods"].is_null() || route.get("default_methods").is_some())
            || route
                .get("default_methods")
                .is_some_and(|set| set.get("known").is_none())
        {
            errors.push(format!("{field}: invalid declared or default methods"));
        }
        if route.get("endpoint").is_some() && !flask {
            errors.push(format!("{field}.endpoint: requires Flask"));
        }
        if route.get("origin_plugin").is_some() && !hapi && route["framework"] != "object-table" {
            errors.push(format!(
                "{field}.origin_plugin: requires a Hapi association"
            ));
        }
        if route.get("superseded_keys").is_some()
            && !matches!(route["framework"].as_str(), Some("object-table" | "chi"))
        {
            errors.push(format!(
                "{field}.superseded_keys: requires a Map or chi write"
            ));
        }
        let element = route.get("element_site").is_some();
        let install = route.get("install_site").is_some();
        if (element || install) != route.get("element_ordinal").is_some()
            || (element && install)
            || (element && route["framework"] != "object-table")
            || (install && !hapi)
        {
            errors.push(format!("{field}: invalid table origin fields"));
        }
        let paths = items(&route["paths"]);
        if paths
            .windows(2)
            .any(|pair| path_key(&pair[0]) > path_key(&pair[1]))
        {
            errors.push(format!("{field}.paths: out of wire order"));
        }
        for (position, path) in paths.iter().enumerate() {
            let field = format!("{field}.paths[{position}]");
            if path["status"] == "full" && path["root"].is_null() {
                errors.push(format!("{field}.root: full exposure requires a root"));
            }
            if path.get("strict_slashes").is_some() != flask {
                errors.push(format!("{field}.strict_slashes: required only on Flask"));
            }
            if !path["version"].is_null() && route["framework"] != "nestjs" {
                errors.push(format!("{field}.version: active selectors require NestJS"));
            }
            if weak
                && (!path["root"].is_null()
                    || !(path["status"] == "dynamic"
                        || (path["status"] == "suffix" && path["reason"] == "no_known_mount")))
            {
                errors.push(format!(
                    "{field}: plugin_contract requires a null-root suffix (no_known_mount) or dynamic path"
                ));
            }
            if failed.iter().any(|failed| failed["root"] == path["root"])
                && path["binding_blocked"] != "any"
            {
                errors.push(format!("{field}.binding_blocked: failed root requires any"));
            }
            if path["binding_blocked"]["status"] == "unknown" {
                errors.push(format!(
                    "{field}.binding_blocked: expected a known set or any"
                ));
            }
            let refusal = path["mount_refusal"].as_str();
            if weak != refusal.is_some()
                || refusal.is_some_and(|value| {
                    !matches!(value, "no_known_mount" | "hapi_registration_unproven")
                })
            {
                errors.push(format!(
                    "{field}.mount_refusal: required only for weak exposures"
                ));
            }
            let expected_normalized = path["path_tokens"].as_array().map(|tokens| {
                tokens
                    .iter()
                    .map(|token| token[1].as_str().unwrap_or("{}"))
                    .collect::<String>()
            });
            if path["normalized"] != json!(expected_normalized) {
                errors.push(format!("{field}.normalized: does not render path_tokens"));
            }
            if path["status"] == "dynamic" && !path["path_tokens"].is_null() {
                errors.push(format!("{field}.path_tokens: dynamic path has no key"));
            }
            if path["host_kind"] == "unconstrained" && !items(&path["hosts"]).is_empty() {
                errors.push(format!("{field}.hosts: unconstrained host has no strings"));
            }
            if path["host_kind"] != "unconstrained"
                && !matches!(
                    route["framework"].as_str(),
                    Some("flask" | "hapi" | "nestjs" | "net/http")
                )
            {
                errors.push(format!("{field}.host_kind: framework has no host selector"));
            }
        }
        let callbacks = items(&route["callbacks"]);
        for (position, callback) in callbacks.iter().enumerate() {
            if (hapi && route.get("path").is_some() && !candidate && callback.get("role").is_none())
                || (!hapi && callback.get("role").is_some())
                || callback["position"] != position
                || (receiver["status"] == "unproven" && callback["status"] != "unresolved")
            {
                errors.push(format!(
                    "{field}.callbacks[{position}]: inconsistent callback"
                ));
            }
            let last = position + 1 == callbacks.len();
            if hapi
                && ((!last && callback["role"] == "handler")
                    || (last && !candidate && callback["role"] != "handler"))
            {
                errors.push(format!(
                    "{field}.callbacks[{position}].role: handler must be last"
                ));
            }
            if receiver["status"] == "unproven" && callback["reason"] != receiver["reason"] {
                errors.push(format!(
                    "{field}.callbacks[{position}].reason: must match receiver reason"
                ));
            }
            if !callback["delegate"].is_null()
                && (callback["status"] != "unresolved" || callback["reason"] != "wrapper_call")
            {
                errors.push(format!(
                    "{field}.callbacks[{position}].delegate: requires wrapper_call"
                ));
            }
        }
        for (key, reason) in [
            ("superseded_keys", "superseded"),
            ("conflicts", "dispatch_conflict"),
        ] {
            if items(&route[key]).iter().any(|cut| cut["reason"] != reason) {
                errors.push(format!("{field}.{key}: invalid reason"));
            }
        }
    }
}

fn fits(want: &Value, got: &Value) -> bool {
    let mut errors = Vec::new();
    compare_pins(want, Some(got), "", &mut errors);
    errors.is_empty()
}

pub(super) fn compare_collection(
    expected: &Value,
    actual: &[Value],
    sources: &BTreeMap<String, String>,
    field: &str,
    errors: &mut Vec<String>,
) {
    let mut want = items(expected).to_vec();
    for (index, record) in want.iter_mut().enumerate() {
        resolve_pins(record, sources, &format!("{field}[{index}]"), errors);
    }
    if want.len() != actual.len() {
        errors.push(format!(
            "{field}: expected {} items, got {}",
            want.len(),
            actual.len()
        ));
    }
    let mut pairs = vec![None; want.len()];
    let mut remaining: BTreeSet<_> = (0..actual.len()).collect();
    // Full partial-pin matches disambiguate shared sites (mount alternatives,
    // consumed elements, and declarations written on the same source line).
    for exact in [true, false] {
        for (pin, pair) in want.iter().zip(&mut pairs) {
            if pair.is_some() {
                continue;
            }
            let mut candidates = remaining.iter().copied().filter(|&index| {
                let got = &actual[index];
                if exact {
                    fits(pin, got)
                } else {
                    [
                        "site",
                        "element_site",
                        "install_site",
                        "element_ordinal",
                        "origin_plugin",
                        "alternative_ordinal",
                        "child_ordinal",
                    ]
                    .iter()
                    .all(|&key| pin.get(key).is_none_or(|value| fits(value, &got[key])))
                }
            });
            let Some(index) = candidates.next() else {
                continue;
            };
            if exact || candidates.next().is_none() {
                *pair = Some(index);
                remaining.remove(&index);
            }
        }
    }
    // Preserve field-level diagnostics for a single changed origin.
    let mut unmatched = pairs.iter_mut().filter(|pair| pair.is_none());
    if let (Some(pair), None, 1) = (unmatched.next(), unmatched.next(), remaining.len()) {
        *pair = remaining.pop_first();
    }
    for (index, (record, pair)) in want.iter().zip(pairs).enumerate() {
        let field = format!("{field}[{index}]");
        if let Some(matched) = pair {
            compare_pins(record, Some(&actual[matched]), &field, errors);
        } else {
            errors.push(format!("{field}: missing origin {}", record["site"]));
        }
    }
    for index in remaining {
        errors.push(format!(
            "{field}: unexpected origin {}",
            actual[index]["site"]
        ));
    }
}

pub(super) fn check_events(
    row: &Value,
    summary: &Value,
    sources: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    for (pin, field) in [
        ("expect_diagnostics", "diagnostics"),
        ("expect_failed_roots", "failed_roots"),
    ] {
        let mut want = items(&row[pin]).to_vec();
        for (index, item) in want.iter_mut().enumerate() {
            resolve_pins(item, sources, &format!("{pin}[{index}]"), errors);
        }
        let mut got = items(&summary[field]).to_vec();
        want.sort_by_cached_key(Value::to_string);
        got.sort_by_cached_key(Value::to_string);
        if want != got {
            errors.push(format!("summary.{field}: expected {want:?}, got {got:?}"));
        }
    }
}

#[cfg(test)]
mod tests;
