use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod binding;
mod listing;
mod pins;
mod truth;

const ENABLED_IDS: &[&str] = &["e01", "e02"];
const ROUTE_FIELDS: &[&str] = &[
    "framework",
    "protocol",
    "element_site",
    "install_site",
    "element_ordinal",
    "origin_plugin",
    "default_methods",
    "endpoint",
    "candidate",
    "reason",
    "superseded_keys",
    "conflicts",
    "methods",
    "path",
    "paths",
    "callbacks",
    "receiver",
    "site",
];
const CALLBACK_STATUSES: &[&str] = &["resolved", "anonymous", "unresolved"];
const PATH_STATUSES: &[&str] = &["full", "suffix", "dynamic"];
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
                if matches!(key.as_str(), "reason" | "mount_refusal" | "expect_error")
                    && serde_json::from_value::<prism::framework_entries::routes::Reason>(
                        child.clone(),
                    )
                    .is_err()
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
            for (key, site) in object.iter().filter(|(key, _)| pins::is_site(key)) {
                validate_site(site, dir, &format!("{field}.{key}"), errors);
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
        for key in ["expect_diagnostics", "expect_failed_roots", "entry_queries"] {
            validate_reasons(&row[key], key, &mut violations);
        }
        if let Some(dir) = dir.as_deref() {
            pins::validate(row, dir, &mut violations);
        }
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

fn check_summary(routes: &[Value], mounts: &[Value], summary: &Value, errors: &mut Vec<String>) {
    let mut proven = 0;
    let mut unproven = 0;
    let mut candidates = 0;
    for (i, route) in routes.iter().enumerate() {
        match route["receiver"]["status"].as_str() {
            Some("router") => proven += 1,
            Some("unproven") => unproven += 1,
            _ => errors.push(format!(
                "route[{i}].receiver.status: expected router or unproven"
            )),
        }
        candidates += usize::from(route["candidate"] == true);
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
    for (field, count) in [
        ("registrations", routes.len()),
        ("proven", proven),
        ("unproven", unproven),
        ("candidates", candidates),
        ("mounts", mounts.len()),
    ] {
        if summary[field] != count {
            errors.push(format!(
                "summary.{field}: expected {count}, got {}",
                summary[field]
            ));
        }
    }
}

struct Listing {
    routes: Vec<Value>,
    mounts: Vec<Value>,
    summary: Value,
}

fn read_listing(output: &str) -> Result<Listing, String> {
    let mut lines: Vec<Value> = output
        .lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).map_err(|err| format!("listing line {}: {err}", i + 1))
        })
        .collect::<Result<_, _>>()?;
    let mut last = lines.pop().ok_or("listing is missing its summary")?;
    let summary = last
        .as_object_mut()
        .filter(|line| line.len() == 1)
        .and_then(|line| line.remove("summary"))
        .ok_or("the last listing line must be a summary")?;
    let mut listing = Listing {
        routes: Vec::new(),
        mounts: Vec::new(),
        summary,
    };
    for (index, mut line) in lines.into_iter().enumerate() {
        if line.get("summary").is_some() || line.get("error").is_some() {
            return Err(format!(
                "listing line {}: unexpected summary or error",
                index + 1
            ));
        }
        if let Some(mount) = line
            .as_object_mut()
            .filter(|line| line.len() == 1)
            .and_then(|line| line.remove("mount"))
        {
            if !mount.is_object() {
                return Err(format!(
                    "listing line {}: expected a mount object",
                    index + 1
                ));
            }
            listing.mounts.push(mount);
        } else if listing.mounts.is_empty() && line.get("mount").is_none() {
            listing.routes.push(line);
        } else {
            return Err(format!(
                "listing line {}: malformed mount or route after mounts",
                index + 1
            ));
        }
    }
    Ok(listing)
}

fn compare_listing(
    expected: &Value,
    expected_mounts: &Value,
    output: &str,
    sources: &BTreeMap<String, String>,
) -> Vec<String> {
    let listing = match read_listing(output) {
        Ok(listing) => listing,
        Err(error) => return vec![error],
    };
    let mut errors = Vec::new();
    check_summary(
        &listing.routes,
        &listing.mounts,
        &listing.summary,
        &mut errors,
    );
    listing::compare_collection(expected, &listing.routes, sources, "expect", &mut errors);
    listing::compare_collection(
        expected_mounts,
        &listing.mounts,
        sources,
        "expect_mounts",
        &mut errors,
    );
    errors
}

fn query(dir: &Path, entry: Option<&str>, format: &str) -> std::process::Output {
    let mut command = crate::prism_std();
    command
        .args(["nav", "--no-cache", "routes", "--repo"])
        .arg(dir);
    command.args(["--mounts", "--format", format]);
    if let Some(entry) = entry {
        command.args(["--entry", entry]);
    }
    command.output().unwrap()
}

fn check_roots(routes: &[Value], root: &Value, errors: &mut Vec<String>) {
    for (index, route) in routes.iter().enumerate() {
        for (position, path) in route["paths"].as_array().into_iter().flatten().enumerate() {
            if path["status"] == "full" && path["root"] != *root {
                errors.push(format!(
                    "route[{index}].paths[{position}].root: expected {root}, got {}",
                    path["root"]
                ));
            }
        }
    }
}

fn successful_query(dir: &Path, entry: Option<&str>) -> Result<Listing, String> {
    let output = query(dir, entry, "json");
    if !output.status.success() {
        return Err(format!(
            "entry {entry:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|error| error.to_string())?;
    read_listing(text)
}

fn compare_row_listing(dir: &Path, row: &Value, errors: &mut Vec<String>) -> Result<(), String> {
    let mut sources = BTreeMap::new();
    for key in [
        "expect",
        "expect_mounts",
        "expect_diagnostics",
        "expect_failed_roots",
        "entry_queries",
    ] {
        pins::collect_sources(dir, &row[key], &mut sources)
            .map_err(|error| format!("{key}: {error}"))?;
    }
    let entry = row.get("query_entry").unwrap_or(&row["entry"]).as_str();
    let root = entry
        .map(|entry| pins::selected_root(dir, entry))
        .transpose()?;
    let primary = successful_query(dir, entry)?;
    check_summary(&primary.routes, &primary.mounts, &primary.summary, errors);
    listing::validate(
        &primary.routes,
        &primary.mounts,
        &primary.summary,
        Some(dir),
        errors,
    );
    listing::compare_collection(&row["expect"], &primary.routes, &sources, "expect", errors);
    listing::compare_collection(
        &row["expect_mounts"],
        &primary.mounts,
        &sources,
        "expect_mounts",
        errors,
    );
    listing::check_events(row, &primary.summary, &sources, errors);
    if let Some(root) = &root {
        check_roots(&primary.routes, root, errors);
    }
    binding::check_bindings(row, dir, root.as_ref(), &primary.routes, errors);
    for secondary in row["entry_queries"].as_array().into_iter().flatten() {
        let entry = secondary["entry"]
            .as_str()
            .ok_or("secondary entry is missing its selector")?;
        if let Some(reason) = secondary.get("expect_error") {
            check_query_error(dir, entry, reason, errors);
            continue;
        }
        let root = pins::selected_root(dir, entry)?;
        let result = successful_query(dir, Some(entry))?;
        check_summary(&result.routes, &result.mounts, &result.summary, errors);
        listing::validate(
            &result.routes,
            &result.mounts,
            &result.summary,
            Some(dir),
            errors,
        );
        check_roots(&result.routes, &root, errors);
        for (index, pin) in secondary["expect_paths"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let mut pin = pin.clone();
            let field = format!("{entry}.expect_paths[{index}]");
            pins::resolve_pins(&mut pin, &sources, &field, errors);
            let found = result
                .routes
                .iter()
                .filter(|route| route["site"] == pin["site"])
                .collect::<Vec<_>>();
            if let [route] = found.as_slice() {
                compare_pins(&pin["paths"], route.get("paths"), &field, errors);
            } else {
                errors.push(format!(
                    "{field}: expected one origin at {}, got {}",
                    pin["site"],
                    found.len()
                ));
            }
        }
    }
    Ok(())
}

fn check_query_error(dir: &Path, entry: &str, reason: &Value, errors: &mut Vec<String>) {
    let output = query(dir, Some(entry), "json");
    let expected = json!({"error": {"entry": entry, "reason": reason}});
    if output.status.success()
        || serde_json::from_slice::<Value>(&output.stdout)
            .ok()
            .as_ref()
            != Some(&expected)
        || String::from_utf8_lossy(&output.stdout).lines().count() != 1
    {
        errors.push(format!(
            "entry {entry:?}: expected {expected}, got {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
}

#[test]
fn routes_matrix_matches_rows() {
    let root = fixture_root();
    let mut errors = Vec::new();
    let rows = read_rows(&root.join("rows.jsonl"), &mut errors);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    for id in ENABLED_IDS {
        let row = &rows[*id];
        let dir =
            checked_path(&root, &row["dir"], true).unwrap_or_else(|err| panic!("{id}.dir: {err}"));
        let mut differences = Vec::new();
        if let Err(error) = compare_row_listing(&dir, row, &mut differences) {
            differences.push(error);
        }
        errors.extend(differences.into_iter().map(|error| format!("{id}.{error}")));
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn core_routes_text_json_and_explicit_refusals() {
    let root = fixture_root();
    for (id, selectors) in [
        ("e01", &["../outside.js", "app:bad(", ""][..]),
        ("e02", &["app:app", "app:factory()", "app.js#default"][..]),
    ] {
        let dir = root.join(id);
        let run = |format: &str, entry: Option<&str>| {
            let mut command = crate::prism_std();
            command
                .args(["nav", "--no-cache", "routes", "--repo"])
                .arg(&dir);
            command.args(["--format", format]);
            if let Some(entry) = entry {
                command.args(["--entry", entry]);
            }
            command.output().unwrap()
        };
        let output = run("json", None);
        assert!(
            output.status.success(),
            "{id}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.lines().count(), 1);
        assert_eq!(
            serde_json::from_str::<Value>(&output).unwrap(),
            json!({
                "summary": {"enabled_adapters": [], "registrations": 0, "proven": 0,
                    "unproven": 0, "candidates": 0, "mounts": 0, "diagnostics": [], "failed_roots": []}
            })
        );
        let output = run("text", None);
        assert!(
            output.status.success(),
            "{id}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(),
            "summary candidates=0 diagnostics=[] enabled_adapters=[] failed_roots=[] mounts=0 proven=0 registrations=0 unproven=0\n");
        for entry in selectors {
            let output = run("json", Some(entry));
            assert!(!output.status.success(), "{id} {entry:?}");
            assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 1);
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                json!({"error": {"entry": entry, "reason": "entry_unproven"}})
            );
        }
    }
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
        json!({"summary": {"registrations": 1, "proven": 1, "unproven": 0, "candidates": 0, "mounts": 0}})
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
fn matches_by_object_qualified_origin_before_comparing_roles_and_roots() {
    let mut first = listing_example();
    first["origin_plugin"] = json!({"file":"plugins.js","object_byte":5});
    first["paths"][0]["root"] = json!({"file":"app.js","binding":"first"});
    first["callbacks"][0]["role"] = json!("handler");
    let mut second = first.clone();
    second["origin_plugin"]["object_byte"] = json!(9);
    second["paths"][0]["root"]["binding"] = json!("second");
    second["callbacks"][0]["role"] = json!("pre");
    let summary =
        json!({"summary":{"registrations":2,"proven":2,"unproven":0,"candidates":0,"mounts":0}});
    let compare = |a: &Value, b: &Value| {
        compare_listing(
            &json!([first, second]),
            &json!([]),
            &format!("{a}\n{b}\n{summary}\n"),
            &BTreeMap::new(),
        )
    };
    assert!(compare(&second, &first).is_empty());
    let mut wrong = first.clone();
    wrong["paths"][0]["root"] = second["paths"][0]["root"].clone();
    wrong["callbacks"][0]["role"] = json!("pre");
    let errors = compare(&second, &wrong);
    for field in ["paths[0].root.binding", "callbacks[0].role"] {
        assert!(
            errors.iter().any(|error| error.contains(field)),
            "{errors:?}"
        );
    }
    wrong["origin_plugin"]["object_byte"] = json!(9);
    assert!(!compare(&second, &wrong).is_empty());
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
    let summary = json!({"summary": {"registrations": 0, "proven": 0, "unproven": 0, "candidates": 0, "mounts": 1}});
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
