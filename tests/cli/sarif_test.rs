//! CLI end-to-end tests for the `--format sarif` FINDING MAPPING: the
//! single-algorithm document shape and ordering (§7.2.1), the multi-algorithm
//! run (§7.2.2), load warnings (§2.3.2), plus in-process document tests for
//! the mapping cases no CLI fixture can produce.
//!
//! The run-level tests — allow-list (§7.2.5), algorithm errors (§7.2.3), byte
//! determinism (§7.2.4), degraded parse (§7.2.10) — live in
//! `sarif_shape_test.rs`, which keeps its own copies of the helpers below per
//! repo convention for CLI test files.
//!
//! Structural only: every assertion names a key and a value, never a byte
//! offset. The build-specific keys of §2.2.1 (`tool.driver.version`,
//! `semanticVersion`, `properties.prism_build_identity`, `prism_git_sha`,
//! `binary_input_dirty`) are never compared.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn prism_cmd() -> Command {
    Command::cargo_bin("prism").unwrap()
}

/// Write `files` (relative path -> content) into a fresh temp repo, plus a
/// JSON diff spec (the `DiffInput::from_json` shape) marking `diff_lines`
/// per file. Returns `(TempDir, repo_dir, diff_json_path)` — keep the
/// `TempDir` alive for the duration of the test.
fn write_repo(
    files: &[(&str, &str)],
    diff_files: &[(&str, &[usize])],
) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().to_path_buf();
    for (path, content) in files {
        let full = repo.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, content).unwrap();
    }

    let files_json: Vec<Value> = diff_files
        .iter()
        .map(|(path, lines)| {
            serde_json::json!({
                "file_path": path,
                "modify_type": "Modified",
                "diff_lines": lines,
            })
        })
        .collect();
    let spec = serde_json::json!({ "files": files_json });
    let diff_path = repo.join("diff.json");
    fs::write(&diff_path, serde_json::to_string_pretty(&spec).unwrap()).unwrap();

    (tmp, repo, diff_path)
}

/// Fixture A (absence): `a.py` opens a file on line 2 inside `def read():`
/// and never closes it. Diff = `a.py` line 2.
fn fixture_absence() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    write_repo(
        &[("a.py", "def read():\n    f = open(\"x\")\n    return f\n")],
        &[("a.py", &[2])],
    )
}

/// Fixture B (echo): `client.py::fetch` is the changed function (its `return`
/// line is diff-touched, which is what echo keys on); `svc.py::handle` calls
/// it with no try/except and no result check. `svc.py` carries an empty
/// diff-line set purely so the CLI parses it into the CPG — the CPG is built
/// from the diff's files, so a caller outside the diff is invisible.
fn fixture_echo() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    write_repo(
        &[
            (
                "client.py",
                "def fetch():\n    f = open(\"data.txt\")\n    return f.read()\n",
            ),
            (
                "svc.py",
                "from client import fetch\n\ndef handle():\n    return fetch()\n",
            ),
        ],
        &[("client.py", &[2, 3]), ("svc.py", &[])],
    )
}

fn run_sarif(repo: &std::path::Path, diff: &std::path::Path, algorithm: &str) -> (Vec<u8>, Value) {
    let out = prism_cmd()
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            algorithm,
            "--format",
            "sarif",
        ])
        .output()
        .unwrap();
    let doc: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}); stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.stdout, doc)
}

fn run(doc: &Value) -> &Value {
    &doc["runs"][0]
}

fn results(doc: &Value) -> &Vec<Value> {
    run(doc)["results"].as_array().unwrap()
}

fn rules(doc: &Value) -> &Vec<Value> {
    run(doc)["tool"]["driver"]["rules"].as_array().unwrap()
}

/// §2.2.1 / §7.2.1 + §7.2.2: `results[i].ruleIndex` indexes `results[i].ruleId`
/// in the driver's rule array — asserted for EVERY result, not just the first.
fn assert_rule_index_correspondence(doc: &Value) {
    let rules = rules(doc);
    for r in results(doc) {
        let idx = r["ruleIndex"].as_u64().unwrap() as usize;
        assert_eq!(
            rules[idx]["id"], r["ruleId"],
            "rules[{idx}].id must equal result.ruleId {:?}",
            r["ruleId"]
        );
    }
}

/// §7.2.1
#[test]
fn single_algorithm_sarif_shape_and_rule_index() {
    let (_tmp, repo, diff) = fixture_absence();
    let (_bytes, doc) = run_sarif(&repo, &diff, "absence");

    assert_eq!(doc["version"], "2.1.0");
    assert_eq!(
        doc["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(run(&doc)["tool"]["driver"]["name"], "prism");

    let rules = rules(&doc);
    assert_eq!(
        rules.len(),
        1,
        "one distinct ruleId => one rule: {rules:#?}"
    );
    assert_eq!(rules[0]["id"], "prism/absence/missing_counterpart");
    assert_eq!(rules[0]["name"], "missing_counterpart");
    assert!(
        !rules[0]["fullDescription"]["text"]
            .as_str()
            .unwrap()
            .is_empty(),
        "fullDescription.text must be a real sentence, not empty"
    );

    let results = results(&doc);
    assert!(!results.is_empty(), "absence must produce a finding");
    for r in results {
        assert_eq!(r["level"], "warning");
        let loc = &r["locations"][0]["physicalLocation"];
        assert_eq!(loc["artifactLocation"]["uri"], "a.py");
        assert_eq!(loc["artifactLocation"]["uriBaseId"], "%SRCROOT%");
        assert_eq!(loc["region"]["startLine"], 2);
        assert_eq!(
            r["locations"][0]["logicalLocations"][0]["name"], "read",
            "function_name maps to a logicalLocation"
        );
        assert_eq!(r["locations"][0]["logicalLocations"][0]["kind"], "function");
        let p = &r["properties"];
        assert_eq!(p["algorithm"], "absence");
        assert_eq!(p["category"], "missing_counterpart");
        assert_eq!(p["severity"], "warning");
        assert_eq!(p["confidence"], "exact");
        assert_eq!(p["tier"], "asserted");
        assert_eq!(p["parse_quality"], "clean");
        assert_eq!(p["resolution_mode"], "nominal");
        assert!(
            r["partialFingerprints"]["prism/finding/v1"].is_string(),
            "every result carries the v1 fingerprint"
        );
    }
    assert_rule_index_correspondence(&doc);

    // §2.2.4's FOURTH sort key. This fixture is the only one that exercises it:
    // four pattern families match the same `open(` call, so every result shares
    // `(uri, startLine, ruleId)` and only `message.text` can order them. Without
    // this assertion the message component of the sort is untested and could be
    // dropped without any test noticing.
    let messages: Vec<&str> = results
        .iter()
        .map(|r| r["message"]["text"].as_str().unwrap())
        .collect();
    assert!(
        messages.len() > 1,
        "fixture must yield several same-location results to pin the message key: {messages:?}"
    );
    let mut sorted_messages = messages.clone();
    sorted_messages.sort_unstable();
    assert_eq!(
        messages, sorted_messages,
        "results sharing (uri, line, ruleId) must be ordered by message.text"
    );

    assert!(
        run(&doc).get("originalUriBaseIds").is_none(),
        "a direct producer must not set originalUriBaseIds (sol #23)"
    );
    assert_eq!(run(&doc)["invocations"][0]["executionSuccessful"], true);
    assert_eq!(run(&doc)["properties"]["mapping_version"], "1");
    assert_eq!(run(&doc)["properties"]["resolution_mode"], "nominal");
    assert_eq!(
        run(&doc)["properties"]["algorithms_run"],
        serde_json::json!(["AbsenceSlice"])
    );
    // Build-specific keys: present, never compared (§2.2.1).
    for key in [
        "prism_build_identity",
        "prism_git_sha",
        "binary_input_dirty",
    ] {
        assert!(
            run(&doc)["properties"].get(key).is_some(),
            "run.properties.{key} must be present"
        );
    }
    for key in ["version", "semanticVersion", "informationUri"] {
        assert!(
            run(&doc)["tool"]["driver"].get(key).is_some(),
            "driver.{key} must be present"
        );
    }
}

/// §7.2.2
#[test]
fn multi_algorithm_sarif_is_sorted_and_grades_cpg_evidence() {
    let (_tmp, repo, diff) = fixture_echo();
    let (_bytes, doc) = run_sarif(&repo, &diff, "echo,absence");

    assert_eq!(
        run(&doc)["properties"]["algorithms_run"],
        serde_json::json!(["EchoSlice", "AbsenceSlice"]),
        "algorithms_run keeps CLI order, not sorted order"
    );

    let results = results(&doc);
    let echo: Vec<&Value> = results
        .iter()
        .filter(|r| r["ruleId"] == "prism/echo/missing_error_handling")
        .collect();
    assert_eq!(
        echo.len(),
        1,
        "echo must flag the unguarded caller: {results:#?}"
    );
    assert_eq!(echo[0]["properties"]["confidence"], "exact");
    assert_eq!(echo[0]["properties"]["tier"], "asserted");
    assert_eq!(echo[0]["properties"]["algorithm"], "echo");
    assert_eq!(
        echo[0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "svc.py"
    );

    assert!(
        results
            .iter()
            .any(|r| r["ruleId"] == "prism/absence/missing_counterpart"),
        "the AST algorithm's findings survive the same run"
    );

    // §2.2.4 ordering: (uri, startLine, ruleId) is non-decreasing.
    let key = |r: &Value| {
        (
            r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
                .as_str()
                .unwrap()
                .to_string(),
            r["locations"][0]["physicalLocation"]["region"]["startLine"]
                .as_u64()
                .unwrap_or(0),
            r["ruleId"].as_str().unwrap().to_string(),
        )
    };
    let keys: Vec<_> = results.iter().map(key).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(
        keys, sorted,
        "results must be sorted by (uri, line, ruleId)"
    );

    // rules sorted by id.
    let ids: Vec<&str> = rules(&doc)
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort_unstable();
    assert_eq!(ids, sorted_ids, "rules must be sorted by id");

    assert_rule_index_correspondence(&doc);
}

/// §2.3.2 load warnings: a diff file in an unsupported language is skipped at
/// load and surfaces as one `warning` notification.
#[test]
fn skipped_unsupported_file_becomes_a_notification() {
    let (_tmp, repo, diff) = write_repo(
        &[
            ("a.py", "def read():\n    f = open(\"x\")\n    return f\n"),
            ("notes.txt", "not source\n"),
        ],
        &[("a.py", &[2]), ("notes.txt", &[1])],
    );
    let (_bytes, doc) = run_sarif(&repo, &diff, "absence");

    let notes = run(&doc)["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .unwrap();
    assert!(
        notes.iter().any(|n| n["level"] == "warning"
            && n["message"]["text"]
                == "skipped unsupported file: notes.txt (unsupported language)"),
        "load warnings become warning notifications: {notes:#?}"
    );
    assert_eq!(
        run(&doc)["invocations"][0]["executionSuccessful"],
        true,
        "a skipped file is not an execution failure"
    );
}

/// In-process document tests for the mapping cases no CLI fixture can produce:
/// §7.2.7's unknown severity and §7.2.9's escaping path, plus a category-less
/// finding. They call only the public serializer API. (§7.2.6's attribution
/// cases are unit tests inside `src/output/sarif.rs`, and the tables they use
/// are unit-tested in `src/output/sarif_rules.rs`.)
mod document {
    use prism::finding_confidence::EvidencePath;
    use prism::output::{to_sarif, SarifInputs};
    use prism::slice::SliceFinding;
    use serde_json::Value;
    use std::collections::BTreeMap;

    fn finding(file: &str, severity: &str, category: Option<&str>) -> SliceFinding {
        SliceFinding {
            algorithm: "absence".to_string(),
            file: file.to_string(),
            line: 3,
            severity: severity.to_string(),
            description: "d".to_string(),
            function_name: None,
            related_lines: vec![],
            related_files: vec![],
            category: category.map(str::to_string),
            parse_quality: None,
            diagrams: vec![],
        }
    }

    /// `SarifInputs` is `#[non_exhaustive]` (§2.3.1), so this builds it the way
    /// an embedder must: `new` + setters, never a struct literal. Everything
    /// these cases do not set keeps the builder's empty default.
    fn doc(findings: &[SliceFinding]) -> Value {
        let evidence = vec![Some(EvidencePath::default()); findings.len()];
        to_sarif(&SarifInputs::new(findings).evidence(&evidence))
    }

    /// The builder's setters are what the CLI's two SARIF arms use to pass the
    /// four warning channels; each must reach the document, and one unset
    /// channel must not disturb the others.
    #[test]
    fn builder_defaults_are_empty_and_every_setter_reaches_the_document() {
        let quality = BTreeMap::new();
        let files = BTreeMap::new();
        let mut sources = BTreeMap::new();
        sources.insert("a.py".to_string(), "one\ntwo\nthree\n".to_string());
        let algorithms = ["AbsenceSlice".to_string()];
        let parse = ["parse warning".to_string()];
        let load = ["load warning".to_string()];
        let build = ["build warning".to_string()];

        let findings = [finding("a.py", "warning", Some("c"))];
        let evidence = [Some(EvidencePath::default())];
        let doc = to_sarif(
            &SarifInputs::new(&findings)
                .evidence(&evidence)
                .parse_warnings(&parse)
                .load_warnings(&load)
                .build_warnings(&build)
                .algorithms_run(&algorithms)
                .parse_quality(&quality)
                .files(&files)
                .sources(&sources),
        );

        assert_eq!(
            notification_texts(&doc),
            ["parse warning", "load warning", "build warning"],
            "all three warning channels are notified, in channel order"
        );
        assert_eq!(
            doc["runs"][0]["properties"]["algorithms_run"][0],
            "AbsenceSlice"
        );
        assert_eq!(
            doc["runs"][0]["invocations"][0]["executionSuccessful"], true,
            "`errors` kept its empty default"
        );

        // The sources map reached `fingerprint` via `line_text_of`: the same
        // finding fingerprints differently once its line text is known.
        let without_sources = to_sarif(&SarifInputs::new(&findings).evidence(&evidence));
        let fingerprint = |d: &Value| {
            d["runs"][0]["results"][0]["partialFingerprints"]["prism/finding/v1"].clone()
        };
        assert_ne!(fingerprint(&doc), fingerprint(&without_sources));
        assert!(
            notification_texts(&without_sources).is_empty(),
            "a default-built SarifInputs notifies nothing"
        );
    }

    #[test]
    fn missing_evidence_warns_and_is_never_empty_exact() {
        let findings = [finding("a.py", "warning", Some("c"))];
        let doc = to_sarif(&SarifInputs::new(&findings));
        let result = &doc["runs"][0]["results"][0];
        assert_eq!(result["properties"]["confidence"], "unlabeled");
        assert_eq!(result["properties"]["tier"], "candidate");
        assert!(notification_texts(&doc)
            .iter()
            .any(|warning| warning.contains("evidence alignment mismatch")));
    }

    fn notification_texts(doc: &Value) -> Vec<String> {
        doc["runs"][0]["invocations"][0]["toolExecutionNotifications"]
            .as_array()
            .map(|ns| {
                ns.iter()
                    .map(|n| n["message"]["text"].as_str().unwrap().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// §7.2.7 at document level: an unknown severity becomes `error` (louder,
    /// never invisible — §5.5) and the original string survives in properties.
    #[test]
    fn unknown_severity_becomes_error_and_keeps_the_original() {
        let doc = doc(&[finding("a.py", "critical", Some("c"))]);
        let result = &doc["runs"][0]["results"][0];
        assert_eq!(result["level"], "error");
        assert_eq!(result["properties"]["severity"], "critical");
    }

    /// §7.2.9 at document level: an escaping path is still emitted (§5.3 —
    /// nothing is dropped) and raises one warning however many findings name it.
    #[test]
    fn escaping_path_is_emitted_with_one_warning_notification() {
        let escaping = finding("../x.py", "warning", Some("c"));
        let doc = doc(&[escaping.clone(), escaping]);
        let uri = &doc["runs"][0]["results"][0]["locations"][0]["physicalLocation"]
            ["artifactLocation"]["uri"];
        assert_eq!(uri, "../x.py");
        assert_eq!(
            notification_texts(&doc),
            ["path escapes repo root: ../x.py"],
            "a repeated escaping path is reported once"
        );
    }

    /// A category-less finding still yields a well-formed rule, and a clean run
    /// omits the empty collections rather than emitting `[]`.
    #[test]
    fn missing_category_becomes_uncategorized_and_empty_collections_are_omitted() {
        let doc = doc(&[finding("a.py", "warning", None)]);
        let result = &doc["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "prism/absence/uncategorized");
        assert_eq!(result["properties"]["category"], "uncategorized");
        assert!(
            result.get("relatedLocations").is_none(),
            "no related data => no empty array"
        );
        let rule = &doc["runs"][0]["tool"]["driver"]["rules"][0];
        assert_eq!(rule["id"], "prism/absence/uncategorized");
        assert_eq!(rule["fullDescription"]["text"], "absence: uncategorized");

        let invocation = &doc["runs"][0]["invocations"][0];
        assert_eq!(invocation["executionSuccessful"], true);
        assert!(invocation.get("toolExecutionNotifications").is_none());
        assert!(doc["runs"][0]["properties"].get("errors").is_none());
    }
}
