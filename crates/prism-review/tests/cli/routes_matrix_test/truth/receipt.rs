//! Check the saved receipt's raw calls, served records, and derived projection.
use super::{items, native_tokens};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(row: &Value, truth: &Value, digest: &str) -> Result<(), String> {
    if truth["id"] != row["id"] {
        return Err("truth.id: receipt belongs to another row".into());
    }
    if truth["fixture_sha256"] != digest {
        return Err(format!("truth.fixture_sha256: fixture hashes to {digest}"));
    }
    let (framework, kinds): (&str, &[_]) = match truth["framework"].as_str() {
        Some("express") => ("express", &["route", "use", "verb"]),
        Some("fastapi") => ("fastapi", &["api_route", "websocket"]),
        Some("flask") => ("flask", &["url_rule"]),
        _ => return Err("truth.framework: expected an executed adapter".into()),
    };
    if items(&row["expect"])
        .iter()
        .any(|pin| pin.get("framework").is_some_and(|value| value != framework))
    {
        return Err("truth.framework: disagrees with the source pins".into());
    }
    let mut calls = BTreeMap::new();
    let mut ordinals = BTreeMap::new();
    for (index, call) in items(&truth["calls"]).iter().enumerate() {
        let id = (index + 1) as u64;
        let site = &call["site"];
        let ordinal = ordinals
            .entry((site["file"].as_str(), site["line"].as_u64()))
            .or_insert(0);
        *ordinal += 1;
        let origin = call.get("origin_call_id");
        let kind = &call["kind"];
        if call["id"] != id
            || kind.as_str().is_none_or(|name| !kinds.contains(&name))
            || site["ordinal"] != *ordinal
            || call["mounted"] != origin.is_some()
            || (framework != "express" && call["framework"] != framework)
            || origin.is_some_and(|origin| {
                origin
                    .as_u64()
                    .and_then(|id| calls.get(&id))
                    .is_none_or(|prior: &&Value| &prior["kind"] != kind)
            })
        {
            return Err(format!(
                "truth.calls[{index}]: invalid sequential call, ordinal, or origin"
            ));
        }
        let args = &call["args"];
        let path = args.get(if framework == "flask" { "rule" } else { "path" });
        let Some(callbacks) = call["callbacks"].as_array() else {
            return Err(format!("truth.calls[{index}].callbacks: expected array"));
        };
        if path.is_none()
            || (call["path"]["literal"].is_string() && path != call["path"].get("literal"))
        {
            return Err(format!(
                "truth.calls[{index}].args: missing or contradictory path"
            ));
        }
        if call
            .get("path")
            .is_some_and(|path| !path["raw"].is_string() || path.get("literal").is_none())
        {
            return Err(format!(
                "truth.calls[{index}].path: missing raw or nullable literal"
            ));
        }
        if !matches!(kind.as_str(), Some("use" | "route"))
            && !args.get("methods").is_some_and(|value| {
                value.is_null()
                    || value == "ANY"
                    || value
                        .as_array()
                        .is_some_and(|methods| methods.iter().all(Value::is_string))
            })
        {
            return Err(format!(
                "truth.calls[{index}].args.methods: expected native method declaration"
            ));
        }
        if callbacks
            .iter()
            .any(|callback| callback["kind"] == "javascript" && !callback["anonymous"].is_boolean())
        {
            return Err(format!(
                "truth.calls[{index}].callbacks: missing JavaScript anonymity marker"
            ));
        }
        if let Some(prior) = origin.and_then(Value::as_u64).and_then(|id| calls.get(&id)) {
            if call["callbacks"] != prior["callbacks"]
                || args.get("methods").is_none()
                || args.get("methods") != prior["args"].get("methods")
            {
                return Err(format!("truth.calls[{index}].origin_call_id: copy changed callbacks or declared methods"));
            }

            if framework == "fastapi" {
                let prior = &prior["args"];
                let source = prior["router_prefix"]
                    .as_str()
                    .zip(prior["path"].as_str())
                    .map(|(prefix, path)| format!("{prefix}{path}"));
                if !source.is_some_and(|suffix| {
                    args["path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with(&suffix))
                }) {
                    return Err(format!(
                        "truth.calls[{index}].args.path: copy lost its native source path"
                    ));
                }
            }
        }
        if kind == "use" {
            let has_path = args["has_path_arg"].as_bool();
            let prefix = if has_path == Some(true) {
                path.and_then(Value::as_str)
            } else {
                Some("")
            };
            if has_path.is_none()
                || callbacks.is_empty()
                || (call["mount"] == true
                    && (prefix.is_none() || call["mount_prefix"].as_str() != prefix))
                || (has_path == Some(false) && args["path"] != "/")
            {
                return Err(format!(
                    "truth.calls[{index}]: invalid Express use arguments or callbacks"
                ));
            }
        }
        if kind == "route" && !callbacks.is_empty() {
            return Err(format!(
                "truth.calls[{index}].callbacks: route builder has no callbacks"
            ));
        }
        calls.insert(id, call);
    }
    if items(&row["expect"]).is_empty() && super::runtime_only_count(row).is_none() {
        if let Some(want) = row["expect_unlisted"].as_u64() {
            let count = calls
                .values()
                .filter(|call| call["kind"] == "use" && call["mount"] != true)
                .count() as u64;
            if count != want {
                return Err(format!(
                    "truth.calls: expected {want} unlisted middleware calls, got {count}"
                ));
            }
        }
    }

    let mut paths: BTreeMap<u64, Vec<&str>> = BTreeMap::new();
    for (index, served) in items(&truth["served"]).iter().enumerate() {
        let linked: Vec<_> = items(&served["call_ids"])
            .iter()
            .map(|id| calls[&id.as_u64().expect("checked registration link")])
            .collect();
        let first = linked[0];
        let kind = if first["kind"] == "websocket" {
            "websocket"
        } else {
            "http"
        };
        let declared = if framework == "express" {
            let legs: Vec<_> = linked
                .iter()
                .filter(|call| call["kind"] == "verb")
                .collect();
            if !served["path"].is_string()
                && !legs
                    .iter()
                    .any(|call| call["args"]["path"] == served["path"])
            {
                return Err(format!(
                    "truth.served[{index}].path: disagrees with linked regex"
                ));
            }
            if legs.iter().any(|call| call["args"]["methods"] == "ANY") {
                json!("ANY")
            } else {
                let methods: BTreeSet<_> = legs
                    .iter()
                    .flat_map(|call| items(&call["args"]["methods"]))
                    .filter_map(Value::as_str)
                    .collect();
                json!(methods)
            }
        } else {
            let mut previous = None;
            for call in &linked {
                if call.get("origin_call_id") != previous {
                    return Err(format!(
                        "truth.served[{index}].call_ids: incomplete or reordered copy chain"
                    ));
                }
                previous = call.get("id");
            }
            let last = &linked.last().unwrap()["args"];
            let path = if framework == "flask" {
                last["rule"].as_str().map(str::to_owned)
            } else {
                last["router_prefix"]
                    .as_str()
                    .zip(last["path"].as_str())
                    .map(|(prefix, path)| format!("{prefix}{path}"))
            };
            if path.is_none() || path.as_deref() != served["path"].as_str() {
                return Err(format!(
                    "truth.served[{index}].path: disagrees with installed call"
                ));
            }
            if framework == "flask" {
                if let Some(callback) = items(&first["callbacks"]).first() {
                    if served.get("runtime_callback") != Some(callback) {
                        return Err(format!("truth.served[{index}].runtime_callback: disagrees with linked callback"));
                    }
                }
            }
            normalize_methods(&first["args"]["methods"])
        };
        if served["kind"] != kind || served.get("declared_methods") != Some(&declared) {
            return Err(format!(
                "truth.served[{index}]: protocol or declared methods disagree with linked calls"
            ));
        }
        if let Some(path) = served["path"].as_str() {
            for call in linked {
                let exposed = paths.entry(call["id"].as_u64().unwrap()).or_default();
                if !exposed.contains(&path) {
                    exposed.push(path);
                }
            }
        }
    }

    let pinned_regex = items(&row["expect"]).iter().any(|pin| {
        items(&pin["paths"])
            .iter()
            .any(|path| path["reason"] == "regex_path")
    });
    if pinned_regex
        && calls.values().any(|call| {
            call["kind"] == "verb"
                && call["args"]["path"].is_object()
                && !items(&truth["served"])
                    .iter()
                    .any(|route| items(&route["call_ids"]).contains(&call["id"]))
        })
    {
        return Err("truth.served: missing served regex evidence".into());
    }

    let mut projected = Vec::new();
    for (id, call) in calls {
        if call["mounted"] == true || matches!(call["kind"].as_str(), Some("route" | "use")) {
            continue;
        }
        let exposed: Vec<_> = paths
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|path| {
                let mut exposure = json!({"status":"full","value":path});
                if let Some(normalized) = projected_normalized(framework, path) {
                    exposure["normalized"] = json!(normalized);
                }
                exposure
            })
            .collect();
        let mut record = json!({
            "call_id":id, "framework":framework, "site":{"file":call["site"]["file"],"line":call["site"]["line"]},
            "paths":exposed,
        });
        if let Some(path) = call.get("path").filter(|path| !path.is_null()) {
            record["path"] = path.clone();
        }
        let methods = &call["args"]["methods"];
        if methods == "ANY" {
            record["methods"] = json!({"any":true});
        } else if !methods.is_null() {
            record["methods"] = json!({"known":normalize_methods(methods)});
        }
        let Some(raw) = call["callbacks"].as_array() else {
            return Err(format!("truth.calls[{id}].callbacks: expected array"));
        };
        let callbacks = raw
            .iter()
            .enumerate()
            .map(|(position, callback)| {
                let file = callback["file"].as_str()?;
                if file.starts_with('<')
                    || !matches!(callback["kind"].as_str(), Some("python" | "javascript"))
                {
                    return None;
                }
                let mut projected = json!({
                    "position":position, "file":file, "name":callback["name"],
                    "start_line":callback["first_line"],
                    "status":if callback["anonymous"] == true { "anonymous" } else { "resolved" },
                });
                if let Some(end) = callback.get("end_line") {
                    projected["end_line"] = end.clone();
                }
                Some(projected)
            })
            .collect::<Option<Vec<_>>>();
        if let Some(callbacks) = callbacks {
            record["callbacks"] = json!(callbacks);
        }
        projected.push(record);
    }
    if truth["projection"] != json!(projected) {
        return Err("truth.projection: disagrees with its raw calls or served paths".into());
    }
    Ok(())
}

fn normalize_methods(methods: &Value) -> Value {
    if let Some(methods) = methods.as_array() {
        let sorted: BTreeSet<_> = methods
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_uppercase)
            .collect();
        json!(sorted)
    } else {
        methods.clone()
    }
}

// Match run_truth.normalize, including its legacy display-only grammar.
fn projected_normalized(framework: &str, path: &str) -> Option<String> {
    if framework == "express" {
        let tokens = native_tokens(framework, path)?;
        return tokens
            .iter()
            .map(|token| {
                let text = token[1].as_str().unwrap_or("{}");
                (!text.contains(':')).then_some(text)
            })
            .collect();
    }
    let (open, close, converters): (char, char, &[_]) = match framework {
        "flask" => ('<', '>', &["string", "int", "float", "uuid"]),
        "fastapi" => ('{', '}', &["str", "int", "float", "uuid"]),
        _ => return None,
    };
    let mut output = String::new();
    let mut copied = 0;
    for (start, _) in path.match_indices(open) {
        if start < copied {
            continue;
        }
        let after = &path[start + 1..];
        let Some(end) = after.find(close) else {
            break;
        };
        let body = &after[..end];
        let (name, converter) = match body.split_once(':') {
            Some((converter, name)) if framework == "flask" => (name, Some(converter)),
            Some((name, converter)) => (name, Some(converter)),
            None => (body, None),
        };
        let mut chars = name.chars();
        if converter.is_some_and(|converter| !converters.contains(&converter))
            || !chars
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            || !chars.all(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        output.push_str(&path[copied..start]);
        output.push_str("{}");
        copied = start + 1 + end + 1;
    }
    output.push_str(&path[copied..]);
    let valid = if framework == "flask" {
        !output.contains(['<', '>'])
    } else {
        !output.replace("{}", "").contains(['{', '}'])
    };
    valid.then_some(output)
}

#[test]
fn projected_display_retains_the_runner_grammar() {
    for (framework, path, expected) in [
        ("flask", "/items/<any(red,blue):color>", None),
        ("flask", "/items/<int(min=1):id>", None),
        ("flask", "/items/<int:id_é>", Some("/items/{}")),
        ("fastapi", "/items/{draft-name}", None),
        ("fastapi", "/items/{}", Some("/items/{}")),
        ("express", "/items/:", None),
    ] {
        assert_eq!(
            projected_normalized(framework, path).as_deref(),
            expected,
            "{framework} {path}"
        );
    }
}
