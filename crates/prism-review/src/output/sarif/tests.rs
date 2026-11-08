use super::*;
use serde_json::Value;

fn finding(algorithm: &str, file: &str, line: usize) -> SliceFinding {
    SliceFinding {
        algorithm: algorithm.to_string(),
        file: file.to_string(),
        line,
        severity: "warning".to_string(),
        description: "d".to_string(),
        function_name: Some("f".to_string()),
        related_lines: vec![],
        related_files: vec![],
        category: Some("c".to_string()),
        parse_quality: None,
        diagrams: vec![],
    }
}

fn doc_of(f: SliceFinding) -> Value {
    to_sarif(&SarifInputs::new(&[f]))
}

/// `"<uri>:<startLine> id=<n>"` of a `location` / `relatedLocation`.
/// A `-` line means the `region` KEY is absent, not a null `startLine`.
fn at(location: &Value, id: u64) -> String {
    let physical = &location["physicalLocation"];
    let line = physical.get("region").map_or("-".to_string(), |r| {
        r["startLine"]
            .as_u64()
            .expect("region without a startLine")
            .to_string()
    });
    let uri = physical["artifactLocation"]["uri"].as_str().unwrap();
    format!("{uri}:{line} id={id}")
}

/// `(relatedLocations rendered by `at`, properties)` of the one result.
fn related_and_properties(f: SliceFinding) -> (Vec<String>, Value) {
    let doc = doc_of(f);
    let result = &doc["runs"][0]["results"][0];
    let related = result["relatedLocations"]
        .as_array()
        .map(|ls| {
            ls.iter()
                .map(|l| at(l, l["id"].as_u64().unwrap()))
                .collect()
        })
        .unwrap_or_default();
    (related, result["properties"].clone())
}

/// §7.2.6 (a): SARIF requires `startLine >= 1`, so line 0 omits the whole
/// `region` KEY — not `"region": {"startLine": null}`, which would fail
/// schema validation while looking the same to a `.as_u64()` probe.
#[test]
fn line_zero_omits_the_region() {
    let zero = doc_of(finding("symmetry", "a.py", 0));
    let physical = &zero["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
    assert_eq!(physical["artifactLocation"]["uri"], "a.py");
    assert!(
        physical.get("region").is_none(),
        "the region key itself must be absent: {physical:#?}"
    );

    let one = doc_of(finding("symmetry", "a.py", 1));
    let location = &one["runs"][0]["results"][0]["locations"][0];
    assert_eq!(at(location, 0), "a.py:1 id=0");
}

/// §7.2.6 (b): SameFile attribution — lines land in `finding.file`,
/// sorted, deduplicated, zeros dropped, ids `0..n`; the related FILE is a
/// separate region-less location.
#[test]
fn same_file_attribution_locates_lines_in_the_finding_file() {
    let mut f = finding("absence", "a.py", 1);
    f.related_lines = vec![5, 0, 3, 5];
    f.related_files = vec!["b.py".to_string()];
    let (related, properties) = related_and_properties(f);
    assert_eq!(related, ["a.py:3 id=0", "a.py:5 id=1", "b.py:- id=2"]);
    assert!(
        properties.get("related_lines").is_none(),
        "every line was attributed, so nothing spills into properties"
    );
    assert_eq!(properties["related_files"], serde_json::json!(["b.py"]));
}

/// §7.2.6 (c): CounterpartFile attribution — symmetry's lines belong to
/// the counterpart file, never to the anchor file.
#[test]
fn counterpart_attribution_locates_lines_in_the_related_file() {
    let mut f = finding("symmetry", "a.py", 1);
    f.related_lines = vec![10, 20];
    f.related_files = vec!["b.py".to_string()];
    let (related, properties) = related_and_properties(f);
    assert_eq!(related, ["b.py:10 id=0", "b.py:20 id=1"]);
    assert!(properties.get("related_lines").is_none());
}

/// §7.2.6 (d): Ambiguous attribution — with candidate files the lines are
/// NOT guessed onto one, they are preserved in `properties`; with no
/// candidate file the finding's own file is the only possibility.
#[test]
fn ambiguous_attribution_preserves_lines_it_cannot_place() {
    let mut f = finding("callback_dispatcher", "a.py", 1);
    f.related_lines = vec![7, 4, 4];
    f.related_files = vec!["c.py".to_string(), "b.py".to_string()];
    let (related, properties) = related_and_properties(f);
    assert_eq!(
        related,
        ["b.py:- id=0", "c.py:- id=1"],
        "files only, sorted"
    );
    assert_eq!(
        properties["related_lines"],
        serde_json::json!([4, 7]),
        "unattributable lines are sorted, deduplicated and kept"
    );

    let mut unknown = finding("brand_new_algo", "a.py", 1);
    unknown.related_lines = vec![9];
    let (related, properties) = related_and_properties(unknown);
    assert_eq!(related, ["a.py:9 id=0"]);
    assert!(properties.get("related_lines").is_none());
}

/// Repo-root escape detection must be HOST-INDEPENDENT (final review,
/// terra #2). A Windows drive-rooted or UNC path is absolute everywhere,
/// but `Path::is_absolute()` answers `false` for both on a Unix host — so
/// the old predicate emitted `C:/outside/a.py` under `%SRCROOT%` as if it
/// were repo-relative, with no warning, whenever prism ran on Unix.
#[test]
fn escape_detection_is_host_independent() {
    // The four escaping shapes, at the shared predicate `targets` uses too.
    for escaping in [
        "C:/outside/a.py",
        "//srv/share/x.py",
        "/abs/x.py",
        "a/../b.py",
    ] {
        assert!(
            path_escapes_repo(escaping),
            "{escaping} escapes the repo root on every host"
        );
        assert!(
            sarif_uri(escaping).1,
            "{escaping} must be flagged by sarif_uri too"
        );
    }
    assert!(
        sarif_uri("C:\\outside\\a.py").1,
        "…including after backslash normalisation"
    );

    for inside in ["a/b.py", "a..b/x.py", "c:x.py", "C:", "src/mod.rs", ""] {
        assert!(
            !path_escapes_repo(inside),
            "{inside:?} stays inside the repo"
        );
    }
}

/// The build warnings `api::build_context` returns are notifications too
/// (final review, terra #1): a non-fatal cache or type-database condition
/// must be visible to a consumer reading only the document.
#[test]
fn build_warnings_become_warning_notifications() {
    let build = ["Warning: failed to write CPG cache: read-only".to_string()];
    let doc = to_sarif(&SarifInputs::new(&[]).build_warnings(&build));
    let notes = doc["runs"][0]["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .expect("a build warning produces a notification")
        .clone();
    assert_eq!(notes.len(), 1, "one notification per warning: {notes:#?}");
    assert_eq!(notes[0]["level"], "warning");
    assert_eq!(notes[0]["message"]["text"], build[0].as_str());
    assert_eq!(
        doc["runs"][0]["invocations"][0]["executionSuccessful"], true,
        "a non-fatal build condition is not an execution failure"
    );
}
