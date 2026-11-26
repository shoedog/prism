//! Acceptance controls for the opt-in Python dependency inventory.
use super::schema_support::{check_against_schema, contract_schema};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub(super) fn run(repo: &Path, args: &[&str]) -> std::process::Output {
    crate::prism_std()
        .args(["targets", "--repo"])
        .arg(repo)
        .args(["--inventory", "python-deps"])
        .args(args)
        .output()
        .unwrap()
}

pub(super) fn document(output: &std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn fixture() -> TempDir {
    let repo = TempDir::new().unwrap();
    fs::create_dir(repo.path().join("pkg")).unwrap();
    fs::write(
        repo.path().join("pkg/x.py"),
        "import requests\n\ndef fetch(u):\n    return requests.get(u)\n",
    )
    .unwrap();
    repo
}

#[test]
fn vendored_1_1_schema_matches_its_recorded_hash() {
    let bytes = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/contracts/targets-1.1.schema.json"
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "a760e0125de8c924d22d9c85e32a2d312a524481134405b28cdb3ab222de708b"
    );
}

#[test]
fn inventory_only_run_needs_no_diff_and_writes_schema_1_1() {
    let repo = fixture();
    let doc = document(&run(repo.path(), &[]));
    check_against_schema(&doc, &contract_schema("targets-1.1.schema.json"));
    assert_eq!(doc["schema_version"], "1.1");
    assert_eq!(doc["producer"]["algorithms"], json!(["prism.inventory/1"]));
    assert_eq!(doc["diff"]["files"], json!(["pkg/x.py"]));
    let bytes = serde_json::to_vec(&doc["diff"]["files"]).unwrap();
    assert_eq!(
        doc["diff"]["sha256"],
        format!("{:x}", Sha256::digest(bytes))
    );
}

#[test]
fn inventory_row_has_the_documented_shape() {
    let repo = fixture();
    let doc = document(&run(repo.path(), &[]));
    let row = &doc["targets"][0];
    assert_eq!(
        row["site"],
        json!({"file":"pkg/x.py", "line":4, "symbol":"fetch", "function_start_line":3, "function_end_line":4, "language":"python"})
    );
    assert_eq!(row["kind"], "external_call");
    assert_eq!(row["category"], "dependency_call.http");
    assert_eq!(
        row["dependency_hint"],
        json!({"kind":"http", "callee":"requests.get"})
    );
    assert_eq!(
        row["expected"],
        json!({"property":"error_handled", "detail":"source callee: requests.get"})
    );
    assert_eq!(row["confidence"], "exact");
    assert_eq!(row["tier"], "asserted");
    assert_eq!(row["severity"], "info");
}

#[test]
fn unsupported_site_is_an_other_row_with_a_reason() {
    let repo = fixture();
    fs::write(
        repo.path().join("pkg/x.py"),
        "import requests\nrequests.get(u)\n",
    )
    .unwrap();
    let doc = document(&run(repo.path(), &[]));
    let row = &doc["targets"][0];
    assert_eq!(row["kind"], "other");
    assert_eq!(row["expected"]["detail"], "module_level");
    assert_eq!(
        row["description"],
        "requests.get: unsupported dependency call: module_level"
    );
    assert_eq!(row["dependency_hint"]["callee"], "requests.get");
    assert!(row["site"].get("function_start_line").is_none());
}

#[test]
fn inventory_without_diff_rejects_review_runner_options() {
    let repo = fixture();
    for args in [
        &["--algorithm", "echo"][..],
        &["--algorithm=echo,absence,contract,provenance,membrane"],
        &["--scoped-cpg"],
        &["--old-repo", "."],
        &["--compile-commands", "x"],
        &["--cache-dir", "x"],
        &["--no-cache"],
    ] {
        let output = run(repo.path(), args);
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(args[0].split('=').next().unwrap())
        );
    }
}

#[test]
fn inventory_kinds_rejects_an_unsupported_family() {
    let repo = fixture();
    let output = run(repo.path(), &["--inventory-kinds", "filesystem"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("filesystem") && stderr.contains("http"),
        "{stderr}"
    );
}

#[test]
fn inventory_ids_are_stable_across_runs_and_thread_counts() {
    let repo = fixture();
    let execute = |threads: &str| {
        crate::prism_std()
            .args(["targets", "--repo"])
            .arg(repo.path())
            .args(["--inventory", "python-deps"])
            .env("RAYON_NUM_THREADS", threads)
            .output()
            .unwrap()
    };
    let one = execute("1");
    let four = execute("4");
    assert!(one.status.success() && four.status.success());
    assert_eq!(one.stdout, four.stdout);
}

#[test]
fn inventory_skips_venv_and_site_packages_directories() {
    let repo = fixture();
    for path in ["venv/lib", "lib/site-packages"] {
        fs::create_dir_all(repo.path().join(path)).unwrap();
        fs::write(
            repo.path().join(path).join("y.py"),
            "import requests\nrequests.get(1)\n",
        )
        .unwrap();
    }
    let doc = document(&run(repo.path(), &[]));
    assert_eq!(doc["diff"]["files"], json!(["pkg/x.py"]));
    assert!(doc["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w.as_str().unwrap().contains("venv, site-packages")));
}

#[test]
fn files_filter_narrows_the_inventory() {
    let repo = fixture();
    fs::write(
        repo.path().join("pkg/y.py"),
        "import requests\nrequests.get(1)\n",
    )
    .unwrap();
    let doc = document(&run(repo.path(), &["--files", "pkg/x.py"]));
    assert_eq!(doc["diff"]["files"], json!(["pkg/x.py"]));
    assert!(doc["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["site"]["file"] == "pkg/x.py"));
}

pub(super) fn matrix_doc(files: &str, args: &[&str]) -> Value {
    let mut options = vec!["--files", files];
    options.extend_from_slice(args);
    document(&run(
        Path::new(crate::targets_inventory_matrix_test::FIXTURE_REPO),
        &options,
    ))
}

fn rows(doc: &Value) -> &[Value] {
    doc["targets"].as_array().unwrap()
}

fn row(doc: &Value, file: &str, line: u64) -> Value {
    rows(doc)
        .iter()
        .find(|row| row["site"]["file"] == file && row["site"]["line"] == line)
        .unwrap_or_else(|| panic!("missing {file}:{line} in {doc:#}"))
        .clone()
}

pub(super) fn warning(doc: &Value, expected: &str) {
    let warnings = doc["warnings"].as_array().unwrap();
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.as_str() == Some(expected))
            .count(),
        1,
        "{doc:#}",
    );
}

#[test]
fn unsupported_calls_of_one_callee_on_one_line_are_one_row() {
    let file = "http_root/tr1-16.py";
    let doc = matrix_doc(file, &[]);
    assert_eq!(rows(&doc).len(), 1);
    assert_eq!(
        row(&doc, file, 3)["expected"]["detail"],
        "not_line_anchor; 2 calls"
    );
}

#[test]
fn identity_unproven_calls_are_counted_per_file_and_cause() {
    let doc = matrix_doc(
        "http_root/n04.py,http_root/n05/client.py,http_root/sl1-67.py,http_root/as4-03.py",
        &[],
    );
    for detail in [
        "http_root/n04.py: 1 (shadowed_root: 1)",
        "http_root/n05/client.py: 1 (repository_local_module: 1)",
        "http_root/sl1-67.py: 3 (shadowed_root: 2, not_imported: 1)",
        "http_root/as4-03.py: 2 (shadowed_root: 1, not_imported: 1)",
    ] {
        warning(&doc, &format!("inventory: identity unproven: {detail}"));
    }
}

#[test]
fn inventory_only_document_carries_repo_sha_in_a_git_checkout() {
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(sha.status.success());
    let git_dir = std::process::Command::new("git")
        .args(["rev-parse", "--absolute-git-dir"])
        .output()
        .unwrap();
    assert!(git_dir.status.success());
    let repo = fixture();
    fs::write(
        repo.path().join(".git"),
        format!("gitdir: {}", String::from_utf8(git_dir.stdout).unwrap()),
    )
    .unwrap();
    let doc = document(&run(repo.path(), &[]));
    assert_eq!(
        doc["repo"]["sha"],
        String::from_utf8(sha.stdout).unwrap().trim()
    );
}

fn review_fixture(repo: &Path, diff: &Path, extra: &[&str]) -> Value {
    let output = crate::prism_std()
        .args(["targets", "--repo"])
        .arg(repo)
        .arg("--diff")
        .arg(diff)
        .args(extra)
        .output()
        .unwrap();
    document(&output)
}

#[test]
fn without_inventory_the_document_is_schema_1_0() {
    let repo = Path::new("tests/fixtures/targets");
    let doc = review_fixture(repo, &repo.join("diff.json"), &[]);
    check_against_schema(&doc, &contract_schema("targets.schema.json"));
    assert_eq!(doc["schema_version"], "1.0");
    assert!(rows(&doc)
        .iter()
        .all(|row| row["source_algorithm"] != "prism.inventory/1"));
}

fn copy_review_files(repo: &Path) {
    for file in ["client.py", "svc.py"] {
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/targets")
                .join(file),
            repo.join(file),
        )
        .unwrap();
    }
}

#[test]
fn inventory_merges_with_projected_findings() {
    let repo = fixture();
    copy_review_files(repo.path());
    let diff = Path::new("tests/fixtures/targets/diff.json");
    let legacy = review_fixture(repo.path(), diff, &[]);
    let merged = review_fixture(repo.path(), diff, &["--inventory", "python-deps"]);
    check_against_schema(&merged, &contract_schema("targets-1.1.schema.json"));
    assert_eq!(merged["diff"], legacy["diff"]);
    assert_eq!(merged["errors"], legacy["errors"]);
    let mut producers = legacy["producer"]["algorithms"].as_array().unwrap().clone();
    producers.push(json!("prism.inventory/1"));
    assert_eq!(merged["producer"]["algorithms"], json!(producers));
    assert!(rows(&merged)
        .iter()
        .any(|row| row["source_algorithm"] == "prism.inventory/1"));
    let projected: Vec<_> = rows(&merged)
        .iter()
        .filter(|row| row["source_algorithm"] != "prism.inventory/1")
        .cloned()
        .collect();
    assert!(!projected.is_empty());
    assert_eq!(projected, rows(&legacy));
    let key = |row: &Value| {
        (
            row["site"]["file"].as_str().unwrap().to_owned(),
            row["site"]["line"].as_u64().unwrap(),
            row["source_algorithm"].as_str().unwrap().to_owned(),
            row["category"].as_str().unwrap().to_owned(),
            row["id"].as_str().unwrap().to_owned(),
        )
    };
    assert!(rows(&merged)
        .windows(2)
        .all(|pair| key(&pair[0]) < key(&pair[1])));
    assert_eq!(
        rows(&merged)
            .iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        rows(&merged).len()
    );
}

#[test]
fn inventory_respects_confidence_and_severity_filters() {
    let files = "http_root/h01.py,http_root/o23.py,http_root/u01.py,http_root/n04.py";
    let doc = matrix_doc(files, &["--min-confidence", "exact"]);
    assert_eq!(rows(&doc).len(), 4);
    let doc = matrix_doc(files, &["--min-tier", "asserted"]);
    assert_eq!(rows(&doc).len(), 2);
    let doc = matrix_doc(files, &["--min-severity", "warning"]);
    assert!(rows(&doc).is_empty());
    warning(&doc, "inventory: 4 rows removed by --min-severity warning");
    warning(
        &doc,
        "inventory: identity unproven: http_root/n04.py: 1 (shadowed_root: 1)",
    );
}

#[test]
fn review_focus_1_parse_errors_in_owning_definition() {
    let doc = matrix_doc("http_root/o23.py,http_root/sl1-24.py,http_root/sl2-14.py,http_root/as4-08.py,http_root/as1-10.py", &[]);
    for (file, line) in [("o23", 5), ("sl1-24", 4), ("as4-08", 7)] {
        let target = row(&doc, &format!("http_root/{file}.py"), line);
        assert_eq!(target["confidence"], "exact");
        assert_eq!(target["tier"], "candidate");
        assert!(matches!(
            target["parse_quality"].as_str(),
            Some("poor" | "degraded")
        ));
    }
    for (file, line) in [
        ("o23", 9),
        ("sl1-24", 6),
        ("sl2-14", 3),
        ("as4-08", 3),
        ("as1-10", 3),
    ] {
        let target = row(&doc, &format!("http_root/{file}.py"), line);
        assert!(target["expected"]["detail"]
            .as_str()
            .unwrap()
            .starts_with("parse_error_in_function"));
        assert_eq!(target["dependency_hint"]["kind"], "http");
        assert_eq!(target["tier"], "candidate");
    }
}

#[test]
fn review_focus_2_decorated_function_bounds() {
    let repo = fixture();
    fs::write(
        repo.path().join("pkg/x.py"),
        "import requests\n@decorator\ndef fetch(u):\n    return requests.get(u)\n    # trailing\n",
    )
    .unwrap();
    let doc = document(&run(repo.path(), &[]));
    assert_eq!(rows(&doc)[0]["site"]["function_start_line"], 3);
    assert_eq!(rows(&doc)[0]["site"]["function_end_line"], 4);
}

#[test]
fn review_focus_4_normalised_paths() {
    let repo = fixture();
    let canonical = document(&run(repo.path(), &["--files", "pkg/x.py"]));
    let alias = document(&run(repo.path(), &["--files", "./pkg\\x.py"]));
    assert_eq!(canonical, alias);
    assert_eq!(rows(&alias)[0]["site"]["file"], "pkg/x.py");

    copy_review_files(repo.path());
    let path = repo.path().join("client.py");
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str("\nimport httpx\ndef extra(u):\n    return httpx.get(u)\n");
    fs::write(path, source).unwrap();
    let diff = Path::new("tests/fixtures/targets/diff.json");
    let merged = review_fixture(repo.path(), diff, &["--inventory", "python-deps"]);
    let finding = rows(&merged)
        .iter()
        .find(|r| r["site"]["file"] == "client.py" && r["source_algorithm"] != "prism.inventory/1")
        .unwrap();
    let expected = rows(&merged)
        .iter()
        .find(|r| r["site"]["file"] == "client.py" && r["source_algorithm"] == "prism.inventory/1")
        .unwrap();
    for spelling in ["client.py", "./client.py", ".\\client.py"] {
        let selected = document(&run(repo.path(), &["--files", spelling]));
        assert_eq!(selected["diff"]["files"], json!(["client.py"]));
        assert_eq!(rows(&selected)[0]["site"]["file"], finding["site"]["file"]);
        assert_eq!(rows(&selected)[0]["id"], expected["id"]);
    }
}

#[test]
fn review_focus_5_same_callee_two_lines_and_files_filter() {
    let file = "http_root/o24.py";
    let doc = matrix_doc(file, &[]);
    assert_ne!(row(&doc, file, 5)["id"], row(&doc, file, 6)["id"]);
    let excluded = matrix_doc("http_root/h01.py", &[]);
    assert_eq!(excluded["diff"]["files"], json!(["http_root/h01.py"]));
    assert!(rows(&excluded)
        .iter()
        .all(|target| target["site"]["file"] != file));
}

#[test]
fn inventory_normalizes_newlines_without_rewriting_source_detail() {
    let repo = fixture();
    for newline in ["\r", "\r\n"] {
        fs::write(repo.path().join("pkg/x.py"), format!("import requests{newline}def fetch(u):{newline}    return (requests).get(u){newline}")).unwrap();
        let doc = document(&run(repo.path(), &[]));
        let row = row(&doc, "pkg/x.py", 3);
        assert_eq!(row["site"]["function_end_line"], 3);
        assert_eq!(row["expected"]["detail"], "source callee: (requests).get");
    }
}

#[test]
fn targets_without_diff_or_inventory_retains_missing_diff_diagnostic() {
    let output = crate::prism_std()
        .args(["targets", "--repo", "."])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        concat!(
            "error: the following required arguments were not provided:\n",
            "  --diff <DIFF>\n\n",
            "Usage: prism targets --repo <REPO> --diff <DIFF>\n\n",
            "For more information, try '--help'.\n",
        )
    );
}

#[cfg(unix)]
#[test]
fn inventory_refuses_a_literal_backslash_in_a_filename() {
    let repo = TempDir::new().unwrap();
    fs::create_dir(repo.path().join("pkg")).unwrap();
    for (name, symbol) in [(r"pkg\fetch.py", "wrong"), ("pkg/fetch.py", "right")] {
        fs::write(
            repo.path().join(name),
            format!("import requests\ndef {symbol}(u): return requests.get(u)\n"),
        )
        .unwrap();
    }
    let doc = document(&run(repo.path(), &[]));
    assert_eq!(doc["diff"]["files"], json!(["pkg/fetch.py"]));
    assert_eq!(rows(&doc).len(), 1);
    assert_eq!(row(&doc, "pkg/fetch.py", 2)["site"]["symbol"], "right");
    assert_eq!(
        doc["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|warning| {
                warning
                    .as_str()
                    .unwrap()
                    .starts_with(r"inventory: skipped: pkg\fetch.py:")
            })
            .count(),
        1,
        "{doc:#}"
    );
    let filtered = document(&run(repo.path(), &["--files", r".\pkg\fetch.py"]));
    assert_eq!(filtered["diff"]["files"], json!(["pkg/fetch.py"]));
    assert_eq!(row(&filtered, "pkg/fetch.py", 2)["site"]["symbol"], "right");
}

#[test]
fn inventory_without_diff_rejects_taint_context_options() {
    let repo = fixture();
    let accepted: Vec<_> = [
        ("--taint-context", Some("imports")),
        ("--taint-context-edges", Some("exact")),
        ("--taint-context-depth", Some("2")),
        ("--taint-context-max-files", Some("4")),
    ]
    .into_iter()
    .filter_map(|(option, value)| {
        let args = [Some(option), value]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let output = run(repo.path(), &args);
        (output.status.success()
            || !String::from_utf8_lossy(&output.stderr)
                .contains(&format!("{option} requires --diff")))
        .then(|| (option, String::from_utf8_lossy(&output.stderr).into_owned()))
    })
    .collect();
    assert!(accepted.is_empty(), "{accepted:?}");
}

#[test]
fn targets_help_identifies_both_output_schemas() {
    let output = crate::prism_std()
        .args(["targets", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("schema 1.0") && help.contains("schema 1.1"));
}
