use assert_cmd::Command;
use prism::api::*;
use prism::slice::{SliceConfig, SlicingAlgorithm};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_repo(
    files: &[(&str, &str)],
    diff_files: &[(&str, &[usize])],
) -> (TempDir, PathBuf, String) {
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
    let diff_text = serde_json::to_string_pretty(&serde_json::json!({
        "files": files_json,
    }))
    .unwrap();

    (tmp, repo, diff_text)
}

fn absence_fixture(include_notes: bool) -> (TempDir, PathBuf, String) {
    let files = &[("a.py", "def read():\n    f = open(\"x\")\n    return f\n")];
    if include_notes {
        write_repo(files, &[("a.py", &[2]), ("notes.txt", &[1])])
    } else {
        write_repo(files, &[("a.py", &[2])])
    }
}

fn echo_fixture() -> (TempDir, PathBuf, String) {
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

fn degraded_absence_fixture() -> (TempDir, PathBuf, String) {
    write_repo(
        &[(
            "a.py",
            "def read():\n    f = open(\"x\")\n    return f\n\nbroken =\n",
        )],
        &[("a.py", &[2])],
    )
}

#[test]
fn one_shot_review_returns_inputs_and_findings() {
    let (_tmp, repo, diff_text) = absence_fixture(false);
    let outcome = review(
        &ReviewOptions::new(&repo),
        &diff_text,
        &[SlicingAlgorithm::AbsenceSlice],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
    )
    .unwrap();

    assert!(outcome
        .run
        .findings
        .iter()
        .any(|finding| finding.category.as_deref() == Some("missing_counterpart")));
    assert_eq!(outcome.inputs.diff.files.len(), 1);
    assert!(!outcome.inputs.parse_quality.contains_key("a.py"));
    assert!(outcome.inputs.files.contains_key("a.py"));

    let (_tmp, repo, diff_text) = absence_fixture(true);
    let with_notes = review(
        &ReviewOptions::new(&repo),
        &diff_text,
        &[SlicingAlgorithm::AbsenceSlice],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
    )
    .unwrap();
    assert_eq!(
        with_notes.inputs.load_warnings,
        ["skipped unsupported file: notes.txt (unsupported language)"]
    );
    assert_eq!(with_notes.run.warnings, with_notes.inputs.parse_warnings);
}

#[test]
fn two_phase_api_installs_its_own_pool_and_reports_each_algorithm() {
    let (_tmp, repo, diff_text) = echo_fixture();
    let opts = ReviewOptions::new(&repo);
    let inputs = load_review_inputs(&opts, &diff_text).unwrap();
    let built = build_context(&inputs, &opts).unwrap();
    let run = run_review(
        &built.ctx,
        &inputs,
        &[SlicingAlgorithm::EchoSlice, SlicingAlgorithm::AbsenceSlice],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        &repo,
    );

    assert_eq!(run.algorithms_run, ["EchoSlice", "AbsenceSlice"]);
    assert_eq!(run.results.len(), 2);
    assert_eq!(run.results[0].algorithm, SlicingAlgorithm::EchoSlice);
    assert_eq!(run.results[1].algorithm, SlicingAlgorithm::AbsenceSlice);
    assert!(run.errors.is_empty());
    assert_eq!(run.warnings, inputs.parse_warnings);
    assert!(built.warnings.is_empty());
}

#[test]
fn multi_run_keeps_raw_result_findings_and_annotates_flattened_findings() {
    let (_tmp, repo, diff_text) = degraded_absence_fixture();
    let opts = ReviewOptions::new(&repo);
    let inputs = load_review_inputs(&opts, &diff_text).unwrap();
    assert_eq!(inputs.parse_quality["a.py"].quality, "degraded");
    let built = build_context(&inputs, &opts).unwrap();
    let run = run_review(
        &built.ctx,
        &inputs,
        &[SlicingAlgorithm::AbsenceSlice],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        &repo,
    );

    assert!(!run.results[0].findings.is_empty());
    assert!(run.results[0]
        .findings
        .iter()
        .all(|finding| finding.parse_quality.is_none()));
    assert!(!run.findings.is_empty());
    assert!(run
        .findings
        .iter()
        .all(|finding| finding.parse_quality.as_deref() == Some("degraded")));
}

#[test]
fn non_fatal_build_conditions_are_returned_as_build_warnings() {
    let (_tmp, repo, diff_text) = absence_fixture(false);
    let cache_file = repo.join("not-a-cache-directory");
    fs::write(&cache_file, "occupied").unwrap();

    let mut opts = ReviewOptions::new(&repo);
    opts.compile_commands = Some(repo.join("missing-compile-commands.json"));
    opts.cache_dir = Some(cache_file);
    let inputs = load_review_inputs(&opts, &diff_text).unwrap();
    let built = build_context(&inputs, &opts).unwrap();

    assert!(built
        .warnings
        .iter()
        .any(|warning| warning.starts_with("Warning: failed to load type database:")));
    assert!(built
        .warnings
        .iter()
        .any(|warning| warning.starts_with("Warning: failed to write CPG cache:")));
}

#[test]
fn results_are_the_successful_subsequence() {
    let (_tmp, repo, diff_text) = absence_fixture(false);
    let opts = ReviewOptions::new(&repo);
    let inputs = load_review_inputs(&opts, &diff_text).unwrap();
    let built = build_context(&inputs, &opts).unwrap();
    let run = run_review(
        &built.ctx,
        &inputs,
        &[SlicingAlgorithm::Chop, SlicingAlgorithm::AbsenceSlice],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        &repo,
    );

    assert_eq!(run.algorithms_run, ["Chop", "AbsenceSlice"]);
    assert_eq!(run.results.len(), 1);
    assert_eq!(run.results[0].algorithm, SlicingAlgorithm::AbsenceSlice);
    assert_eq!(run.errors.len(), 1);
    assert_eq!(run.errors[0].algorithm, "Chop");
}

#[test]
fn defaults_are_shared_with_clap() {
    let params = AlgorithmParams::default();
    assert_eq!(params.barrier_depth, DEFAULT_BARRIER_DEPTH);
    assert_eq!(params.spiral_max_ring, DEFAULT_SPIRAL_MAX_RING);
    assert_eq!(params.temporal_days, DEFAULT_TEMPORAL_DAYS);

    let output = Command::cargo_bin("prism")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    let start = help.find("--barrier-depth").unwrap();
    let end = help[start..]
        .find("--barrier-symbols")
        .map(|offset| start + offset)
        .unwrap_or(help.len());
    assert!(
        help[start..end].contains(&format!("[default: {}]", DEFAULT_BARRIER_DEPTH)),
        "barrier-depth help block did not contain the shared default: {}",
        &help[start..end]
    );
}

#[test]
fn chop_without_params_errors_like_the_cli() {
    let (_tmp, repo, diff_text) = absence_fixture(false);
    let opts = ReviewOptions::new(&repo);
    let inputs = load_review_inputs(&opts, &diff_text).unwrap();
    let built = build_context(&inputs, &opts).unwrap();
    let error = run_algorithm(
        SlicingAlgorithm::Chop,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        &repo,
    )
    .unwrap_err();
    assert!(error.to_string().contains("--chop-source required"));
}

#[test]
fn build_info_is_this_binary() {
    let info = build_info();
    assert_eq!(info.package_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(info.build_identity.len(), 64);
}

#[test]
fn nav_session_and_callers_work_without_outer_pool() {
    let (tmp, repo, _diff_text) = write_repo(
        &[
            ("helper.py", "def helper():\n    return 1\n"),
            (
                "caller.py",
                "from helper import helper\n\ndef caller():\n    return helper()\n",
            ),
        ],
        &[],
    );
    let _keep_alive = tmp;
    let mut opts = NavOptions::default();
    opts.no_cache = true;
    let session = nav_session(&repo, &opts).unwrap();
    let evidence = callers(&session, Seed::Symbol("helper"), 1, false).unwrap();
    let json = serde_json::to_string(&evidence).unwrap();
    assert!(json.contains("caller"), "callers evidence was: {json}");
}

#[test]
fn api_types_are_non_exhaustive() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths: Vec<PathBuf> = fs::read_dir(root.join("src/api"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .collect();
    paths.push(root.join("src/finding_confidence.rs"));

    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if !(trimmed.starts_with("pub struct ") || trimmed.starts_with("pub enum ")) {
                continue;
            }
            let attributes: Vec<&str> = lines[..index]
                .iter()
                .rev()
                .map(|line| line.trim())
                .take_while(|line| line.is_empty() || line.starts_with("#["))
                .filter(|line| !line.is_empty())
                .collect();
            assert!(
                attributes.contains(&"#[non_exhaustive]"),
                "{}:{} `{}` lacks a nearest preceding #[non_exhaustive] attribute; saw {:?}",
                path.display(),
                index + 1,
                trimmed,
                attributes
            );
        }
    }
}
