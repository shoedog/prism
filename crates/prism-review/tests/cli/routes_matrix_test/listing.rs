//! Static listing checks, independent of the executed-truth projection.
use super::{
    compare_pins,
    pins::{files_and_spans, resolve_pins},
};
use prism::framework_entries::routes::{MethodSet, Mount, Route, RouterId, Summary};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
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
    let path = items(&route["paths"]).first().unwrap_or(&Value::Null);
    let plugin = &route["origin_plugin"];
    (
        path["normalized"]
            .as_str()
            .or_else(|| path["value"].as_str())
            .or_else(|| route["path"]["raw"].as_str()),
        methods(&route["methods"]),
        site_key(&route["site"]),
        route.get("install_site").map(site_key),
        route["element_ordinal"].as_u64(),
        plugin["file"].as_str().zip(plugin["object_byte"].as_u64()),
        route["framework"].as_str(),
    )
}

fn mount_key(mount: &Value) -> impl Ord + '_ {
    (
        site_key(&mount["site"]),
        mount["alternative_ordinal"].as_u64(),
        mount["child_ordinal"].as_u64(),
    )
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
    for (index, mount) in mounts.iter().enumerate() {
        let field = format!("mount[{index}]");
        check_wire::<Mount>(mount, &field, errors);
        files_and_spans(mount, dir, &field, errors);
        if mount["child"].get("realm").is_some() && mount["child"]["status"] != "router" {
            errors.push(format!("{field}.child.realm: requires a grounded child"));
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
        if route.get("path").is_none() && !candidate {
            errors.push(format!(
                "{field}: missing declaration is limited to opaque candidates"
            ));
        }
        if route["protocol"] == "websocket" && !route["methods"].is_null()
            || route
                .get("default_methods")
                .is_some_and(|set| set.get("known").is_none())
        {
            errors.push(format!("{field}: invalid declared or default methods"));
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
            if path["host_kind"] == "unconstrained" && !items(&path["hosts"]).is_empty() {
                errors.push(format!("{field}.hosts: unconstrained host has no strings"));
            }
        }
        for (position, callback) in items(&route["callbacks"]).iter().enumerate() {
            if (hapi && route.get("path").is_some() && !candidate && callback.get("role").is_none())
                || (!hapi && callback.get("role").is_some())
                || callback["position"] != position
                || (receiver["status"] == "unproven" && callback["status"] != "unresolved")
            {
                errors.push(format!(
                    "{field}.callbacks[{position}]: inconsistent callback"
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

#[test]
fn duplicate_diagnostics_and_changed_failure_frames_fail_exact_comparison() {
    use super::check_roots;
    let source = "é\nregister(p)\n";
    let sources = BTreeMap::from([("app.js".into(), source.into())]);
    let reason = "hapi_registration_unproven";
    let root = json!({
        "file":"app.js","binding":"app","construction_byte":0,"invocations":[
            {"file":"app.js","start_byte":3,"end_byte":14}
        ]
    });
    let row = json!({
        "expect_diagnostics":[{"site":{"file":"app.js","line":2,"text":"register(p)"},"reason":reason}],
        "expect_failed_roots":[{"root":root,"site":{"file":"app.js","line":2,"text":"register(p)"},"reason":reason}]
    });
    let site = json!({"file":"app.js","line":2,"start_byte":3,"end_byte":14});
    let diagnostic = json!({"site":site,"reason":reason});
    let mut summary = json!({
        "diagnostics":[diagnostic],
        "failed_roots":[{"root":root,"site":site,"reason":reason}]
    });
    let mut errors = Vec::new();
    check_events(&row, &summary, &sources, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    summary["diagnostics"]
        .as_array_mut()
        .unwrap()
        .push(diagnostic);
    summary["failed_roots"][0]["root"]["invocations"][0]["start_byte"] = json!(5);
    check_events(&row, &summary, &sources, &mut errors);
    assert_eq!(errors.len(), 2, "{errors:?}");
    let mut exposure = json!({"status":"full","root":root});
    let mut errors = Vec::new();
    check_roots(&[json!({"paths":[exposure]})], &root, &mut errors);
    assert!(errors.is_empty());
    exposure["root"]["binding"] = json!("other");
    check_roots(&[json!({"paths":[exposure]})], &root, &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn wire_requires_nullable_fields_and_failed_root_blocks() {
    let root = json!({"file":"app.js","binding":"app","construction_byte":0,"invocations":[]});
    let site = json!({"file":"app.js","line":1,"start_byte":0,"end_byte":20});
    let mut route = json!({
        "framework":"hapi", "protocol":"http", "site":site,
        "receiver":{"status":"router","file":"app.js","binding":"app","construction_byte":0,"invocations":[]},
        "methods":null, "path":{"raw":"'/ready'","literal":"/ready"},
        "paths":[{"root":root,"status":"full","value":"/ready","path_tokens":[["literal","/ready"]],
            "normalized":"/ready","host_kind":"unconstrained","hosts":[],"version":null,
            "method_guard":"any","binding_blocked":"any","shadowed":null,"strict_slashes":null}],
        "callbacks":[{"position":0,"status":"resolved","role":"pre","group":0,"delegate":null,
            "file":"app.js","name":"prepare","start_line":1,"end_line":1,"start_byte":0,"end_byte":5}]
    });
    let summary = json!({"enabled_adapters":["hapi"],"registrations":1,"proven":1,"unproven":0,"candidates":0,
        "mounts":0,"diagnostics":[],"failed_roots":[{"root":root,"site":site,"reason":"hapi_handler_conflict"}]});
    let mut errors = Vec::new();
    validate(&[route.clone()], &[], &summary, None, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    route["callbacks"][0]
        .as_object_mut()
        .unwrap()
        .remove("delegate");
    route["paths"][0]
        .as_object_mut()
        .unwrap()
        .remove("binding_blocked");
    validate(&[route], &[], &summary, None, &mut errors);
    assert!(
        errors.iter().any(|error| error.contains("noncanonical")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("binding_blocked")),
        "{errors:?}"
    );
}
