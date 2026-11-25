//! Executed-truth comparison follows `tests/routes_matrix/truth/reconcile.py`.

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

fn identifier(text: &str) -> bool {
    let mut chars = text.bytes();
    chars
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && chars.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn native_tokens(framework: &str, path: &str) -> Option<Vec<Value>> {
    let mut tokens = Vec::new();
    let mut literal_start = 0;
    let (open, close) = match framework {
        "flask" => ('<', '>'),
        "fastapi" => ('{', '}'),
        "express" if !path.contains(['*', '?', '+', '(', ')', '[', ']', '\\']) => (':', '/'),
        _ => return None,
    };
    for (start, _) in path.match_indices(open) {
        if start < literal_start {
            continue;
        }
        let after = &path[start + 1..];
        let end = if framework == "express" {
            let end = after
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            if end == 0 {
                continue;
            }
            end + start + 1
        } else {
            let Some(length) = after.find(close) else {
                continue;
            };
            let parameter = &after[..length];
            let (name, converter) =
                parameter
                    .split_once(':')
                    .map_or((parameter, None), |(left, right)| {
                        if framework == "flask" {
                            (right, Some(left))
                        } else {
                            (left, Some(right))
                        }
                    });
            if !identifier(name) {
                continue;
            }
            if let Some(converter) = converter {
                let kind = converter.split('(').next().unwrap();
                if !matches!(kind, "string" | "str" | "int" | "float" | "uuid" | "any") {
                    return None;
                }
            }
            length + start + 2
        };
        if start > literal_start {
            tokens.push(json!(["literal", &path[literal_start..start]]));
        }
        tokens.push(json!(["parameter"]));
        literal_start = end;
    }
    if literal_start < path.len() {
        tokens.push(json!(["literal", &path[literal_start..]]));
    }
    Some(tokens)
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

    for (index, pin) in pins.iter().enumerate() {
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

fn runtime_only(row: &Value, call: &Value) -> bool {
    let reason = match row["id"].as_str() {
        Some("et1-10" | "es1-20") => "runtime_generated_registration",
        Some("es3-07") => "unsupported_registration",
        _ => return false,
    };
    row["expect_diagnostics"]
        .as_array()
        .is_some_and(|diagnostics| {
            diagnostics.iter().any(|item| {
                item["reason"] == reason && site_key(&item["site"]) == site_key(&call["site"])
            })
        })
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

    let (pairs, remaining) = pair_records(expected, &observed);
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
            let mut written = observation
                .call
                .get("path")
                .unwrap_or(&actual["path"])
                .clone();
            if written["literal"].is_null() && path["literal"].is_string() {
                let key = if actual["framework"] == "flask" {
                    "rule"
                } else {
                    "path"
                };
                written["literal"] = observation.call["args"][key].clone();
            }
            compare_fields(
                path,
                &written,
                &["raw", "literal"],
                &format!("{field}.path"),
                errors,
            );
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

fn example() -> (Value, Value) {
    let site = json!({"file": "app.js", "line": 7});
    let path = json!({"raw": "'/:id'", "literal": "/:id"});
    let exposure = json!({"status": "full", "value": "/api/:id", "normalized": "/api/{}"});
    let callback = json!({"position": 0, "file": "app.js", "name": "handler", "start_line": 3});
    let mut projected = json!({
        "framework": "express", "site": site, "path": path, "paths": [exposure],
        "methods": {"known": ["GET"]}, "callbacks": [callback]
    });
    let row = json!({"truth_source": "executed", "expect": [projected]});
    projected["call_id"] = json!(1);
    let truth = json!({
        "status": "executed", "projection": [projected],
        "calls": [{"id": 1, "site": {"file": "app.js", "line": 7, "ordinal": 2},
            "callbacks": [{"file": "app.js", "name": "handler", "first_line": 3}]}],
        "served": [{"call_ids": [1], "path": "/api/:id", "effective_methods": ["GET", "HEAD"]}]
    });
    (row, truth)
}

fn assert_fields(row: &Value, truth: Option<&Value>, expected: &[&str]) {
    let mut errors = Vec::new();
    compare_row(row, truth, &mut errors);
    let fields: Vec<_> = errors
        .iter()
        .map(|error| error.split_once(':').unwrap().0)
        .collect();
    assert_eq!(fields, expected, "{errors:?}");
}

#[test]
fn rejects_changed_callback_full_path_declared_method_and_site_line() {
    let (row, truth) = example();
    assert_fields(&row, Some(&truth), &[]);
    let mut changed = row.clone();
    changed["expect"][0]["callbacks"][0]["name"] = json!("other_handler");
    assert_fields(&changed, Some(&truth), &["expect[0].callbacks[0].name"]);
    let mut changed = row.clone();
    changed["expect"][0]["paths"][0]["value"] = json!("/:id");
    assert_fields(&changed, Some(&truth), &["expect[0].paths[0].value"]);
    let mut changed = row.clone();
    changed["expect"][0]["methods"] = json!({"known": ["GET", "HEAD"]});
    assert_fields(&changed, Some(&truth), &["expect[0].methods"]);
    let mut changed = row;
    changed["expect"][0]["site"]["line"] = json!(8);
    assert_fields(&changed, Some(&truth), &["expect[0].site.line"]);
}

#[test]
fn pairs_reordered_same_line_calls_by_raw_ordinal() {
    let (mut row, mut truth) = example();
    let mut second = truth["projection"][0].clone();
    second["call_id"] = json!(2);
    second["methods"] = json!({"known": ["POST"]});
    truth["projection"].as_array_mut().unwrap().push(second);
    let mut call = truth["calls"][0].clone();
    call["id"] = json!(2);
    call["site"]["ordinal"] = json!(3);
    truth["calls"].as_array_mut().unwrap().push(call);
    truth["served"][0]["call_ids"] = json!([1, 2]);
    let mut second_pin = row["expect"][0].clone();
    second_pin["methods"] = json!({"known": ["POST"]});
    row["expect"].as_array_mut().unwrap().insert(0, second_pin);

    assert_fields(
        &row,
        Some(&truth),
        &["expect[0].site.ordinal", "expect[1].site.ordinal"],
    );
    row["expect"][0]["site"]["ordinal"] = json!(4);
    row["expect"][1]["site"]["ordinal"] = json!(2);
    assert_fields(&row, Some(&truth), &["expect[0].site.ordinal"]);
    row["expect"][0]["site"]["ordinal"] = json!(3);
    assert_fields(&row, Some(&truth), &[]);
    row["expect"][0]["methods"] = json!({"known": ["GET"]});
    row["expect"][1]["methods"] = json!({"known": ["POST"]});
    assert_fields(
        &row,
        Some(&truth),
        &["expect[0].methods", "expect[1].methods"],
    );
}

#[test]
fn checks_linked_served_paths_and_normalized_suffixes() {
    let (mut row, mut truth) = example();
    row["expect"][0]["paths"] = json!([{"status": "suffix", "value": "/:id", "normalized": "/{}"}]);
    truth["served"]
        .as_array_mut()
        .unwrap()
        .push(json!({"call_ids": [1], "path": "/also"}));
    assert_fields(&row, Some(&truth), &[]);
    row["expect"][0]["paths"][0]["normalized"] = json!("/wrong");
    assert_fields(&row, Some(&truth), &["expect[0].paths[0].normalized"]);
    truth["served"][0]["path"] = json!("/api/:id/");
    assert_fields(&row, Some(&truth), &["expect[0].paths[0].value"]);
    row["expect"][0]["paths"] = json!([{"status": "full", "value": "/also"}]);
    assert_fields(&row, Some(&truth), &["expect[0].paths"]);
    row["expect"][0]["paths"] = json!([
        {"status": "full", "value": "/also"},
        {"status": "full", "value": "/api/:id/"}
    ]);
    assert_fields(&row, Some(&truth), &[]);
}

#[test]
fn skips_only_static_paths_and_static_metadata() {
    let (mut row, mut truth) = example();
    let pin = &mut row["expect"][0];
    pin["receiver"] = json!({"status": "unproven"});
    pin["site"]["text"] = json!("different source text");
    pin["site"]["occurrence"] = json!(5);
    pin["path"]["text"] = json!("static metadata");
    pin["callbacks"][0]["status"] = json!("unresolved");
    pin["callbacks"][0]["end_line"] = json!(100);
    pin["callbacks"][0]["delegates"] = json!(["somewhere_else"]);
    pin["paths"] = json!([{"status": "dynamic", "value": "/wrong"}]);
    row["expect_mounts"] = json!([{"site": {"line": 23}}]);
    row["expect_unlisted"] = json!(true);
    assert_fields(&row, Some(&truth), &[]);

    truth["served"][0]["call_ids"] = json!([99]);
    for reason in ["no_known_mount", "mount_cycle", "expansion_truncated"] {
        row["expect"][0]["paths"][0] =
            json!({"status": "suffix", "value": "/wrong", "reason": reason});
        assert_fields(&row, Some(&truth), &[]);
    }
    row["expect"][0]["paths"][0]["reason"] = json!("variable_prefix");
    assert_fields(&row, Some(&truth), &["expect[0].paths[0].value"]);
    row["expect"][0]["paths"][0]["reason"] = json!("no_known_mount");
    truth["served"][0]["call_ids"] = json!([1]);
    assert_fields(&row, Some(&truth), &["expect[0].paths[0].value"]);
}

#[test]
fn checks_raw_callback_count_and_falls_back_to_raw_identity() {
    let (mut row, mut truth) = example();
    truth["calls"][0]["callbacks"][0]["name"] = json!("raw_name");
    assert_fields(&row, Some(&truth), &[]);
    truth["projection"][0]
        .as_object_mut()
        .unwrap()
        .remove("callbacks");
    assert_fields(&row, Some(&truth), &["expect[0].callbacks[0].name"]);
    row["expect"][0]["callbacks"][0]["name"] = json!("raw_name");
    assert_fields(&row, Some(&truth), &[]);
    row["expect"][0]["callbacks"] = json!([]);
    assert_fields(&row, Some(&truth), &["expect[0].callbacks.length"]);
}

#[test]
fn accepts_only_compatible_declared_method_pins() {
    let (mut row, mut truth) = example();
    truth["projection"][0]
        .as_object_mut()
        .unwrap()
        .remove("methods");
    assert_fields(&row, Some(&truth), &["expect[0].methods"]);
    for pin in [json!("any"), json!({"any": true}), json!({"status": "any"})] {
        row["expect"][0]["methods"] = pin;
        truth["projection"][0]["methods"] = Value::Null;
        assert_fields(&row, Some(&truth), &[]);
        truth["projection"][0]["methods"] = json!({"any": true});
        assert_fields(&row, Some(&truth), &[]);
        truth["projection"][0]["methods"] = json!({"known": ["GET"]});
        assert_fields(&row, Some(&truth), &["expect[0].methods"]);
    }
    row["expect"][0]["methods"] = json!({"status": "unknown", "text": "M"});
    assert_fields(&row, Some(&truth), &[]);
    row["expect"][0].as_object_mut().unwrap().remove("methods");
    assert_fields(&row, Some(&truth), &[]);
}

#[test]
fn checks_missing_and_extra_registrations_with_static_exception() {
    let (mut row, mut truth) = example();
    let pin = row["expect"][0].clone();
    row["expect"] = json!([]);
    assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
    row["expect"] = json!([pin]);
    truth["projection"] = json!([]);
    assert_fields(&row, Some(&truth), &["expect[0].registrations"]);
    row["expect"][0]["receiver"] = json!({"status": "unproven"});
    assert_fields(&row, Some(&truth), &["expect[0].registrations"]);
    row["expect"][0]["paths"] = json!([]);
    assert_fields(&row, Some(&truth), &[]);
}

#[test]
fn runtime_only_registrations_require_approved_diagnostic_sites() {
    let (mut row, mut truth) = example();
    row["expect"] = json!([]);
    assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
    row["accepted_loss"] = json!("runtime_generated_registration");
    assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
    row["id"] = json!("et1-10");
    row["expect_diagnostics"] =
        json!([{"site": {"file":"app.js","line":7},"reason":"runtime_generated_registration"}]);
    assert_fields(&row, Some(&truth), &[]);
    truth["calls"][0]["site"]["line"] = json!(8);
    assert_fields(&row, Some(&truth), &["truth.projection[0].registrations"]);
}

#[test]
fn checks_truth_availability_by_row_source() {
    let (mut row, truth) = example();
    let refused = json!({"status": "hand_required"});
    assert_fields(&row, None, &["truth"]);
    assert_fields(&row, Some(&refused), &["truth.status"]);
    row["truth_source"] = json!("hand");
    assert_fields(&row, None, &[]);
    assert_fields(&row, Some(&refused), &[]);
    assert_fields(&row, Some(&truth), &["truth.status"]);
}
