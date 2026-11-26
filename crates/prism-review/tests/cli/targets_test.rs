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

use super::schema_support::{check_against_schema, contract_schema};
use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const REPO: &str = "tests/fixtures/targets";
const DIFF: &str = "tests/fixtures/targets/diff.json";
const DIFF_UNSUPPORTED: &str = "tests/fixtures/targets/diff-with-unsupported.json";
const DEFAULT_ALGORITHMS: &str = "echo,absence,contract,provenance,membrane";

// `requests.py` is a repo-local module named `requests` (the CPG resolves
// Python imports against the analyzed tree only — it never fetches real
// packages), so `import requests` in `svc.py` binds to it precisely. That
// lets `svc.py`'s `requests.post(...)` call become a real, algorithm-produced
// echo/missing_error_handling finding whose call-site text is exactly the
// http-shaped syntax this feature targets, without faking a finding by hand.
const REPO_HINT: &str = "tests/fixtures/targets_hint";
const DIFF_HINT: &str = "tests/fixtures/targets_hint/diff.json";

fn prism_cmd() -> Command {
    crate::prism_bin().unwrap()
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

fn fixture_default_run(extra: &[&str]) -> std::process::Output {
    let mut cmd = prism_cmd();
    cmd.arg("targets")
        .arg("--repo")
        .arg(REPO)
        .arg("--diff")
        .arg(DIFF);
    cmd.args(extra).output().unwrap()
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

#[test]
fn omitted_algorithm_uses_all_five_defaults_in_order() {
    let output = fixture_default_run(&[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        json(&output)["producer"]["algorithms"],
        serde_json::json!([
            "EchoSlice",
            "AbsenceSlice",
            "ContractSlice",
            "ProvenanceSlice",
            "MembraneSlice"
        ])
    );
}

#[test]
fn today_no_external_call_rows_for_dependency_calls() {
    use crate::targets_inventory_matrix_test::FIXTURE_REPO;

    let fixture = "http_root/h01.py";
    let source_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(FIXTURE_REPO)
        .join(fixture);
    let source = fs::read_to_string(source_path).unwrap();
    let diff_lines: Vec<_> = (1..=source.lines().count()).collect();
    let dir = TempDir::new().unwrap();
    let diff_path = dir.path().join("diff.json");
    let diff = serde_json::json!({
        "files": [{
            "file_path": fixture,
            "modify_type": "Modified",
            "diff_lines": diff_lines,
        }]
    });
    fs::write(&diff_path, diff.to_string()).unwrap();

    let output = prism_cmd()
        .args(["targets", "--repo", FIXTURE_REPO, "--diff"])
        .arg(&diff_path)
        .assert()
        .success();
    let doc = json(output.get_output());
    assert_eq!(doc["diff"]["files"], serde_json::json!([fixture]));
    assert!(
        targets(&doc)
            .iter()
            .all(|target| target["kind"] != "external_call"),
        "unexpected external_call target: {doc:#}"
    );
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

    check_against_schema(&doc, &contract_schema("targets.schema.json"));
}

/// Roadmap `03-tooling-plan-roadmap.md` §3 Phase 1 / controller ruling
/// 2026-09-04: an `external_call` target's `dependency_hint` must carry the
/// call-site syntax (`requests.post`, not the call graph's resolved function
/// name `post`) — before this feature the hint was `{"callee": "post"}`,
/// accurate to the call graph and useless as a glob target.
///
/// This fixture is deliberately the NEGATIVE classification case: its
/// `requests` is the repository's own `requests.py`, which is exactly what
/// makes the `echo` finding real (the CPG resolves Python imports only against
/// the analyzed tree). Identical spelling, different dependency. `kind` must
/// therefore be absent — the harness then selects by callee glob with library
/// precedence (`~/code/tools/specs/2026-09-04-runtime-harness-v0-spec.md`
/// §5.2 steps 2–4), which is strictly better than a wrong `http` restriction.
#[test]
fn echo_external_call_gains_ast_recovered_callee_but_not_a_shadowed_kind() {
    let output = run(Path::new(REPO_HINT), Path::new(DIFF_HINT), "echo", &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc = json(&output);
    let echo = target(&doc, "echo");
    assert_eq!(echo["kind"], "external_call");
    assert_eq!(echo["dependency_hint"]["callee"], "requests.post");
    assert!(
        echo["dependency_hint"].get("kind").is_none(),
        "a repo-local requests.py is not the requests library: {}",
        echo["dependency_hint"]
    );
    check_against_schema(&doc, &contract_schema("targets.schema.json"));
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
    check_against_schema(&doc, &contract_schema("targets.schema.json"));
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
    check_against_schema(&first_doc, &contract_schema("targets.schema.json"));
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
    check_against_schema(&asserted, &contract_schema("targets.schema.json"));

    let concern = json(&fixture_run(
        DEFAULT_ALGORITHMS,
        &["--min-severity", "concern"],
    ));
    assert!(targets(&concern)
        .iter()
        .all(|value| value["source_algorithm"] != "absence"));
    check_against_schema(&concern, &contract_schema("targets.schema.json"));
}

#[test]
fn acceptance_table_and_strict_without_errors_exit_codes() {
    let cases = [
        ("chop", "requires --chop-source"),
        ("delta", "requires --old-repo"),
        ("leftflow", "produces slice blocks, not findings"),
        ("review", "produces slice blocks, not findings"),
        ("all", "produces slice blocks, not findings"),
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
    check_against_schema(&angle_doc, &contract_schema("targets.schema.json"));
    assert!(String::from_utf8_lossy(&angle.stderr)
        .contains("targets: angle produces no findings at this version"));

    // No accepted targets producer at this base has a controllable runtime Err
    // path. The binary-unit test pins the non-empty-errors -> 3 decision; this
    // live case pins that strict does not fail a complete run.
    // The live recorded-error -> exit 3 observable remains a known coverage gap.
    let strict = fixture_run("absence", &["--strict"]);
    assert_eq!(strict.status.code(), Some(0));
    let strict_doc = json(&strict);
    assert!(strict_doc.get("errors").is_none());
    check_against_schema(&strict_doc, &contract_schema("targets.schema.json"));

    let invalid_format = fixture_run("absence", &["--format", "yaml"]);
    assert_eq!(invalid_format.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid_format.stderr).contains("invalid value"));
}

#[test]
fn out_file_matches_stdout_document_bytes() {
    let expected = fixture_run("absence", &[]);
    assert!(expected.status.success());
    check_against_schema(&json(&expected), &contract_schema("targets.schema.json"));
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
    check_against_schema(&unsupported_doc, &contract_schema("targets.schema.json"));

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
    check_against_schema(&absolute_doc, &contract_schema("targets.schema.json"));
}
