//! Join served observations and approved losses to their exact raw calls.
use super::super::listing::items;
use super::{compare_pin, site_key, Observation};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

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

fn same_router(left: &Value, right: &Value) -> bool {
    // Source pins use construction text, while a wire root can pin the byte.
    ["file", "binding"]
        .iter()
        .all(|&key| !left[key].is_null() && left[key] == right[key])
        && items(&left["invocations"]) == items(&right["invocations"])
        && ["construction_byte", "construction_text"]
            .iter()
            .all(|&key| {
                left.get(key)
                    .zip(right.get(key))
                    .is_none_or(|(a, b)| a == b)
            })
        && (left.get("construction_text").is_none()
            || right.get("construction_text").is_none()
            || left["occurrence"].as_u64().unwrap_or(1)
                == right["occurrence"].as_u64().unwrap_or(1))
}

fn matches_mount_chain(
    mounts: &[Value],
    chain: &[&Value],
    receiver: &Value,
    root: &Value,
    unproven_child: bool,
) -> bool {
    // Walk backwards from the receiver using exactly one pinned edge per call.
    let Some((call, previous)) = chain.split_last() else {
        return if root.is_null() {
            !mounts
                .iter()
                .any(|pin| same_router(receiver, &pin["child"]))
        } else {
            same_router(receiver, root)
        };
    };
    mounts.iter().any(|pin| {
        (same_router(receiver, &pin["child"])
            || (unproven_child && pin["child"]["status"] == "unproven"))
            && matches_mount(pin, call)
            && matches_mount_chain(mounts, previous, &pin["parent"], root, false)
    })
}

fn matches_exposure_chain(mounts: &[Value], chain: &[&Value], pin: &Value, path: &Value) -> bool {
    let mut exposures = items(&pin["paths"])
        .iter()
        .filter(|exposure| match exposure["status"].as_str() {
            Some("dynamic") => true,
            Some("full") => path.is_string() && *path == exposure["value"],
            Some("suffix") => path
                .as_str()
                .zip(exposure["value"].as_str())
                .is_some_and(|(path, suffix)| path.ends_with(suffix)),
            _ => false,
        })
        .peekable();
    if exposures.peek().is_none() {
        return matches_mount_chain(mounts, chain, &pin["receiver"], &Value::Null, false);
    }
    // A no_known_mount suffix may have an unproven final child in the pins.
    exposures.any(|exposure| {
        matches_mount_chain(
            mounts,
            chain,
            &pin["receiver"],
            &exposure["root"],
            exposure["reason"] == "no_known_mount",
        )
    })
}

pub(super) fn compare_mount_links(
    row: &Value,
    truth: &Value,
    calls: &[Value],
    served: &[Value],
    registrations: &BTreeMap<u64, &Value>,
    errors: &mut Vec<String>,
) {
    let calls: BTreeMap<_, _> = calls
        .iter()
        .filter_map(|call| Some((call["id"].as_u64()?, call)))
        .collect();
    let mounts = items(&row["expect_mounts"]);
    for (index, route) in served.iter().enumerate() {
        if route.get("mount_call_ids").is_none() && truth["framework"] != "express" {
            continue;
        }
        let prefix = route["mount_call_ids"].as_array().and_then(|ids| {
            let mut seen = BTreeSet::new();
            let chain: Vec<_> = ids
                .iter()
                .map(|id| {
                    let id = id.as_u64()?;
                    let call = *calls.get(&id)?;
                    (seen.insert(id) && call["kind"] == "use" && call["mount"] == true)
                        .then_some(call)
                })
                .collect::<Option<_>>()?;
            let mut pins = items(&route["call_ids"])
                .iter()
                .filter_map(|id| registrations.get(&id.as_u64()?))
                .peekable();
            if (pins.peek().is_none() && !(chain.is_empty() && mounts.is_empty()))
                || !pins.all(|pin| matches_exposure_chain(mounts, &chain, pin, &route["path"]))
            {
                return None;
            }
            chain
                .iter()
                .map(|call| call["mount_prefix"].as_str())
                .collect::<Option<String>>()
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
                "truth.served[{index}].mount_call_ids: links {} do not join pinned mount endpoints to {}",
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
