//! Live acceptance tests for `prism targets` (design §7.4.1-§7.4.7b).
//!
//! Fixture discrimination map:
//! - echo: the `external_call` assertion selects `source_algorithm == "echo"`.
//! - absence: the resource/counterpart/confidence assertions select `absence`.
//! - contract: the default-run producer-set assertion requires `contract`.
//! - provenance: the origin property/detail assertions select `provenance`.
//! - membrane: the boundary/callee assertion selects `membrane`.
//!
//! `svc.py` is intentionally present in `diff.json` with no changed lines so it
//! is parsed into the CPG and can supply the cross-file caller for echo and
//! membrane. `client.py` line 14 is the one-line guard/nullable-return contract
//! change; line 15 is the user-input origin, line 16 the unreleased open, and
//! line 20 changes only `serialize_x`, leaving `deserialize_x` unchanged.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const REPO: &str = "tests/fixtures/targets";
const DIFF: &str = "tests/fixtures/targets/diff.json";
const DIFF_UNSUPPORTED: &str = "tests/fixtures/targets/diff-with-unsupported.json";
const DEFAULT_ALGORITHMS: &str = "echo,absence,contract,provenance,membrane";

fn prism_cmd() -> Command {
    Command::cargo_bin("prism").unwrap()
}

fn run(repo: &Path, diff: &Path, algorithm: &str, extra: &[&str]) -> std::process::Output {
    let mut cmd = prism_cmd();
    cmd.arg("targets")
        .arg("--repo")
        .arg(repo)
        .arg("--diff")
        .arg(diff)
        .arg("--algorithm")
        .arg(algorithm);
    cmd.args(extra).output().unwrap()
}

fn fixture_run(algorithm: &str, extra: &[&str]) -> std::process::Output {
    run(Path::new(REPO), Path::new(DIFF), algorithm, extra)
}

fn json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not JSON ({error}); status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn targets(doc: &Value) -> &[Value] {
    doc["targets"].as_array().unwrap()
}

fn target<'a>(doc: &'a Value, algorithm: &str) -> &'a Value {
    targets(doc)
        .iter()
        .find(|target| target["source_algorithm"] == algorithm)
        .unwrap_or_else(|| panic!("missing {algorithm} target in {doc:#}"))
}

/// Small in-repo Draft 2020-12 structural checker. It deliberately implements
/// only the schema features the targets contract uses: required/property
/// closure, const/enum, arrays, integer minima, the id/hash pattern, and local
/// `$ref` resolution into `$defs`.
fn check_against_schema(doc: &Value, schema: &Value) {
    fn resolve<'a>(root: &'a Value, node: &'a Value) -> &'a Value {
        match node.get("$ref").and_then(Value::as_str) {
            Some(reference) => root
                .pointer(reference.strip_prefix('#').expect("local $ref"))
                .unwrap_or_else(|| panic!("unresolved schema ref {reference}")),
            None => node,
        }
    }

    fn walk(value: &Value, raw_schema: &Value, root: &Value, path: &str) {
        let schema = resolve(root, raw_schema);
        if let Some(expected) = schema.get("const") {
            assert_eq!(value, expected, "const mismatch at {path}");
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            assert!(values.contains(value), "enum mismatch at {path}: {value}");
        }
        if schema.get("pattern").and_then(Value::as_str) == Some("^[0-9a-f]{64}$") {
            let text = value
                .as_str()
                .unwrap_or_else(|| panic!("non-string hash at {path}"));
            assert_eq!(text.len(), 64, "hash length at {path}");
            assert!(
                text.bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                "hash alphabet at {path}: {text}"
            );
        }

        match schema.get("type").and_then(Value::as_str) {
            Some("object") => {
                let object = value
                    .as_object()
                    .unwrap_or_else(|| panic!("non-object at {path}"));
                let properties = schema["properties"].as_object().unwrap();
                for key in schema["required"].as_array().into_iter().flatten() {
                    let key = key.as_str().unwrap();
                    assert!(object.contains_key(key), "missing {path}.{key}");
                }
                if schema["additionalProperties"] == false {
                    for key in object.keys() {
                        assert!(properties.contains_key(key), "unknown {path}.{key}");
                    }
                }
                for (key, child) in object {
                    if let Some(child_schema) = properties.get(key) {
                        walk(child, child_schema, root, &format!("{path}.{key}"));
                    }
                }
            }
            Some("array") => {
                let array = value
                    .as_array()
                    .unwrap_or_else(|| panic!("non-array at {path}"));
                if let Some(item_schema) = schema.get("items") {
                    for (index, item) in array.iter().enumerate() {
                        walk(item, item_schema, root, &format!("{path}[{index}]"));
                    }
                }
            }
            Some("string") => assert!(value.is_string(), "non-string at {path}"),
            Some("integer") => {
                let integer = value
                    .as_u64()
                    .unwrap_or_else(|| panic!("non-integer at {path}"));
                if let Some(minimum) = schema.get("minimum").and_then(Value::as_u64) {
                    assert!(integer >= minimum, "{path} below minimum {minimum}");
                }
            }
            Some(other) => panic!("unsupported schema type {other} at {path}"),
            None => {}
        }
    }

    walk(doc, schema, schema, "$");
}

fn contract_schema() -> Value {
    serde_json::from_str(&fs::read_to_string("docs/contracts/targets.schema.json").unwrap())
        .unwrap()
}

#[test]
fn default_run_emits_all_five_producers_and_live_mappings() {
    let output = fixture_run(DEFAULT_ALGORITHMS, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc = json(&output);

    assert_eq!(doc["schema_version"], "1.0");
    assert_eq!(doc["producer"]["tool"], "prism");
    assert_eq!(doc["producer"]["resolution_mode"], "nominal");
    assert_eq!(
        doc["producer"]["algorithms"],
        serde_json::json!([
            "EchoSlice",
            "AbsenceSlice",
            "ContractSlice",
            "ProvenanceSlice",
            "MembraneSlice"
        ])
    );
    assert!(!doc["diff"]["files"].as_array().unwrap().is_empty());
    for algorithm in ["echo", "absence", "contract", "provenance", "membrane"] {
        assert!(
            targets(&doc)
                .iter()
                .any(|value| value["source_algorithm"] == algorithm),
            "fixture did not produce {algorithm}: {doc:#}"
        );
    }

    let echo = target(&doc, "echo");
    assert_eq!(echo["kind"], "external_call");
    assert_eq!(echo["expected"]["property"], "error_handled");
    assert_eq!(echo["dependency_hint"]["callee"], "fetch");
    assert_eq!(echo["confidence"], "unlabeled");
    assert_eq!(echo["tier"], "candidate");
    assert_eq!(echo["site"]["symbol"], "serve");
    assert_eq!(echo["site"]["language"], "python");
    assert!(
        echo["site"]["function_start_line"].as_u64().unwrap()
            <= echo["site"]["line"].as_u64().unwrap()
    );
    assert!(
        echo["site"]["line"].as_u64().unwrap()
            <= echo["site"]["function_end_line"].as_u64().unwrap()
    );

    let membrane = target(&doc, "membrane");
    assert_eq!(membrane["kind"], "boundary");
    assert_eq!(membrane["dependency_hint"]["callee"], "fetch");

    let absence = targets(&doc)
        .iter()
        .find(|value| {
            value["source_algorithm"] == "absence"
                && value["dependency_hint"]["counterpart"] == "close"
        })
        .expect("file-open absence target");
    assert_eq!(absence["kind"], "resource_acquire");
    assert_eq!(absence["expected"]["property"], "resource_released");
    assert_eq!(absence["dependency_hint"]["kind"], "filesystem");
    assert_eq!(absence["confidence"], "exact");
    assert_eq!(absence["tier"], "asserted");
    assert_eq!(absence["parse_quality"], "clean");

    let provenance = targets(&doc)
        .iter()
        .find(|value| {
            value["source_algorithm"] == "provenance"
                && value["description"]
                    .as_str()
                    .unwrap()
                    .contains("user_input")
        })
        .expect("user-input provenance target");
    assert_eq!(provenance["kind"], "other");
    assert_eq!(provenance["expected"]["property"], "origin_trusted");
    assert!(provenance["expected"]["detail"]
        .as_str()
        .unwrap()
        .contains("origin at use site"));

    check_against_schema(&doc, &contract_schema());
}

#[test]
fn symmetry_uses_enclosing_function_bounds_and_warns_on_name_disagreement() {
    let output = fixture_run("symmetry", &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc = json(&output);
    let symmetry = target(&doc, "symmetry");
    assert_eq!(symmetry["kind"], "contract");
    assert_eq!(symmetry["dependency_hint"]["counterpart"], "deserialize_x");
    // Symmetry anchors on the file's first diff line (inside `fetch`) while its
    // finding names `serialize_x`; v5 requires the real enclosing bounds.
    assert_eq!(symmetry["site"]["symbol"], "fetch");
    let line = symmetry["site"]["line"].as_u64().unwrap();
    assert!(symmetry["site"]["function_start_line"].as_u64().unwrap() <= line);
    assert!(line <= symmetry["site"]["function_end_line"].as_u64().unwrap());
    assert!(doc["warnings"].as_array().unwrap().iter().any(|warning| {
        warning
            .as_str()
            .unwrap()
            .contains("symbol fetch differs from finding's function serialize_x")
    }));
    check_against_schema(&doc, &contract_schema());
}

#[test]
fn ids_are_deterministic_and_filters_apply() {
    let first = fixture_run(DEFAULT_ALGORITHMS, &[]);
    let second = fixture_run(DEFAULT_ALGORITHMS, &[]);
    assert_eq!(
        first.stdout, second.stdout,
        "same input must be byte deterministic"
    );
    let first_doc = json(&first);
    check_against_schema(&first_doc, &contract_schema());
    for target in targets(&first_doc) {
        let id = target["id"].as_str().unwrap();
        assert_eq!(id.len(), 64);
        assert!(id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    }

    let asserted = json(&fixture_run(
        DEFAULT_ALGORITHMS,
        &["--min-tier", "asserted"],
    ));
    assert!(targets(&asserted)
        .iter()
        .any(|value| value["source_algorithm"] == "absence"));
    assert!(targets(&asserted)
        .iter()
        .all(|value| value["tier"] == "asserted"));
    check_against_schema(&asserted, &contract_schema());

    let concern = json(&fixture_run(
        DEFAULT_ALGORITHMS,
        &["--min-severity", "concern"],
    ));
    assert!(targets(&concern)
        .iter()
        .all(|value| value["source_algorithm"] != "absence"));
    check_against_schema(&concern, &contract_schema());
}

#[test]
fn acceptance_table_and_strict_without_errors_exit_codes() {
    let cases = [
        ("chop", "requires --chop-source"),
        ("delta", "requires --old-repo"),
        ("leftflow", "produces slice blocks, not findings"),
    ];
    for (algorithm, message) in cases {
        let output = fixture_run(algorithm, &[]);
        assert_eq!(output.status.code(), Some(1), "{algorithm}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{algorithm}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let angle = fixture_run("angle", &[]);
    assert!(angle.status.success());
    let angle_doc = json(&angle);
    assert!(targets(&angle_doc).is_empty());
    check_against_schema(&angle_doc, &contract_schema());
    assert!(String::from_utf8_lossy(&angle.stderr)
        .contains("targets: angle produces no findings at this version"));

    // No accepted targets producer at this base has a controllable runtime Err
    // path. The binary-unit test pins the non-empty-errors -> 3 decision; this
    // live case pins that strict does not fail a complete run.
    let strict = fixture_run("absence", &["--strict"]);
    assert_eq!(strict.status.code(), Some(0));
    let strict_doc = json(&strict);
    assert!(strict_doc.get("errors").is_none());
    check_against_schema(&strict_doc, &contract_schema());

    let invalid_format = fixture_run("absence", &["--format", "yaml"]);
    assert_eq!(invalid_format.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid_format.stderr).contains("invalid value"));
}

#[test]
fn out_file_matches_stdout_document_bytes() {
    let expected = fixture_run("absence", &[]);
    assert!(expected.status.success());
    check_against_schema(&json(&expected), &contract_schema());
    let temp = TempDir::new().unwrap();
    let out_path = temp.path().join("targets.json");
    let actual = fixture_run("absence", &["--out", out_path.to_str().unwrap()]);
    assert!(
        actual.status.success(),
        "{}",
        String::from_utf8_lossy(&actual.stderr)
    );
    assert!(actual.stdout.is_empty());
    assert_eq!(fs::read(out_path).unwrap(), expected.stdout);
}

#[test]
fn load_warnings_fatal_reads_and_escaping_paths_are_visible() {
    let unsupported = run(Path::new(REPO), Path::new(DIFF_UNSUPPORTED), "absence", &[]);
    assert!(unsupported.status.success());
    let unsupported_doc = json(&unsupported);
    assert!(unsupported_doc["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| {
            warning == "skipped unsupported file: notes.txt (unsupported language)"
        }));
    check_against_schema(&unsupported_doc, &contract_schema());

    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    let missing_diff = repo.join("missing.json");
    fs::write(
        &missing_diff,
        r#"{"files":[{"file_path":"missing.py","modify_type":"Modified","diff_lines":[2]}]}"#,
    )
    .unwrap();
    let missing = run(&repo, &missing_diff, "absence", &[]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("Failed to read source"));

    let outside = temp.path().join("outside.py");
    fs::write(&outside, "def read():\n    f = open(\"x\")\n    return f\n").unwrap();
    let absolute_diff = repo.join("absolute.json");
    fs::write(
        &absolute_diff,
        serde_json::to_vec(&serde_json::json!({
            "files": [{
                "file_path": outside.to_str().unwrap(),
                "modify_type": "Modified",
                "diff_lines": [2]
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let absolute = run(&repo, &absolute_diff, "absence", &[]);
    assert!(
        absolute.status.success(),
        "{}",
        String::from_utf8_lossy(&absolute.stderr)
    );
    let absolute_doc = json(&absolute);
    assert!(targets(&absolute_doc).is_empty());
    assert!(absolute_doc["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| {
            warning
                .as_str()
                .unwrap()
                .contains("dropped finding with path escaping repo root")
        }));
    check_against_schema(&absolute_doc, &contract_schema());
}
