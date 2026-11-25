use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod truth;

const ENABLED_GROUPS: &[&str] = &[];
const ROUTE_FIELDS: &[&str] = &[
    "framework",
    "methods",
    "path",
    "paths",
    "callbacks",
    "receiver",
    "site",
];
const CALLBACK_STATUSES: &[&str] = &["resolved", "anonymous", "unresolved"];
const PATH_STATUSES: &[&str] = &["full", "suffix", "dynamic"];
const REASONS: &[&str] = &[
    "class_view",
    "expansion_truncated",
    "func_field_value",
    "hapi_handler_conflict",
    "hapi_handler_object",
    "hapi_plugin_unproven",
    "hapi_plugin_unregistered",
    "hapi_registration_unproven",
    "hapi_route_not_literal",
    "hapi_server_param_unproven",
    "imported_unproven",
    "inner_decorator",
    "member_handler",
    "method_expression",
    "method_value_external",
    "method_value_interface",
    "method_value_method_unproven",
    "method_value_receiver_shadowed",
    "method_value_receiver_unproven",
    "mount_child_unproven",
    "mount_cycle",
    "no_known_mount",
    "non_literal_path",
    "qualified_function_value",
    "receiver_unproven",
    "table_consumer_unproven",
    "variable_prefix",
    "wrapper_call",
];

fn fixture_root() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    checked_path(&workspace, &json!("tests/routes_matrix"), true)
        .unwrap_or_else(|err| panic!("fixture root: {err}"))
}

fn read_rows(path: &Path, errors: &mut Vec<String>) -> BTreeMap<String, Value> {
    let mut rows = BTreeMap::new();
    let contents =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    for (index, line) in contents.lines().enumerate() {
        let location = format!("{}:{}", path.display(), index + 1);
        let row: Value = match serde_json::from_str(line) {
            Ok(row) => row,
            Err(err) => {
                errors.push(format!("{location}: {err}"));
                continue;
            }
        };
        let Some(id) = nonempty_str(&row["id"]) else {
            errors.push(format!("{location}.id: expected a nonempty string"));
            continue;
        };
        let id = id.to_owned();
        if rows.insert(id.clone(), row).is_some() {
            errors.push(format!("{location}.id: duplicate ID {id}"));
        }
    }
    rows
}

fn nonempty_str(value: &Value) -> Option<&str> {
    value.as_str().filter(|text| !text.trim().is_empty())
}

fn valid_group(group: &str) -> bool {
    matches!(
        group,
        "core" | "w1-python" | "w1-express" | "w2-gin" | "w2-method-value" | "w2-nethttp"
    ) || group.strip_prefix("w3-").is_some_and(|name| {
        name.split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_lowercase()))
    })
}

fn checked_path(root: &Path, value: &Value, directory: bool) -> Result<PathBuf, String> {
    let relative = nonempty_str(value).ok_or("expected a relative path")?;
    let mut path = root.to_owned();
    let mut parts = relative.split('/').peekable();
    while let Some(part) = parts.next() {
        if matches!(part, "" | "." | "..") || part.contains(['\\', ':']) {
            return Err(format!("invalid relative path {relative:?}"));
        }
        path.push(part);
        let info =
            fs::symlink_metadata(&path).map_err(|err| format!("{}: {err}", path.display()))?;
        let want_dir = directory || parts.peek().is_some();
        if (want_dir && !info.is_dir()) || (!want_dir && !info.is_file()) {
            return Err(format!("{relative:?} must use real directories and files"));
        }
    }
    Ok(path)
}

fn site_range(site: &Value, source: &str) -> Result<(usize, usize), String> {
    let text = nonempty_str(&site["text"]).ok_or("text must be a nonempty string")?;
    let occurrence = match site.get("occurrence") {
        Some(value) => value
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|&n| n > 0)
            .ok_or("occurrence must be a positive integer")?,
        None => 1,
    };
    let (start, _) = source
        .match_indices(text)
        .nth(occurrence - 1)
        .ok_or_else(|| format!("text {text:?} has fewer than {occurrence} occurrences"))?;
    Ok((start, start + text.len()))
}

fn validate_site(site: &Value, dir: Option<&Path>, field: &str, errors: &mut Vec<String>) {
    let Some(site_fields) = site.as_object() else {
        errors.push(format!("{field}: expected an object"));
        return;
    };
    for key in site_fields.keys() {
        if !matches!(
            key.as_str(),
            "file" | "line" | "text" | "occurrence" | "ordinal"
        ) {
            errors.push(format!("{field}.{key}: unknown key"));
        }
    }
    if !site["line"].as_u64().is_some_and(|line| line > 0) {
        errors.push(format!("{field}.line: expected a positive integer"));
    }
    if site
        .get("ordinal")
        .is_some_and(|value| !value.as_u64().is_some_and(|ordinal| ordinal > 0))
    {
        errors.push(format!("{field}.ordinal: expected a positive integer"));
    }
    let valid_file = nonempty_str(&site["file"]).is_some();
    if !valid_file {
        errors.push(format!("{field}.file: expected a nonempty filename"));
    }
    let valid_occurrence = match site.get("occurrence") {
        Some(value) => {
            value.as_u64().is_some_and(|number| number > 0) && site.get("text").is_some()
        }
        None => true,
    };
    if !valid_occurrence {
        errors.push(format!(
            "{field}.occurrence: expected a positive integer with text"
        ));
    }
    let has_text = site.get("text").is_some();
    let valid_text = nonempty_str(&site["text"]).is_some();
    if has_text && !valid_text {
        errors.push(format!("{field}.text: expected a nonempty string"));
    }
    if let Some(dir) = dir.filter(|_| valid_file) {
        match checked_path(dir, &site["file"], false) {
            Err(err) => errors.push(format!("{field}.file: {err}")),
            Ok(file) if valid_text && valid_occurrence => match fs::read_to_string(&file) {
                Ok(source) => {
                    if let Err(err) = site_range(site, &source) {
                        errors.push(format!("{field}.text: {err}"));
                    }
                }
                Err(err) => errors.push(format!("{field}.file: {}: {err}", file.display())),
            },
            Ok(_) => {}
        }
    }
}

fn validate_reasons(value: &Value, field: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let field = format!("{field}.{key}");
                if key == "reason"
                    && !child
                        .as_str()
                        .is_some_and(|reason| REASONS.contains(&reason))
                {
                    errors.push(format!("{field}: unknown reason {child}"));
                }
                validate_reasons(child, &field, errors);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                validate_reasons(child, &format!("{field}[{i}]"), errors);
            }
        }
        _ => {}
    }
}

fn validate_records(row: &Value, dir: Option<&Path>, errors: &mut Vec<String>) {
    for key in ["expect", "expect_mounts"] {
        let Some(records) = row[key].as_array() else {
            errors.push(format!("{key}: expected an array"));
            continue;
        };
        for (i, record) in records.iter().enumerate() {
            let field = format!("{key}[{i}]");
            let Some(object) = record.as_object() else {
                errors.push(format!("{field}: expected an object"));
                continue;
            };
            if let Some(site) = object.get("site") {
                validate_site(site, dir, &format!("{field}.site"), errors);
            }
            validate_reasons(record, &field, errors);
            if key != "expect" {
                continue;
            }
            for key in object
                .keys()
                .filter(|key| !ROUTE_FIELDS.contains(&key.as_str()))
            {
                errors.push(format!("{field}.{key}: unknown route key"));
            }
            for (key, statuses) in [("paths", PATH_STATUSES), ("callbacks", CALLBACK_STATUSES)] {
                let Some(values) = object.get(key) else {
                    continue;
                };
                let Some(items) = values.as_array() else {
                    errors.push(format!("{field}.{key}: expected an array"));
                    continue;
                };
                for (i, item) in items.iter().enumerate() {
                    let field = format!("{field}.{key}[{i}]");
                    if !item["status"]
                        .as_str()
                        .is_some_and(|status| statuses.contains(&status))
                    {
                        errors.push(format!("{field}.status: invalid status {}", item["status"]));
                    }
                    if key == "callbacks" && item["status"] == "unresolved" {
                        if nonempty_str(&item["text"]).is_none() {
                            errors.push(format!("{field}.text: unresolved callback needs text"));
                        }
                        if item.get("reason").is_none() {
                            errors.push(format!(
                                "{field}.reason: unresolved callback needs a reason"
                            ));
                        }
                    }
                }
            }
        }
    }
}

// Objects pin only their named fields. Arrays pin their exact length and order.
fn compare_pins(expected: &Value, actual: Option<&Value>, field: &str, errors: &mut Vec<String>) {
    let Some(actual) = actual else {
        errors.push(format!("{field}: missing, expected {expected}"));
        return;
    };
    match (expected, actual) {
        (Value::Object(want), Value::Object(got)) => {
            for (key, value) in want {
                compare_pins(value, got.get(key), &format!("{field}.{key}"), errors);
            }
        }
        (Value::Array(want), Value::Array(got)) => {
            if want.len() != got.len() {
                errors.push(format!(
                    "{field}: expected {} items, got {}",
                    want.len(),
                    got.len()
                ));
            }
            for (i, (want, got)) in want.iter().zip(got).enumerate() {
                compare_pins(want, Some(got), &format!("{field}[{i}]"), errors);
            }
        }
        _ if expected != actual => {
            errors.push(format!("{field}: expected {expected}, got {actual}"))
        }
        _ => {}
    }
}

/// SHA-256 over a row directory, as `truth/run_truth.py` computes it: each relative path, NUL, bytes, NUL,
/// in path order. A fixture edited without regenerating truth then fails the well-formedness test.
fn tree_sha256(dir: &Path) -> String {
    use sha2::{Digest, Sha256};
    fn files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    files(dir, &mut paths);
    paths.sort_by_key(|path| {
        path.strip_prefix(dir)
            .unwrap()
            .to_string_lossy()
            .into_owned()
    });
    let mut digest = Sha256::new();
    for path in paths {
        digest.update(path.strip_prefix(dir).unwrap().to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(std::fs::read(&path).unwrap());
        digest.update([0]);
    }
    format!("{:x}", digest.finalize())
}

#[test]
fn routes_matrix_rows_are_well_formed() {
    let root = fixture_root();
    let mut errors = Vec::new();
    let rows = read_rows(&root.join("rows.jsonl"), &mut errors);
    let truth = read_rows(&root.join("truth/truth.jsonl"), &mut errors);
    for (id, row) in &rows {
        let mut violations = Vec::new();
        let dir = checked_path(&root, &row["dir"], true)
            .inspect_err(|err| {
                violations.push(format!("dir: {err}"));
            })
            .ok();
        if !row["group"].as_str().is_some_and(valid_group) {
            violations.push(format!("group: invalid group {}", row["group"]));
        }
        validate_records(row, dir.as_deref(), &mut violations);
        if row["truth_source"] == "executed" && nonempty_str(&row["entry"]).is_none() {
            violations.push("entry: executed row needs a nonempty entry".to_owned());
        }
        if let (Some(dir), Some(entry)) = (dir.as_deref(), truth.get(id)) {
            if row["truth_source"] == "executed" && entry["status"] == "executed" {
                let want = tree_sha256(dir);
                if entry["fixture_sha256"] != want {
                    violations.push(format!(
                        "truth.fixture_sha256: stale truth, the fixture hashes to {want}, truth says {}",
                        entry["fixture_sha256"]
                    ));
                }
            }
        }
        truth::compare_row(row, truth.get(id), &mut violations);
        errors.extend(violations.into_iter().map(|err| format!("{id}.{err}")));
    }
    for id in truth.keys() {
        if !rows.contains_key(id) {
            errors.push(format!(
                "{id}.truth_source: truth entry has no matching row"
            ));
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

// Materialize fixture-only source anchors as the byte offsets the listing exposes.
fn resolve_pins(
    want: &mut Value,
    got: &Value,
    sources: &BTreeMap<String, String>,
    field: &str,
    errors: &mut Vec<String>,
) {
    if field.ends_with(".site") && want.is_object() {
        if want.get("text").is_some() {
            let range = want["file"]
                .as_str()
                .and_then(|file| sources.get(file))
                .ok_or_else(|| format!("missing source for {}", want["file"]))
                .and_then(|source| site_range(want, source));
            match range {
                Ok((start, end)) => {
                    want["start_byte"] = json!(start);
                    want["end_byte"] = json!(end);
                }
                Err(err) => errors.push(format!("{field}.text: {err}")),
            }
            let site = want.as_object_mut().unwrap();
            site.remove("text");
            site.remove("occurrence");
        }
        // The ordinal identifies a raw runtime call. The static listing uses byte offsets.
        want.as_object_mut().unwrap().remove("ordinal");
    }
    match want {
        Value::Object(object) => {
            if let Some(text) = object.remove("construction_text") {
                let found = object
                    .get("file")
                    .and_then(Value::as_str)
                    .and_then(|file| sources.get(file))
                    .zip(
                        got["construction_byte"]
                            .as_u64()
                            .and_then(|n| usize::try_from(n).ok()),
                    )
                    .and_then(|(source, start)| source.get(start..))
                    .zip(nonempty_str(&text))
                    .is_some_and(|(tail, text)| tail.starts_with(text));
                if !found {
                    errors.push(format!(
                        "{field}.construction_byte: expected {text} at byte {}",
                        got["construction_byte"]
                    ));
                }
            }
            for (key, child) in object {
                resolve_pins(child, &got[key], sources, &format!("{field}.{key}"), errors);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter_mut().enumerate() {
                resolve_pins(child, &got[i], sources, &format!("{field}[{i}]"), errors);
            }
        }
        _ => {}
    }
}

fn collect_sources(
    dir: &Path,
    value: &Value,
    sources: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            if object.contains_key("construction_text")
                || (object.contains_key("line") && object.contains_key("text"))
            {
                let file = nonempty_str(&value["file"]).ok_or("source anchor needs a filename")?;
                if !sources.contains_key(file) {
                    let path = checked_path(dir, &value["file"], false)?;
                    let source = fs::read_to_string(&path)
                        .map_err(|err| format!("{}: {err}", path.display()))?;
                    sources.insert(file.to_owned(), source);
                }
            }
            for child in object.values() {
                collect_sources(dir, child, sources)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_sources(dir, child, sources)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_summary(routes: &[Value], summary: &Value, errors: &mut Vec<String>) {
    let mut proven = 0;
    let mut unproven = 0;
    for (i, route) in routes.iter().enumerate() {
        match route["receiver"]["status"].as_str() {
            Some("router") => proven += 1,
            Some("unproven") => unproven += 1,
            _ => errors.push(format!(
                "route[{i}].receiver.status: expected router or unproven"
            )),
        }
        if let Some(callbacks) = route["callbacks"].as_array() {
            for (j, callback) in callbacks.iter().enumerate() {
                if !callback["status"]
                    .as_str()
                    .is_some_and(|status| CALLBACK_STATUSES.contains(&status))
                {
                    errors.push(format!(
                        "route[{i}].callbacks[{j}].status: missing or invalid status"
                    ));
                }
            }
        } else {
            errors.push(format!("route[{i}].callbacks: expected an array"));
        }
    }
    if summary["registrations"] != proven + unproven || summary["registrations"] != routes.len() {
        errors.push(format!("summary.registrations: got {}, listed {} routes ({proven} proven, {unproven} unproven)", summary["registrations"], routes.len()));
    }
    if summary["proven"] != proven {
        errors.push(format!(
            "summary.proven: expected {proven}, got {}",
            summary["proven"]
        ));
    }
}

fn compare_listing(
    expected: &Value,
    expected_mounts: &Value,
    output: &str,
    sources: &BTreeMap<String, String>,
) -> Vec<String> {
    let lines: Result<Vec<Value>, _> = output
        .lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).map_err(|err| format!("listing line {}: {err}", i + 1))
        })
        .collect();
    let lines = match lines {
        Ok(lines) => lines,
        Err(err) => return vec![err],
    };
    let Some((last, records)) = lines.split_last() else {
        return vec!["listing is missing its summary".to_owned()];
    };
    let Some(summary) = last
        .get("summary")
        .filter(|_| last.as_object().is_some_and(|line| line.len() == 1))
    else {
        return vec!["the last listing line must be a summary".to_owned()];
    };
    let mut errors = Vec::new();
    let mut routes = Vec::new();
    let mut mounts = Vec::new();
    for (i, line) in records.iter().enumerate() {
        if line.get("summary").is_some() {
            errors.push(format!(
                "listing line {}: summary precedes the last line",
                i + 1
            ));
        } else if let Some(mount) = line.get("mount") {
            if !line.as_object().is_some_and(|object| object.len() == 1) || !mount.is_object() {
                errors.push(format!("listing line {}: expected a mount object", i + 1));
            }
            mounts.push(mount.clone());
        } else {
            routes.push(line.clone());
        }
    }
    check_summary(&routes, summary, &mut errors);
    for (field, want, got) in [
        ("expect", expected, Value::Array(routes)),
        ("expect_mounts", expected_mounts, Value::Array(mounts)),
    ] {
        let mut want = want.clone();
        resolve_pins(&mut want, &got, sources, field, &mut errors);
        compare_pins(&want, Some(&got), field, &mut errors);
    }
    errors
}

#[test]
fn routes_matrix_matches_rows() {
    if ENABLED_GROUPS.is_empty() {
        return;
    }
    let root = fixture_root();
    let mut errors = Vec::new();
    let rows = read_rows(&root.join("rows.jsonl"), &mut errors);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    for (id, row) in rows {
        if row.get("ignore").is_some()
            || !row["group"]
                .as_str()
                .is_some_and(|group| ENABLED_GROUPS.contains(&group))
        {
            continue;
        }
        let dir =
            checked_path(&root, &row["dir"], true).unwrap_or_else(|err| panic!("{id}.dir: {err}"));
        let mut sources = BTreeMap::new();
        for key in ["expect", "expect_mounts"] {
            collect_sources(&dir, &row[key], &mut sources)
                .unwrap_or_else(|err| panic!("{id}.{key}: {err}"));
        }
        let out = crate::prism_bin()
            .unwrap()
            .args(["nav", "--no-cache", "routes", "--repo"])
            .arg(&dir)
            .args(["--mounts", "--format", "json"])
            .output()
            .unwrap_or_else(|err| panic!("{id}: {err}"));
        assert!(
            out.status.success(),
            "{id}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let output = std::str::from_utf8(&out.stdout).unwrap_or_else(|err| panic!("{id}: {err}"));
        let differences = compare_listing(&row["expect"], &row["expect_mounts"], output, &sources);
        errors.extend(differences.into_iter().map(|err| format!("{id}.{err}")));
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

fn listing_example() -> Value {
    json!({
        "site": {"file": "app.js", "line": 7},
        "receiver": {"status": "router"},
        "paths": [{"status": "full", "value": "/x"}],
        "callbacks": [
            {"position": 0, "status": "resolved", "name": "first"},
            {"position": 1, "status": "resolved", "name": "second"}
        ]
    })
}

fn compare_example(expected: Value, actual: Value) -> Vec<String> {
    let output = format!(
        "{actual}\n{}",
        json!({"summary": {"registrations": 1, "proven": 1}})
    );
    compare_listing(&json!([expected]), &json!([]), &output, &BTreeMap::new())
}

#[test]
fn rejects_swapped_callbacks() {
    let expected = listing_example();
    let mut actual = expected.clone();
    actual["callbacks"][0]["name"] = json!("second");
    actual["callbacks"][1]["name"] = json!("first");
    assert!(compare_example(expected, actual)
        .iter()
        .any(|error| error.starts_with("expect[0].callbacks[0].name:")));
}

#[test]
fn rejects_wrong_site_line() {
    let expected = listing_example();
    let mut actual = expected.clone();
    actual["site"]["line"] = json!(8);
    assert!(compare_example(expected, actual)
        .iter()
        .any(|error| error.starts_with("expect[0].site.line:")));
}

#[test]
fn rejects_full_and_suffix_mismatch() {
    let expected = listing_example();
    let mut actual = expected.clone();
    actual["paths"][0]["status"] = json!("suffix");
    assert!(compare_example(expected, actual)
        .iter()
        .any(|error| error.starts_with("expect[0].paths[0].status:")));
}

#[test]
fn ignores_unpinned_fields() {
    let expected = listing_example();
    let mut actual = expected.clone();
    actual["callbacks"][0]["end_line"] = json!(20);
    assert_eq!(compare_example(expected, actual), Vec::<String>::new());
}

#[test]
fn compares_mount_site_using_byte_offsets_and_occurrence() {
    let sources = BTreeMap::from([("app.js".to_owned(), "é use(r)\nuse(r)\n".to_owned())]);
    let expected =
        json!([{"site": {"file": "app.js", "line": 2, "text": "use(r)", "occurrence": 2}}]);
    let mut mount =
        json!({"mount": {"site": {"file": "app.js", "line": 2, "start_byte": 10, "end_byte": 16}}});
    let summary = json!({"summary": {"registrations": 0, "proven": 0}});
    let check = |mount: &Value| {
        compare_listing(
            &json!([]),
            &expected,
            &format!("{mount}\n{summary}"),
            &sources,
        )
    };
    assert!(check(&mount).is_empty());
    mount["mount"]["site"]["end_byte"] = json!(15);
    assert!(check(&mount)
        .iter()
        .any(|error| error.starts_with("expect_mounts[0].site.end_byte:")));
}

#[test]
fn checks_completeness_even_when_receiver_and_callbacks_are_unpinned() {
    let mut actual = listing_example();
    actual["receiver"]["status"] = json!("unproven");
    actual["callbacks"][0]
        .as_object_mut()
        .unwrap()
        .remove("status");
    let summary = json!({"summary": {"registrations": 2, "proven": 1}});
    let output = format!("{actual}\n{summary}");
    let errors = compare_listing(&json!([{}]), &json!([]), &output, &BTreeMap::new());
    for field in [
        "summary.registrations:",
        "summary.proven:",
        "route[0].callbacks[0].status:",
    ] {
        assert!(
            errors.iter().any(|error| error.starts_with(field)),
            "{errors:?}"
        );
    }
}

#[test]
fn rejects_duplicate_ids_and_collects_bad_record_fields() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("rows.jsonl");
    let row = json!({"id": "same", "expect": [{
        "extra": true,
        "paths": [{"status": "partial"}],
        "callbacks": [{"status": "unresolved", "reason": "unknown_reason"}],
        "site": {"file": "app.js", "line": 1, "text": "route()", "occurrence": 2}
    }], "expect_mounts": []});
    fs::write(root.path().join("app.js"), "route()\n").unwrap();
    fs::write(&path, format!("{row}\n{row}\n")).unwrap();
    let mut errors = Vec::new();
    let rows = read_rows(&path, &mut errors);
    validate_records(&rows["same"], Some(root.path()), &mut errors);
    assert!(errors
        .iter()
        .any(|error| error.ends_with("duplicate ID same")));
    for field in [
        "paths[0].status",
        "callbacks[0].reason",
        "callbacks[0].text",
        "site.text",
        "extra",
    ] {
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with(&format!("expect[0].{field}:"))),
            "{errors:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn rejects_linked_and_escaping_row_directories() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("row");
    fs::create_dir_all(dir.join("nested")).unwrap();
    std::os::unix::fs::symlink(&dir, root.path().join("alias")).unwrap();
    assert_eq!(checked_path(root.path(), &json!("row"), true), Ok(dir));
    for path in ["alias", "alias/nested", "row/.."] {
        assert!(
            checked_path(root.path(), &json!(path), true).is_err(),
            "accepted {path}"
        );
    }
}

#[test]
fn site_ordinal_is_validated_and_used_only_for_runtime_truth() {
    let actual = listing_example();
    let mut expected = actual.clone();
    expected["site"]["ordinal"] = json!(2);
    let mut errors = Vec::new();
    validate_site(&expected["site"], None, "site", &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(compare_example(expected.clone(), actual).is_empty());
    expected["site"]["ordinal"] = json!(0);
    validate_site(&expected["site"], None, "site", &mut errors);
    assert_eq!(errors, ["site.ordinal: expected a positive integer"]);
}
