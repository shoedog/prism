//! Join served observations and approved losses to their exact raw calls.
use super::{compare_pin, site_key, Observation};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(super) fn runtime_only(row: &Value, call: &Value) -> bool {
    // Known calls at the Stage D diagnostic anchors. Express reports a
    // one-based stack column. Python reports a zero-based UTF-8 byte span.
    let (Some(id), Some(kind)) = (call["id"].as_u64(), call["kind"].as_str()) else {
        return false;
    };
    let (reason, site) = match (row["id"].as_str(), kind, id) {
        (Some("et1-10"), "verb", 1 | 2) => (
            "runtime_generated_registration",
            json!({"file":"app.js","line":5,"column":14,"ordinal":id}),
        ),
        (Some("es1-20"), "url_rule", 1 | 2) => (
            "runtime_generated_registration",
            json!({"file":"app.py","line":9,"column_byte":4,"end_line":9,"end_column_byte":61,"ordinal":id}),
        ),
        (Some("es3-07"), "url_rule", 1) => (
            "unsupported_registration",
            json!({"file":"app.py","line":11,"column_byte":0,"end_line":11,"end_column_byte":16,"ordinal":id}),
        ),
        _ => return false,
    };
    call["site"] == site
        && row["expect_diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| {
                diagnostics.iter().any(|item| {
                    item["reason"] == reason && site_key(&item["site"]) == site_key(&site)
                })
            })
}

fn matches_mount(pin: &Value, call: &Value) -> bool {
    let site = &pin["site"];
    if site_key(site) != site_key(&call["site"])
        || site
            .get("ordinal")
            .is_some_and(|ordinal| ordinal != &call["site"]["ordinal"])
    {
        return false;
    }
    let prefix = &pin["prefix"];
    match prefix["status"].as_str() {
        Some("literal") => prefix["value"] == call["mount_prefix"],
        Some("absent") => call["args"]["has_path_arg"] == false && call["mount_prefix"] == "",
        Some("unevaluated") => prefix
            .get("text")
            .is_some_and(|raw| call["path"].get("raw") == Some(raw)),
        _ => false,
    }
}

fn matches_leaf(leaf: &Value, suffix: &str) -> bool {
    match leaf {
        Value::String(leaf) => leaf == suffix,
        Value::Array(leaves) => leaves.iter().any(|leaf| matches_leaf(leaf, suffix)),
        _ => false,
    }
}

pub(super) fn compare_mount_links(
    row: &Value,
    truth: &Value,
    calls: &[Value],
    served: &[Value],
    errors: &mut Vec<String>,
) {
    let calls: BTreeMap<_, _> = calls
        .iter()
        .filter_map(|call| Some((call["id"].as_u64()?, call)))
        .collect();
    let pins = row["expect_mounts"].as_array().into_iter().flatten();
    for (index, route) in served.iter().enumerate() {
        if route.get("mount_call_ids").is_none() && truth["framework"] != "express" {
            continue;
        }
        let prefix = route["mount_call_ids"].as_array().and_then(|ids| {
            let prefixes: Option<Vec<_>> = ids
                .iter()
                .map(|id| {
                    let call = calls.get(&id.as_u64()?)?;
                    (call["kind"] == "use"
                        && call["mount"] == true
                        && pins.clone().any(|pin| matches_mount(pin, call)))
                    .then(|| call["mount_prefix"].as_str())
                    .flatten()
                })
                .collect();
            Some(prefixes?.concat())
        });
        let matches = prefix.is_some_and(|prefix| {
            if let Some(path) = route["path"].as_str() {
                path.strip_prefix(&prefix).is_some_and(|suffix| {
                    route["call_ids"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|id| calls.get(&id.as_u64()?))
                        .any(|call| matches_leaf(&call["args"]["path"], suffix))
                })
            } else {
                route["mount_prefix"] == prefix
            }
        });
        if !matches {
            errors.push(format!(
                "truth.served[{index}].mount_call_ids: links {} do not join pinned use sites to {}",
                route["mount_call_ids"], route["path"],
            ));
        }
    }
}

pub(super) fn compare_protocol(
    pin: &Value,
    observed: &Observation<'_>,
    served: &[Value],
    field: &str,
    errors: &mut Vec<String>,
) {
    let Some(protocol) = pin.get("protocol") else {
        return;
    };
    let kind = observed.call.get("kind");
    let http = json!("http");
    let raw = match kind.and_then(Value::as_str) {
        Some("api_route" | "url_rule" | "verb") => Some(&http),
        _ => kind,
    };
    compare_pin(Some(protocol), raw, &format!("{field}.protocol"), errors);
    for (index, route) in served.iter().enumerate() {
        if route["call_ids"]
            .as_array()
            .is_some_and(|ids| ids.contains(&observed.projected["call_id"]))
        {
            compare_pin(
                Some(protocol),
                route.get("kind"),
                &format!("{field}.served[{index}].protocol"),
                errors,
            );
        }
    }
}
