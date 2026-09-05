//! Task 5 `--min-confidence` and `--resolution` admission cases.

use super::min_confidence_test::{call_files, command_output};
use assert_cmd::Command;
use clap::Parser;
use prism::api::{
    ReviewOptions, SarifInputs, TargetsMeta, DEFAULT_MIN_CONFIDENCE, DEFAULT_RESOLUTION,
};
use prism::cli::{Cli, Command as CliCommand};
use serde_json::Value;
use std::process::Output;

fn findings(doc: &Value) -> &[Value] {
    doc["findings"].as_array().unwrap()
}

#[test]
fn cli_and_api_defaults_use_the_shared_confidence_constants() {
    let cli = Cli::try_parse_from(["slicing", "--repo", "repo", "--diff", "diff"]).unwrap();
    assert_eq!(cli.review.min_confidence, DEFAULT_MIN_CONFIDENCE);
    assert_eq!(cli.review.resolution, DEFAULT_RESOLUTION);

    let targets_cli =
        Cli::try_parse_from(["slicing", "targets", "--repo", "repo", "--diff", "diff"]).unwrap();
    let Some(CliCommand::Targets(targets)) = targets_cli.command else {
        panic!("targets subcommand must parse as Targets");
    };
    assert_eq!(targets.min_confidence, DEFAULT_MIN_CONFIDENCE);
    assert_eq!(targets.resolution, DEFAULT_RESOLUTION);

    let options = ReviewOptions::new("repo");
    assert_eq!(options.min_confidence, DEFAULT_MIN_CONFIDENCE);
    assert_eq!(options.resolution, DEFAULT_RESOLUTION);

    let sarif = SarifInputs::new(&[]);
    assert_eq!(sarif.min_confidence, DEFAULT_MIN_CONFIDENCE);
    assert_eq!(sarif.resolution, DEFAULT_RESOLUTION);

    let targets = TargetsMeta::default();
    assert_eq!(targets.min_confidence, DEFAULT_MIN_CONFIDENCE);
    assert_eq!(targets.resolution, DEFAULT_RESOLUTION);
}

fn results(doc: &Value) -> &[Value] {
    doc["runs"][0]["results"].as_array().unwrap()
}

#[test]
fn default_json_and_review_are_byte_identical_to_base() {
    let files = call_files(true);
    for format in ["json", "review"] {
        let implicit = command_output(&files, "src/api.c", 8, "echo", Some(format), false, &[]);
        let explicit = command_output(
            &files,
            "src/api.c",
            8,
            "echo",
            Some(format),
            false,
            &["--min-confidence", "nameonly", "--resolution", "nominal"],
        );
        assert!(implicit.status.success(), "{format}: {implicit:?}");
        assert!(explicit.status.success(), "{format}: {explicit:?}");
        assert_eq!(implicit.status.code(), explicit.status.code(), "{format}");
        assert_eq!(implicit.stdout, explicit.stdout, "{format} stdout");
        assert_eq!(implicit.stderr, explicit.stderr, "{format} stderr");
        assert!(
            !findings(&serde_json::from_slice(&implicit.stdout).unwrap()).is_empty(),
            "fixture must emit a finding for {format}"
        );
    }
}

#[test]
fn min_confidence_exact_drops_the_nameonly_echo_finding() {
    let files = call_files(true);
    for format in ["json", "review"] {
        let scoped = command_output(
            &files,
            "src/api.c",
            8,
            "echo",
            Some(format),
            false,
            &["--resolution", "scoped"],
        );
        let exact = command_output(
            &files,
            "src/api.c",
            8,
            "echo",
            Some(format),
            false,
            &["--resolution", "scoped", "--min-confidence", "exact"],
        );
        assert!(scoped.status.success(), "{format}: {scoped:?}");
        assert!(exact.status.success(), "{format}: {exact:?}");
        let scoped: Value = serde_json::from_slice(&scoped.stdout).unwrap();
        let exact: Value = serde_json::from_slice(&exact.stdout).unwrap();
        assert!(
            !findings(&scoped).is_empty(),
            "fixture must be finding-bearing for {format}"
        );
        assert!(findings(&exact).is_empty(), "{format}: {exact:#}");
    }
}

#[test]
fn confidence_floor_precedes_projection_for_every_finding_bearing_format() {
    let exact_files = call_files(false);
    for format in ["json", "review"] {
        let output = command_output(
            &exact_files,
            "src/device.c",
            4,
            "echo",
            Some(format),
            false,
            &["--resolution", "nominal", "--min-confidence", "exact"],
        );
        assert!(output.status.success(), "{format}: {output:?}");
        let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            !findings(&doc).is_empty(),
            "Exact evidence must survive nominal projection for {format}: {doc:#}"
        );
    }

    let sarif = command_output(
        &exact_files,
        "src/device.c",
        4,
        "echo",
        Some("sarif"),
        false,
        &["--resolution", "nominal", "--min-confidence", "exact"],
    );
    assert!(sarif.status.success(), "{sarif:?}");
    let sarif: Value = serde_json::from_slice(&sarif.stdout).unwrap();
    assert!(!results(&sarif).is_empty(), "{sarif:#}");
    assert!(results(&sarif).iter().all(|result| {
        result["properties"]["confidence"] == "unlabeled"
            && result["properties"]["tier"] == "candidate"
    }));

    let targets = command_output(
        &exact_files,
        "src/device.c",
        4,
        "echo",
        None,
        true,
        &["--resolution", "nominal", "--min-confidence", "exact"],
    );
    assert!(targets.status.success(), "{targets:?}");
    let targets: Value = serde_json::from_slice(&targets.stdout).unwrap();
    assert!(
        !targets["targets"].as_array().unwrap().is_empty(),
        "{targets:#}"
    );
    assert!(targets["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|target| { target["confidence"] == "unlabeled" && target["tier"] == "candidate" }));

    let name_only_files = call_files(true);
    for resolution in ["nominal", "scoped"] {
        for format in ["json", "review", "sarif"] {
            let output = command_output(
                &name_only_files,
                "src/api.c",
                8,
                "echo",
                Some(format),
                false,
                &["--resolution", resolution, "--min-confidence", "exact"],
            );
            assert!(output.status.success(), "{resolution}/{format}: {output:?}");
            let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
            let retained = if format == "sarif" {
                results(&doc)
            } else {
                findings(&doc)
            };
            assert!(
                retained.is_empty(),
                "NameOnly evidence survived exact floor for {resolution}/{format}: {doc:#}"
            );
        }

        let output = command_output(
            &name_only_files,
            "src/api.c",
            8,
            "echo",
            None,
            true,
            &["--resolution", resolution, "--min-confidence", "exact"],
        );
        assert!(output.status.success(), "{resolution}/targets: {output:?}");
        let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            doc["targets"].as_array().unwrap().is_empty(),
            "NameOnly evidence survived exact floor for {resolution}/targets: {doc:#}"
        );
    }
}

#[test]
fn min_confidence_exact_filters_multi_json_and_review_results_and_aggregate() {
    let files = call_files(true);
    for format in ["json", "review"] {
        let scoped = command_output(
            &files,
            "src/api.c",
            8,
            "echo,absence",
            Some(format),
            false,
            &["--resolution", "scoped"],
        );
        let exact = command_output(
            &files,
            "src/api.c",
            8,
            "echo,absence",
            Some(format),
            false,
            &["--resolution", "scoped", "--min-confidence", "exact"],
        );
        assert!(scoped.status.success(), "{format}: {scoped:?}");
        assert!(exact.status.success(), "{format}: {exact:?}");
        let scoped: Value = serde_json::from_slice(&scoped.stdout).unwrap();
        let exact: Value = serde_json::from_slice(&exact.stdout).unwrap();
        assert!(scoped["all_findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["algorithm"] == "echo"));
        assert!(exact["all_findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["algorithm"] != "echo"));
        let echo_result = exact["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|result| result["algorithm"] == "EchoSlice")
            .unwrap();
        assert!(echo_result["findings"].as_array().unwrap().is_empty());
    }
}

#[test]
fn default_nameonly_threshold_retains_an_unlabeled_finding() {
    let bridge = "def source():\n    query = request.GET['q']\n\ndef consume():\n    cursor.execute(query)\n";
    let output = command_output(
        &[("p.py", bridge)],
        "p.py",
        5,
        "provenance",
        Some("sarif"),
        false,
        &["--resolution", "scoped"],
    );
    assert!(output.status.success(), "{output:?}");
    let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!results(&doc).is_empty(), "fixture must emit a finding");
    assert!(results(&doc)
        .iter()
        .all(|result| result["properties"]["confidence"] == "unlabeled"));
}

#[test]
fn min_confidence_exact_sarif_has_only_exact_result_confidences() {
    let files = call_files(false);
    let output = command_output(
        &files,
        "src/device.c",
        4,
        "echo",
        Some("sarif"),
        false,
        &["--resolution", "scoped", "--min-confidence", "exact"],
    );
    assert!(output.status.success(), "{output:?}");
    let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        !results(&doc).is_empty(),
        "fixture must retain an Exact finding"
    );
    assert!(results(&doc)
        .iter()
        .all(|result| result["properties"]["confidence"] == "exact"));
}

#[test]
fn resolution_nominal_sarif_reproduces_the_same_base_document() {
    let files = call_files(true);
    let implicit = command_output(&files, "src/api.c", 8, "echo", Some("sarif"), false, &[]);
    let nominal = command_output(
        &files,
        "src/api.c",
        8,
        "echo",
        Some("sarif"),
        false,
        &["--resolution", "nominal"],
    );
    assert!(implicit.status.success(), "{implicit:?}");
    assert!(nominal.status.success(), "{nominal:?}");
    assert_eq!(implicit.stdout, nominal.stdout);
    assert_eq!(implicit.stderr, nominal.stderr);
    let doc: Value = serde_json::from_slice(&nominal.stdout).unwrap();
    assert!(!results(&doc).is_empty(), "fixture must emit a finding");
    assert!(results(&doc).iter().all(|result| {
        result["properties"]["confidence"] == "unlabeled"
            && result["properties"]["tier"] == "candidate"
    }));
}

#[test]
fn resolution_scoped_sarif_reports_evidence_labels_and_mode_scoped() {
    let files = call_files(true);
    let output = command_output(
        &files,
        "src/api.c",
        8,
        "echo",
        Some("sarif"),
        false,
        &["--resolution", "scoped"],
    );
    assert!(output.status.success(), "{output:?}");
    let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doc["runs"][0]["properties"]["resolution_mode"], "scoped");
    assert!(!results(&doc).is_empty(), "fixture must emit a finding");
    assert!(results(&doc).iter().all(|result| {
        result["properties"]["resolution_mode"] == "scoped"
            && matches!(
                result["properties"]["confidence"].as_str(),
                Some("exact" | "nameonly")
            )
    }));
    assert!(results(&doc)
        .iter()
        .any(|result| result["properties"]["confidence"] == "nameonly"));
}

#[test]
fn targets_scoped_grades_filter_and_nominal_projection_agree_with_sarif() {
    for (name_only, confidence) in [(false, "exact"), (true, "nameonly")] {
        let files = call_files(name_only);
        let (diff_file, diff_line) = if name_only {
            ("src/api.c", 8)
        } else {
            ("src/device.c", 4)
        };
        let output = command_output(
            &files,
            diff_file,
            diff_line,
            "echo",
            None,
            true,
            &["--resolution", "scoped"],
        );
        assert!(output.status.success(), "{output:?}");
        let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(doc["producer"]["resolution_mode"], "scoped");
        assert!(!doc["targets"].as_array().unwrap().is_empty());
        assert!(doc["targets"].as_array().unwrap().iter().all(|target| {
            target["confidence"] == confidence
                && target["tier"]
                    == if confidence == "exact" {
                        "asserted"
                    } else {
                        "candidate"
                    }
        }));
    }

    let files = call_files(true);
    let exact = command_output(
        &files,
        "src/api.c",
        8,
        "echo",
        None,
        true,
        &["--resolution", "scoped", "--min-confidence", "exact"],
    );
    assert!(exact.status.success(), "{exact:?}");
    let exact: Value = serde_json::from_slice(&exact.stdout).unwrap();
    assert!(exact["targets"].as_array().unwrap().is_empty(), "{exact:#}");

    let implicit = command_output(&files, "src/api.c", 8, "echo", None, true, &[]);
    let nominal = command_output(
        &files,
        "src/api.c",
        8,
        "echo",
        None,
        true,
        &["--resolution", "nominal"],
    );
    assert!(implicit.status.success(), "{implicit:?}");
    assert!(nominal.status.success(), "{nominal:?}");
    let mut implicit: Value = serde_json::from_slice(&implicit.stdout).unwrap();
    let mut nominal: Value = serde_json::from_slice(&nominal.stdout).unwrap();
    implicit["repo"]["root"] = Value::Null;
    nominal["repo"]["root"] = Value::Null;
    assert_eq!(implicit, nominal);
    assert_eq!(nominal["producer"]["resolution_mode"], "nominal");
    assert!(nominal["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|target| { target["confidence"] == "unlabeled" && target["tier"] == "candidate" }));
}

fn parse_failure(args: &[&str]) -> Output {
    let mut command = Command::cargo_bin("prism").unwrap();
    command.args(args).output().unwrap()
}

#[test]
fn a_bogus_confidence_value_exits_two_and_lists_the_possible_values() {
    let output = parse_failure(&["--min-confidence", "bogus"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value 'bogus'"), "{stderr}");
    assert!(stderr.contains("exact"), "{stderr}");
    assert!(stderr.contains("nameonly"), "{stderr}");
}

#[test]
fn resolution_precise_and_auto_are_rejected_and_name_roadmap_item_3() {
    for value in ["precise", "auto"] {
        let output = parse_failure(&["--resolution", value]);
        assert_eq!(output.status.code(), Some(2), "{value}: {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("invalid value '{value}'")),
            "{stderr}"
        );
        assert!(stderr.contains("nominal"), "{stderr}");
        assert!(stderr.contains("scoped"), "{stderr}");
    }
    let help = parse_failure(&["--help"]);
    assert!(help.status.success(), "{help:?}");
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("roadmap item 3"), "{stdout}");
    assert!(stdout.contains("precise"), "{stdout}");
    assert!(stdout.contains("auto"), "{stdout}");
}

#[test]
fn min_confidence_is_rejected_for_text_paper_mermaid_and_callers() {
    for format in ["text", "paper", "mermaid", "callers"] {
        let output = parse_failure(&[
            "--repo",
            "does-not-exist",
            "--diff",
            "does-not-exist",
            "--format",
            format,
            "--min-confidence",
            "exact",
        ]);
        assert_eq!(output.status.code(), Some(2), "{format}: {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(format), "{format}: {stderr}");
        assert!(
            stderr.contains("no stable finding projection"),
            "{format}: {stderr}"
        );
    }
}
