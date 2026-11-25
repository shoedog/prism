//! Fixture and saved receipt validation, independent of CLI queries.
use super::{fixture_root, pins, truth};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

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
pub(super) const CALLBACK_STATUSES: &[&str] = &["resolved", "anonymous", "unresolved"];
const PATH_STATUSES: &[&str] = &["full", "suffix", "dynamic"];

pub(super) fn read_rows(path: &Path, errors: &mut Vec<String>) -> BTreeMap<String, Value> {
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

pub(super) fn nonempty_str(value: &Value) -> Option<&str> {
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

pub(super) fn checked_path(root: &Path, value: &Value, directory: bool) -> Result<PathBuf, String> {
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

pub(super) fn site_range(site: &Value, source: &str) -> Result<(usize, usize), String> {
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

pub(super) fn validate_site(
    site: &Value,
    dir: Option<&Path>,
    field: &str,
    errors: &mut Vec<String>,
) {
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

/// SHA-256 over a row directory, as `truth/run_truth.py` computes it: each relative path, NUL, bytes, NUL,
/// in path order. A fixture edited without regenerating truth then fails the well-formedness test.
pub(super) fn tree_sha256(dir: &Path) -> String {
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
        match (dir.as_deref(), truth.get(id)) {
            (Some(dir), Some(entry)) if row["truth_source"] == "executed" => {
                truth::compare_receipt(row, entry, &tree_sha256(dir), &mut violations);
            }
            (_, entry) => truth::compare_row(row, entry, &mut violations),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
