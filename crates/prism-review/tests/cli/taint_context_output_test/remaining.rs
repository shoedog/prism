use super::{
    fixtures::{notifications, row, Fixture},
    presentation,
};
use prism::{
    data_flow::VarAccessKind,
    finding_confidence::{
        classify_with_evidence, FindingConfidence, FindingTier, MinConfidence, ParseQuality,
        ResolutionMode,
    },
    slice::{MultiSliceResult, SliceConfig},
};
use prism_review::{
    algorithms::taint,
    api::*,
    output,
    targets::{self, TargetsMeta},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    process::{Command, Output},
};
use tempfile::TempDir;

fn invoke(f: &Fixture, spec: &str, format: &str, extra: &[&str]) -> Output {
    let mut cmd = crate::prism_std();
    if format == "targets" {
        cmd.arg("targets");
    } else {
        cmd.args(["--format", format]);
    }
    cmd.arg("--repo")
        .arg(f.temp.path())
        .arg("--diff")
        .arg(f.temp.path().join("diff.json"))
        .args(["--algorithm", spec, "--no-cache"]);
    if matches!(format, "json" | "review" | "sarif" | "targets") {
        cmd.args([
            "--min-confidence",
            &f.inputs.min_confidence.to_string(),
            "--resolution",
            f.inputs.resolution.as_str(),
        ]);
    }
    if let Some(path) = &f.options.compile_commands {
        cmd.arg("--compile-commands").arg(path);
    }
    let out = cmd.args(extra).output().unwrap();
    assert!(
        out.status.success(),
        "status={} stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn bytes(value: &impl serde::Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}

fn remaining_stdout(
    f: &Fixture,
    spec: &str,
    format: &str,
    d: &ReviewBoundaryDiagnostics,
) -> Vec<u8> {
    let built = build_context(&f.inputs, &f.options).unwrap();
    let algorithms = parse_algorithms(spec).unwrap();
    let run = run_review(
        &built.ctx,
        &f.inputs,
        &algorithms,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        f.temp.path(),
    );
    assert_eq!(run.diagnostics.as_ref(), Some(d));
    assert!(run.errors.is_empty() && run.results.iter().all(|r| r.diagram_warnings.is_empty()));
    let multi = algorithms.len() > 1;
    match format {
        "paper" if !multi => bytes(&output::to_paper_format(&run.results[0].blocks)),
        "text" | "paper" => run
            .results
            .iter()
            .map(|r| {
                let body = output::format_slice_result(&r.blocks, &f.inputs.sources);
                if multi {
                    format!("=== {} ===\n{body}", r.algorithm.name())
                } else {
                    body
                }
            })
            .collect::<String>()
            .into_bytes(),
        "mermaid" => {
            let document = MultiSliceResult {
                version: "1.0".into(),
                algorithms_run: run.algorithms_run,
                results: run.results,
                findings: if multi { run.findings } else { vec![] },
                errors: run.errors,
                warnings: if multi {
                    f.inputs.parse_warnings.clone()
                } else {
                    vec![]
                },
                parse_quality: if multi {
                    f.inputs.parse_quality.clone()
                } else {
                    BTreeMap::new()
                },
                diagram_warnings: vec![],
            };
            format!("{}\n", output::format_mermaid_report(&document)).into_bytes()
        }
        _ => unreachable!(),
    }
}

fn target_inputs(f: &Fixture, spec: &str) -> (ReviewOutcome, TargetsMeta) {
    let root = fs::canonicalize(f.temp.path()).unwrap();
    let mut options = ReviewOptions::new(&root);
    options.no_cache = true;
    options.compile_commands = f.options.compile_commands.clone();
    options.min_confidence = f.inputs.min_confidence;
    options.resolution = f.inputs.resolution;
    let text = fs::read_to_string(f.temp.path().join("diff.json")).unwrap();
    let outcome = review(
        &options,
        &text,
        &parse_algorithms(spec).unwrap(),
        &SliceConfig::default(),
        &AlgorithmParams::default(),
    )
    .unwrap();
    let mut meta = TargetsMeta::default();
    meta.algorithms_run = outcome.run.algorithms_run.clone();
    meta.repo_root = root;
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&meta.repo_root)
        .output()
        .unwrap();
    meta.repo_sha = revision.status.success().then(|| {
        String::from_utf8(revision.stdout)
            .unwrap()
            .trim()
            .to_owned()
    });
    meta.errors = outcome.run.errors.clone();
    meta.min_confidence = outcome.min_confidence;
    meta.resolution = outcome.resolution;
    meta.run_warnings = outcome
        .inputs
        .parse_warnings
        .iter()
        .chain(&outcome.inputs.load_warnings)
        .chain(&outcome.build_warnings)
        .cloned()
        .collect();
    if let Some(d) = &outcome.run.diagnostics {
        meta.run_warnings.extend(d.messages());
    }
    (outcome, meta)
}

#[test]
fn remaining_formats_keep_stdout_and_emit_once() {
    let mut rows = Vec::new();
    for positive in [false, true] {
        for format in ["text", "paper", "mermaid"] {
            if positive && format == "text" {
                continue;
            }
            for spec in ["taint", "taint,taint"] {
                row(
                    &mut rows,
                    format!("remaining/{positive}/{format}/{spec}"),
                    || {
                        let f = Fixture::new(positive, false, false);
                        let (result, d) = f.admit_with_result(positive, false);
                        assert!(result.diagram_warnings.is_empty());
                        if positive {
                            assert!(
                                result
                                    .findings
                                    .iter()
                                    .any(|finding| !finding.diagrams.is_empty()),
                                "actual positive diagram"
                            );
                        }
                        let expected = remaining_stdout(&f, spec, format, &d);
                        let out = invoke(&f, spec, format, &["--strict-diagrams"]);
                        let stderr: String = d
                            .messages()
                            .iter()
                            .map(|m| format!("WARNING: {m}\n"))
                            .collect();
                        assert_eq!(out.stdout, expected, "unchanged complete renderer stdout");
                        assert_eq!(
                            out.stderr,
                            stderr.as_bytes(),
                            "diagnostics once including omission summary"
                        );
                    },
                );
            }
        }
    }
    eprintln!("REMAINING rows={rows:?}");
    assert_eq!(rows.len(), 10);
    assert!(rows.iter().all(|r| r.1), "all remaining-format rows");
}

#[test]
fn targets_facade_prefix_filter_out_and_strict() {
    let mut rows = Vec::new();
    for positive in [false, true] {
        for spec in ["taint", "taint,taint"] {
            row(&mut rows, format!("targets/{positive}/{spec}"), || {
                let f = Fixture::new(positive, true, false);
                let d = f.admit(positive, false);
                let (outcome, mut meta) = target_inputs(&f, spec);
                assert_eq!(outcome.run.diagnostics.as_ref(), Some(&d));
                let channels = [
                    &outcome.inputs.parse_warnings,
                    &outcome.inputs.load_warnings,
                    &outcome.build_warnings,
                ]
                .map(|v| v.len());
                eprintln!(
                    "TARGET channels={channels:?} quality={:?}",
                    outcome.inputs.parse_quality
                );
                assert_eq!(channels, [1, 1, 1]);
                let project = |meta: &TargetsMeta| {
                    targets::project(
                        &outcome.run.findings,
                        &outcome.run.evidence,
                        &outcome.inputs,
                        meta,
                    )
                };
                let prefix = std::mem::take(&mut meta.run_warnings);
                let bare = project(&meta);
                assert!(
                    d.messages().iter().all(|m| !bare.warnings.contains(m)),
                    "findings alone cannot recover notes"
                );
                meta.run_warnings = prefix.clone();
                let expected = project(&meta);
                assert_eq!(
                    expected.warnings,
                    prefix.into_iter().chain(bare.warnings).collect::<Vec<_>>()
                );
                assert_eq!(
                    !expected.targets.is_empty(),
                    positive,
                    "actual target population"
                );
                assert!(outcome.run.errors.is_empty());
                assert_eq!(
                    expected.repo.as_ref().unwrap().root.as_deref(),
                    meta.repo_root.to_str()
                );
                assert_eq!(expected.repo.as_ref().unwrap().sha, meta.repo_sha);
                assert_eq!(
                    expected.diff.as_ref().unwrap().sha256.as_deref(),
                    Some(
                        format!(
                            "{:x}",
                            Sha256::digest(fs::read(f.temp.path().join("diff.json")).unwrap())
                        )
                        .as_str()
                    )
                );
                if positive && spec.contains(',') {
                    assert!(
                        expected.targets.len() < outcome.run.findings.len(),
                        "duplicate IDs collapse"
                    );
                }
                let normal = invoke(&f, spec, "targets", &["--strict"]);
                let path = f.temp.path().join("targets.json");
                let saved = invoke(
                    &f,
                    spec,
                    "targets",
                    &["--strict", "--out", path.to_str().unwrap()],
                );
                let mut checks = vec![
                    ("stdout", normal.stdout == bytes(&expected)),
                    (
                        "out",
                        saved.stdout.is_empty() && fs::read(path).unwrap() == normal.stdout,
                    ),
                ];
                for mode in ["tier", "severity"] {
                    if !positive {
                        continue;
                    }
                    meta.min_tier = if mode == "tier" {
                        FindingTier::Asserted
                    } else {
                        FindingTier::Candidate
                    };
                    meta.min_severity_rank = if mode == "severity" {
                        output::severity_rank("concern")
                    } else {
                        0
                    };
                    let filtered = project(&meta);
                    assert!(
                        filtered.targets.is_empty(),
                        "actual {mode} before/after separation"
                    );
                    assert_eq!(
                        filtered.warnings, expected.warnings,
                        "filter-independent notes and projector suffix"
                    );
                    let args = if mode == "tier" {
                        ["--min-tier", "asserted"]
                    } else {
                        ["--min-severity", "concern"]
                    };
                    checks.push((
                        mode,
                        invoke(&f, spec, "targets", &args).stdout == bytes(&filtered),
                    ));
                }
                eprintln!(
                    "TARGET checks={checks:?} actual={}",
                    String::from_utf8_lossy(&normal.stdout)
                );
                assert!(
                    d.messages()
                        .iter()
                        .all(|m| !String::from_utf8_lossy(&normal.stderr).contains(m)),
                    "targets has no boundary stderr"
                );
                assert!(
                    checks.iter().all(|c| c.1),
                    "complete targets payload/out/filter checks: {checks:?}"
                );
            });
        }
    }
    eprintln!("TARGETS rows={rows:?}");
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r.1), "all targets facade rows");
}

fn confidence_fixture() -> Fixture {
    let temp = TempDir::new().unwrap();
    let source = "from transport import relay\ndef f(request, c):\n    query = request.GET['q']\n    if c:\n        query = clean(query)\n    else:\n        query = clean(query)\n    cursor.execute(query)\n    relay(query)\n";
    fs::write(temp.path().join("a.py"), source).unwrap();
    fs::write(
        temp.path().join("transport.py"),
        "def relay(x):\n    pass\n",
    )
    .unwrap();
    let text = json!({"files":[{"file_path":"a.py","modify_type":"Modified","diff_lines":[3,9]}]})
        .to_string();
    fs::write(temp.path().join("diff.json"), &text).unwrap();
    let mut options = ReviewOptions::new(temp.path());
    options.no_cache = true;
    options.resolution = ResolutionMode::Scoped;
    let inputs = load_review_inputs(&options, &text).unwrap();
    assert!(
        !inputs.files.contains_key("transport.py") && inputs.files["a.py"].parse_error_count == 0
    );
    Fixture {
        temp,
        options,
        inputs,
    }
}

#[test]
fn confidence_filters_findings_not_notes() {
    let mut rows = Vec::new();
    for floor in [MinConfidence::NameOnly, MinConfidence::Exact] {
        for (format, spec) in [
            ("json", "taint"),
            ("review", "taint"),
            ("sarif", "taint"),
            ("targets", "taint"),
            ("json", "taint,taint"),
        ] {
            row(
                &mut rows,
                format!("confidence/{floor}/{format}/{spec}"),
                || {
                    let mut f = confidence_fixture();
                    f.inputs.min_confidence = floor;
                    let built = build_context(&f.inputs, &f.options).unwrap();
                    let (direct, trace) = taint::slice_with_trace(
                        &built.ctx,
                        &f.inputs.diff,
                        &taint::TaintConfig::default(),
                    )
                    .unwrap();
                    assert!(!direct.findings.is_empty());
                    assert!(
                        direct
                            .evidence
                            .iter()
                            .flatten()
                            .any(
                                |e| classify_with_evidence("Taint", ParseQuality::Clean, e).0
                                    == FindingConfidence::NameOnly
                            ),
                        "actual NameOnly evidence"
                    );
                    assert!(
                        trace.roots.iter().any(|r| r.location.file == "a.py"
                            && r.location.line == 9
                            && r.location.kind == VarAccessKind::Use
                            && r.location.start_byte < r.location.end_byte),
                        "real boundary Use"
                    );
                    let run = run_review(
                        &built.ctx,
                        &f.inputs,
                        &parse_algorithms(spec).unwrap(),
                        &SliceConfig::default(),
                        &AlgorithmParams::default(),
                        f.temp.path(),
                    );
                    assert_eq!(run.findings.len(), run.evidence.len());
                    assert_eq!(
                        !run.findings.is_empty(),
                        floor == MinConfidence::NameOnly,
                        "actual floor separation"
                    );
                    let d = run.diagnostics.as_ref().unwrap();
                    assert_eq!((d.located().len(), d.omitted_count()), (1, 0));
                    let expected = if format == "targets" {
                        let (outcome, meta) = target_inputs(&f, spec);
                        serde_json::to_value(targets::project(
                            &outcome.run.findings,
                            &outcome.run.evidence,
                            &outcome.inputs,
                            &meta,
                        ))
                        .unwrap()
                    } else {
                        f.expected(spec, format).0
                    };
                    let out = invoke(&f, spec, format, &[]);
                    let actual: Value = serde_json::from_slice(&out.stdout).unwrap();
                    let mut checks = vec![("complete", actual == expected)];
                    if format == "sarif" {
                        assert_eq!(notifications(&actual), notifications(&expected));
                    }
                    if format == "review" {
                        for (mode, rank, strip, flags) in [
                            (
                                "severity",
                                output::severity_rank("concern"),
                                false,
                                vec!["--review-min-severity", "concern"],
                            ),
                            (
                                "diagrams",
                                output::severity_rank("warning"),
                                true,
                                vec!["--review-no-diagrams"],
                            ),
                        ] {
                            let mut result = run.results[0].clone();
                            result.warnings = run.warnings.clone();
                            let projected = output::to_compact_review_output(
                                &result,
                                &f.inputs.sources,
                                rank,
                                false,
                                strip,
                            );
                            let varied = invoke(&f, spec, format, &flags);
                            checks.push((
                                mode,
                                serde_json::from_slice::<Value>(&varied.stdout).unwrap()
                                    == serde_json::to_value(projected).unwrap(),
                            ));
                        }
                    }
                    eprintln!(
                        "CONFIDENCE floor={floor} findings={} evidence={} checks={checks:?}",
                        run.findings.len(),
                        run.evidence.len()
                    );
                    assert!(d
                        .messages()
                        .iter()
                        .all(|m| !String::from_utf8_lossy(&out.stderr).contains(m)));
                    assert!(
                        checks.iter().all(|c| c.1),
                        "full confidence/severity/diagram payload checks: {checks:?}"
                    );
                },
            );
        }
    }
    eprintln!("CONFIDENCE rows={rows:?}");
    assert_eq!(rows.len(), 10);
    assert!(rows.iter().all(|r| r.1), "all confidence rows");
}

#[test]
fn rendered_unavailable_message_preserves_real_positive_snapshot() {
    let f = Fixture::new(true, false, false);
    let (result, _) = f.admit_with_result(true, false);
    let snapshot = serde_json::to_value(&result).unwrap();
    let evidence = format!("{:?}", result.evidence);
    let strict = presentation::determine_exit_code(true, &result.diagram_warnings);
    // Already-rendered availability routing; typed capture/serialization have separate S2a/S2b controls.
    let unavailable =
        "taint-context: boundary diagnostics unavailable: read_dir \\\"missing\\\\npath\\\""
            .to_owned();
    let parse = vec!["parse".into()];
    let ordinary = vec![
        "taint-context: ordinary".into(),
        "same".into(),
        "same".into(),
    ];
    let messages = presentation::compose_warnings(&parse, &ordinary, [unavailable.clone()]);
    assert_eq!(
        messages,
        vec![
            "parse",
            "taint-context: ordinary",
            "same",
            "same",
            &unavailable
        ]
    );
    let mut actual = Vec::new();
    presentation::write_messages(&mut actual, &messages).unwrap();
    assert_eq!(actual, format!("WARNING: parse\nWARNING: taint-context: ordinary\nWARNING: same\nWARNING: same\nWARNING: {unavailable}\n").as_bytes());
    assert_eq!(serde_json::to_value(&result).unwrap(), snapshot);
    assert_eq!(
        format!("{:?}", result.evidence),
        evidence,
        "serde-skipped evidence unchanged"
    );
    assert_eq!(
        presentation::determine_exit_code(true, &result.diagram_warnings),
        strict
    );
    assert_eq!(strict, 0);
}
