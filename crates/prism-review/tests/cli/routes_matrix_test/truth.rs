//! Executed-truth comparison follows `tests/routes_matrix/truth/reconcile.py`.

mod links;
mod paths;
mod receipt;
#[cfg(test)]
mod tests;

use super::listing::items;
use links::{compare_mount_links, compare_protocol, runtime_only, runtime_only_count};
use paths::native_tokens;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const SITE_FIELDS: [&str; 2] = ["file", "line"];

struct Observation<'a> {
    projected: &'a Value,
    call: &'a Value,
}

pub(super) fn compare_row(row: &Value, entry: Option<&Value>, errors: &mut Vec<String>) {
    match (row["truth_source"].as_str(), entry) {
        (Some("hand"), None) => {}
        (Some("hand"), Some(truth)) if truth["status"] == "hand_required" => {}
        (Some("hand"), Some(truth)) => errors.push(format!(
            "truth.status: unexpected truth for hand row ({})",
            truth["status"]
        )),
        (Some("executed"), Some(truth)) if truth["status"] == "executed" => {
            compare_executed(row, truth, errors);
        }
        (Some("executed"), Some(truth)) => errors.push(format!(
            "truth.status: expected executed, got {}",
            truth["status"]
        )),
        (Some("executed"), None) => errors.push("truth: missing executed entry".to_owned()),
        _ => errors.push(format!(
            "truth_source: invalid source {}",
            row["truth_source"]
        )),
    }
}

pub(super) fn compare_receipt(row: &Value, entry: &Value, digest: &str, errors: &mut Vec<String>) {
    let before = errors.len();
    compare_row(row, Some(entry), errors);
    if errors.len() == before {
        if let Err(error) = receipt::validate(row, entry, digest) {
            errors.push(error);
        }
    }
}

fn compare_pin(
    expected: Option<&Value>,
    actual: Option<&Value>,
    field: &str,
    errors: &mut Vec<String>,
) {
    if expected != actual {
        let display =
            |value: Option<&Value>| value.map_or_else(|| "<absent>".to_owned(), Value::to_string);
        errors.push(format!(
            "{field}: expected {}, got {}",
            display(expected),
            display(actual)
        ));
    }
}

fn compare_fields(
    expected: &Value,
    actual: &Value,
    keys: &[&str],
    field: &str,
    errors: &mut Vec<String>,
) {
    for key in keys {
        if let Some(want) = expected.get(key) {
            compare_pin(
                Some(want),
                actual.get(key),
                &format!("{field}.{key}"),
                errors,
            );
        }
    }
}

fn method_pin(value: &Value) -> Value {
    if value == "any" || *value == json!({"status": "any"}) {
        json!({"any": true})
    } else {
        value.clone()
    }
}

fn site_key(site: &Value) -> (&Value, &Value) {
    (&site["file"], &site["line"])
}

fn pair_records(
    expected: &[Value],
    observed: &[Observation<'_>],
) -> (Vec<Option<usize>>, BTreeSet<usize>) {
    let mut pairs = vec![None; expected.len()];
    let mut remaining: BTreeSet<_> = (0..observed.len()).collect();

    // Give exact sites (including pinned ordinals) priority over diagnostic fallbacks.
    for exact in [true, false] {
        for (pin, pair) in expected.iter().zip(&mut pairs) {
            if pair.is_some() {
                continue;
            }
            let site = &pin["site"];
            let mut matches: Vec<_> = remaining
                .iter()
                .copied()
                .filter(|&index| {
                    let actual = &observed[index];
                    SITE_FIELDS.iter().all(|&key| {
                        site.get(key)
                            .is_none_or(|want| want == &actual.projected["site"][key])
                    }) && (!exact
                        || site
                            .get("ordinal")
                            .is_none_or(|want| want == &actual.call["site"]["ordinal"]))
                })
                .collect();
            if !exact && matches.len() > 1 {
                if let Some(methods) = pin.get("methods") {
                    let declared = method_pin(methods);
                    matches.retain(|&index| observed[index].projected["methods"] == declared);
                }
            }
            if let [index] = matches.as_slice() {
                *pair = Some(*index);
                remaining.remove(index);
            }
        }
    }

    // A single remaining pair can report mismatched site pins unambiguously.
    let mut unmatched = pairs.iter_mut().filter(|pair| pair.is_none());
    if let (Some(pair), None, 1) = (unmatched.next(), unmatched.next(), remaining.len()) {
        *pair = remaining.pop_first();
    }
    (pairs, remaining)
}

fn compare_paths(
    pins: &[Value],
    observed: &Observation<'_>,
    served: &[Value],
    field: &str,
    errors: &mut Vec<String>,
) {
    let mut paths = Vec::new();
    for route in served {
        let linked = route["call_ids"]
            .as_array()
            .is_some_and(|ids| ids.contains(&observed.projected["call_id"]));
        if let Some(path) = route["path"].as_str().filter(|_| linked) {
            paths.push(path);
        }
    }
    let observed_count = paths.len();
    let unattached = observed_count == 0;

    let mut ordered: Vec<_> = pins.iter().enumerate().collect();
    ordered.sort_by_key(|(_, pin)| {
        (
            pin["status"] != "full",
            std::cmp::Reverse(pin["value"].as_str().map_or(0, str::len)),
        )
    });
    for (index, pin) in ordered {
        let suffix = pin["status"] == "suffix";
        if pin["status"] == "dynamic"
            || (suffix
                && unattached
                && matches!(
                    pin["reason"].as_str(),
                    Some("no_known_mount" | "mount_cycle" | "expansion_truncated")
                ))
        {
            continue;
        }
        let Some(value) = pin["value"].as_str() else {
            continue;
        };
        let Some(index_in_served) = paths
            .iter()
            .position(|path| *path == value || (suffix && path.ends_with(value)))
        else {
            errors.push(format!(
                "{field}[{index}].value: expected {} {value}, got {paths:?}",
                pin["status"]
            ));
            continue;
        };
        paths.remove(index_in_served);
        if pin.get("normalized").is_some() || pin.get("path_tokens").is_some() {
            let framework = observed.projected["framework"].as_str().unwrap_or_default();
            let tokens = native_tokens(framework, value);
            let normalized = tokens.as_ref().map(|tokens| {
                tokens
                    .iter()
                    .map(|token| token[1].as_str().unwrap_or("{}"))
                    .collect::<String>()
            });
            compare_fields(
                pin,
                &json!({"path_tokens": tokens, "normalized": normalized}),
                &["normalized", "path_tokens"],
                &format!("{field}[{index}]"),
                errors,
            );
        }
    }
    let exact = !pins.is_empty()
        && pins
            .iter()
            .all(|pin| pin["status"] == "full" && pin["value"].is_string());
    if exact && observed_count - paths.len() == pins.len() && !paths.is_empty() {
        errors.push(format!("{field}: unexpected linked served paths {paths:?}"));
    }
}

fn compare_callbacks(
    pins: &[Value],
    observed: &Observation<'_>,
    field: &str,
    errors: &mut Vec<String>,
) {
    let Some(raw) = observed.call["callbacks"].as_array() else {
        errors.push(format!("{field}: missing raw callbacks array"));
        return;
    };
    compare_pin(
        Some(&json!(pins.len())),
        Some(&json!(raw.len())),
        &format!("{field}.length"),
        errors,
    );
    let projected: BTreeMap<_, _> = observed.projected["callbacks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|callback| Some((callback["position"].as_u64()?, callback)))
        .collect();

    for (index, pin) in pins.iter().enumerate() {
        let position = json!(index);
        for (key, raw_key) in [
            ("position", "position"),
            ("file", "file"),
            ("name", "name"),
            ("start_line", "first_line"),
        ] {
            if let Some(want) = pin.get(key) {
                let actual = raw.get(index).and_then(|callback| {
                    projected
                        .get(&(index as u64))
                        .and_then(|projected| projected.get(key))
                        .or_else(|| {
                            if key == "position" {
                                Some(&position)
                            } else {
                                callback.get(raw_key)
                            }
                        })
                });
                compare_pin(
                    Some(want),
                    actual,
                    &format!("{field}[{index}].{key}"),
                    errors,
                );
            }
        }
    }
}

fn compare_executed(row: &Value, truth: &Value, errors: &mut Vec<String>) {
    let Some(expected) = row["expect"].as_array() else {
        return;
    };
    let (Some(projection), Some(calls), Some(served)) = (
        truth["projection"].as_array(),
        truth["calls"].as_array(),
        truth["served"].as_array(),
    ) else {
        errors.push("truth: expected projection, calls, and served arrays".to_owned());
        return;
    };
    let mut call_ids = BTreeSet::new();
    for call in calls {
        if !call["id"].as_u64().is_some_and(|id| call_ids.insert(id)) {
            errors.push(format!(
                "truth.calls: invalid or duplicate ID {}",
                call["id"]
            ));
            return;
        }
    }
    for (index, route) in served.iter().enumerate() {
        let mut seen = BTreeSet::new();
        if !route["call_ids"].as_array().is_some_and(|ids| {
            !ids.is_empty()
                && ids.iter().all(|id| {
                    id.as_u64()
                        .is_some_and(|id| call_ids.contains(&id) && seen.insert(id))
                })
        }) {
            errors.push(format!(
                "truth.served[{index}].call_ids: expected distinct existing call IDs"
            ));
            return;
        }
    }
    let mut observed = Vec::new();
    for (index, projected) in projection.iter().enumerate() {
        let call = projected
            .get("call_id")
            .and_then(|id| calls.iter().rev().find(|call| call.get("id") == Some(id)));
        if let Some(call) = call {
            observed.push(Observation { projected, call });
        } else {
            errors.push(format!(
                "truth.projection[{index}].call_id: no call for {}",
                projected["call_id"]
            ));
        }
    }
    if observed.len() != projection.len() {
        return;
    }

    if let Some(count) = runtime_only_count(row) {
        if calls.len() != count
            || projection.len() != count
            || !calls.iter().all(|call| {
                runtime_only(row, call)
                    && projection
                        .iter()
                        .filter(|record| record["call_id"] == call["id"])
                        .count()
                        == 1
                    && served
                        .iter()
                        .any(|route| items(&route["call_ids"]).contains(&call["id"]))
            })
        {
            errors
                .push("truth.runtime_only: expected the complete approved observation".to_owned());
            return;
        }
    }

    let (pairs, remaining) = pair_records(expected, &observed);
    let registrations = expected
        .iter()
        .zip(&pairs)
        .filter_map(|(pin, pair)| Some((observed[(*pair)?].projected["call_id"].as_u64()?, pin)))
        .collect();
    compare_mount_links(row, truth, calls, served, &registrations, errors);
    for (index, (pin, pair)) in expected.iter().zip(pairs).enumerate() {
        let field = format!("expect[{index}]");
        let Some(pair) = pair else {
            if pin["receiver"]["status"] != "unproven"
                || pin["paths"]
                    .as_array()
                    .is_some_and(|paths| !paths.is_empty())
            {
                errors.push(format!(
                    "{field}.registrations: no observed registration for {}",
                    pin["site"]
                ));
            }
            continue;
        };
        let observation = &observed[pair];
        let actual = observation.projected;
        compare_fields(pin, actual, &["framework"], &field, errors);
        compare_protocol(pin, observation, served, &field, errors);
        if let Some(site) = pin.get("site") {
            let field = format!("{field}.site");
            compare_fields(site, &actual["site"], &SITE_FIELDS, &field, errors);
            let multiple_observed = projection
                .iter()
                .filter(|record| site_key(&record["site"]) == site_key(&actual["site"]))
                .nth(1)
                .is_some();
            let multiple_expected = expected
                .iter()
                .filter(|record| site_key(&record["site"]) == site_key(site))
                .nth(1)
                .is_some();
            if site.get("ordinal").is_some() || multiple_observed || multiple_expected {
                compare_pin(
                    site.get("ordinal"),
                    Some(&observation.call["site"]["ordinal"]),
                    &format!("{field}.ordinal"),
                    errors,
                );
            }
        }
        if let Some(methods) = pin.get("methods") {
            let declared = method_pin(methods);
            let runtime = &actual["methods"];
            let implicit_any = runtime.is_null() && declared == json!({"any": true});
            if declared["status"] != "unknown" && !implicit_any {
                compare_pin(
                    Some(&declared),
                    Some(runtime),
                    &format!("{field}.methods"),
                    errors,
                );
            }
        }
        if let Some(path) = pin.get("path") {
            let written = observation.call.get("path").unwrap_or(&actual["path"]);
            let raw_only = path["literal"].is_string();
            let keys: &[_] = if raw_only && written["literal"].is_null() {
                &["raw"]
            } else {
                &["raw", "literal"]
            };
            compare_fields(path, written, keys, &format!("{field}.path"), errors);
            if raw_only {
                let key = if actual["framework"] == "flask" {
                    "rule"
                } else {
                    "path"
                };
                compare_pin(
                    path.get("literal"),
                    observation.call["args"].get(key),
                    &format!("{field}.path.evaluated"),
                    errors,
                );
            }
        }
        if let Some(paths) = pin["paths"].as_array() {
            compare_paths(
                paths,
                observation,
                served,
                &format!("{field}.paths"),
                errors,
            );
        }
        if let Some(callbacks) = pin["callbacks"].as_array() {
            compare_callbacks(
                callbacks,
                observation,
                &format!("{field}.callbacks"),
                errors,
            );
        }
    }
    for index in remaining {
        if runtime_only(row, observed[index].call) {
            continue;
        }
        errors.push(format!(
            "truth.projection[{index}].registrations: unexpected registration at {}",
            observed[index].call["site"]
        ));
    }
}

#[cfg(test)]
mod mutations;
