use serde_json::{json, Value};
use std::collections::BTreeMap;

use std::path::{Path, PathBuf};

mod binding;
mod listing;
mod mutations;
mod pins;
mod rows;
mod truth;

use rows::{checked_path, nonempty_str, read_rows, site_range};
use rows::{validate_site, CALLBACK_STATUSES};

const ENABLED_IDS: &[&str] = &["e01", "e02"];
fn fixture_root() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    checked_path(&workspace, &json!("tests/routes_matrix"), true)
        .unwrap_or_else(|err| panic!("fixture root: {err}"))
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
        // The source listing enables Flask while explicit root selection still refuses.
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
                "summary": {"enabled_adapters": ["flask"], "registrations": 0, "proven": 0,
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
            "summary candidates=0 diagnostics=[] enabled_adapters=[\"flask\"] failed_roots=[] mounts=0 proven=0 registrations=0 unproven=0\n");
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

#[test]
fn hyphen_entries_use_equals_and_missing_values_get_usage_errors() {
    let dir = fixture_root().join("e01");
    let run = |args: &[&str]| {
        crate::prism_std()
            .args(["nav", "--no-cache", "routes", "--repo"])
            .arg(&dir)
            .args(args)
            .output()
            .unwrap()
    };
    // The unambiguous spelling lets a missing value followed by an option stay a usage error.
    let help = run(&["--help"]);
    assert!(String::from_utf8_lossy(&help.stdout).contains("--entry=-app.js"));
    for entry in ["-bad", "-app.js"] {
        let output = run(&[&format!("--entry={entry}"), "--format", "json"]);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 1);
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            json!({"error":{"entry":entry,"reason":"entry_unproven"}})
        );
    }
    for args in [&["--entry"][..], &["--entry", "--format", "json"][..]] {
        let missing = run(args);
        assert_eq!(missing.status.code(), Some(2));
        assert!(missing.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&missing.stderr);
        assert!(
            stderr.contains("a value is required") && stderr.contains("--entry"),
            "{stderr}"
        );
    }
}

#[test]
fn malformed_selector_refuses_before_repo_access() {
    let repo = tempfile::tempdir().unwrap();
    let result = crate::prism_std()
        .args(["nav", "--no-cache", "routes", "--repo"])
        .arg(repo.path().join("missing"))
        .args(["--entry", "app:bad(", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap(),
        json!({"error":{"entry":"app:bad(","reason":"entry_unproven"}})
    );
}
