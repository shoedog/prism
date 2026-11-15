use super::*;
use prism::{data_flow::VarAccessKind, slice::SliceResult};
use prism_review::algorithms::{self, taint};

fn same_result(a: &SliceResult, b: &SliceResult) {
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
    assert_eq!(format!("{:?}", a.evidence), format!("{:?}", b.evidence));
}

#[test]
fn typed_legacy_and_direct_result_have_only_note_delta() {
    let mut rows = Vec::new();
    for loaded in [false, true] {
        let source = "import { send } from './transport';\nfunction h(req) {\n const copy = req;\n eval(copy);\n send(copy);\n}";
        let diff = if loaded {
            vec![("a.js", &[3, 5][..]), ("transport.js", &[][..])]
        } else {
            vec![("a.js", &[3, 5][..])]
        };
        let (_tmp, repo, text) = write_repo(
            &[
                ("a.js", source),
                ("transport.js", "export function send(x) {}"),
            ],
            &diff,
        );
        let opts = ReviewOptions::new(&repo);
        let inputs = load_review_inputs(&opts, &text).unwrap();
        let built = build_context(&inputs, &opts).unwrap();
        let config = SliceConfig::default();
        let params = AlgorithmParams::default();
        let mut direct =
            taint::slice(&built.ctx, &inputs.diff, &taint::TaintConfig::default()).unwrap();
        assert!(!direct.findings.is_empty(), "fixture needs real sink");
        assert!(
            direct.evidence.iter().flatten().any(|e| !e.hops.is_empty()),
            "fixture needs evidence"
        );
        direct.align_evidence();
        algorithms::finalize_diagrams(&mut direct, config.diagram_node_cap);
        annotate_finding_parse_quality(&mut direct.findings, &inputs.files);
        let typed = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            &built.ctx,
            &inputs,
            &config,
            &params,
            &repo,
        )
        .unwrap();
        same_result(&typed.result, &direct);
        let legacy = run_algorithm(
            SlicingAlgorithm::Taint,
            &built.ctx,
            &inputs,
            &config,
            &params,
            &repo,
        )
        .unwrap();
        let projected = typed.diagnostics.as_ref();
        if let Some(d) = projected {
            assert_eq!(d.availability(), &ReviewBoundaryAvailability::Available);
            assert_eq!(d.omitted_count(), 0);
            assert!(d.summary().is_none());
            for note in d.located() {
                assert_eq!(
                    (note.file(), note.line(), note.callee()),
                    ("a.js", 5, "send")
                );
                assert_eq!(&source[note.byte_range()], "send(copy)");
                assert_eq!(note.kind(), ReviewBoundaryKind::InRepo);
                assert_eq!(note.language(), prism::languages::Language::JavaScript);
                assert!(
                    matches!(note.hint(), ReviewImportHint::InRepo { files, .. } if files == &["transport.js"])
                );
                assert!(!note.message().is_empty());
            }
            direct.warnings.extend(d.messages());
        }
        same_result(&legacy, &direct);
        let high = review(&opts, &text, &[SlicingAlgorithm::Taint], &config, &params).unwrap();
        rows.push((
            loaded,
            projected.map(|d| d.located().len()),
            high.run.diagnostics.as_ref().map(|d| d.located().len()),
        ));
        let other = run_algorithm_with_diagnostics(
            SlicingAlgorithm::AbsenceSlice,
            &built.ctx,
            &inputs,
            &config,
            &params,
            &repo,
        )
        .unwrap();
        assert!(other.diagnostics.is_none());
    }
    eprintln!("TYPED rows={rows:?}");
    assert_eq!(rows, [(false, Some(1), Some(1)), (true, Some(0), Some(0))]);
}

#[test]
fn duplicate_mixed_and_note_only_requests_reuse_one_population() {
    use SlicingAlgorithm::{Chop, Taint};
    let source = format!(
        "import {{ relay }} from './transport';\nfunction h(req) {{\n{}\n}}",
        [" relay(req);"; 12].join("\n")
    );
    let lines: Vec<_> = (3..=14).collect();
    let (_tmp, repo, text) = write_repo(
        &[
            ("a.js", &source),
            ("transport.js", "export function relay(x) {}"),
        ],
        &[("a.js", &lines)],
    );
    let opts = ReviewOptions::new(&repo);
    let mut inputs = load_review_inputs(&opts, &text).unwrap();
    inputs.parse_warnings = vec![
        "repeat".into(),
        "repeat".into(),
        "taint-context: ordinary".into(),
    ];
    let built = build_context(&inputs, &opts).unwrap();
    let (direct, trace) =
        taint::slice_with_trace(&built.ctx, &inputs.diff, &taint::TaintConfig::default()).unwrap();
    assert!(direct.findings.is_empty(), "fixture must be note-only");
    assert_eq!(
        built
            .ctx
            .cpg
            .call_graph
            .calls
            .values()
            .flatten()
            .filter(|s| s.callee_name == "relay")
            .count(),
        12
    );
    let retained: Vec<_> = source
        .match_indices("req);")
        .map(|(start, _)| {
            (
                start,
                trace.roots.iter().any(|r| {
                    r.origin == taint::TaintRootOrigin::Diff
                        && r.location.file == "a.js"
                        && r.location.kind == VarAccessKind::Use
                        && r.location.start_byte == start
                        && r.location.end_byte == start + 3
                }),
            )
        })
        .collect();
    assert_eq!(retained.len(), 12);
    assert!(
        retained.iter().all(|(_, present)| *present),
        "fixture needs every argument: {retained:?}"
    );
    let mut rows = Vec::new();
    for requested in [
        vec![Taint, Taint],
        vec![Taint, Chop, Taint],
        vec![Chop, Taint, Taint],
    ] {
        let run = run_review(
            &built.ctx,
            &inputs,
            &requested,
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            &repo,
        );
        assert_eq!(run.results.len(), 2);
        for nested in &run.results {
            same_result(nested, &direct);
        }
        assert!(run.findings.is_empty());
        assert_eq!(
            run.errors
                .iter()
                .map(|e| e.algorithm.as_str())
                .collect::<Vec<_>>(),
            if requested.contains(&Chop) {
                vec!["Chop"]
            } else {
                vec![]
            }
        );
        assert_eq!(
            run.algorithms_run,
            requested.iter().map(|a| a.name()).collect::<Vec<_>>()
        );
        assert!(run.warnings.starts_with(&inputs.parse_warnings));
        rows.push((
            run.diagnostics
                .as_ref()
                .map(|d| (d.located().len(), d.omitted_count())),
            run.warnings.len() - inputs.parse_warnings.len(),
        ));
    }
    eprintln!("MULTI rows={rows:?}");
    assert_eq!(rows, [(Some((10, 2)), 11); 3]);
}

#[test]
fn typed_annotations_and_fallback_finalization_preserve_contract() {
    let echo = echo_fixture();
    let callers: String = (0..5)
        .map(|i| format!("def handle{i}():\n    return fetch()\n\n"))
        .collect();
    fs::write(
        echo.1.join("svc.py"),
        format!("from client import fetch\n\n{callers}"),
    )
    .unwrap();
    for (fixture, algorithm) in [
        (degraded_absence_fixture(), SlicingAlgorithm::AbsenceSlice),
        (echo, SlicingAlgorithm::EchoSlice),
    ] {
        let (_tmp, repo, text) = fixture;
        let opts = ReviewOptions::new(&repo);
        let inputs = load_review_inputs(&opts, &text).unwrap();
        let built = build_context(&inputs, &opts).unwrap();
        let config = SliceConfig {
            algorithm,
            diagram_node_cap: 1,
            ..Default::default()
        };
        let mut direct = algorithms::run_slicing_inner(&built.ctx, &inputs.diff, &config).unwrap();
        direct.align_evidence();
        algorithms::finalize_diagrams(&mut direct, 1);
        if algorithm == SlicingAlgorithm::EchoSlice {
            assert!(
                direct.diagrams.iter().any(|d| d.nodes.len() > 4),
                "fixture must exceed minimum cap4"
            );
            assert!(
                !direct.diagram_warnings.is_empty(),
                "fixture needs actual fallback warning"
            );
            assert!(
                direct
                    .diagrams
                    .iter()
                    .chain(direct.findings.iter().flat_map(|f| &f.diagrams))
                    .any(|d| !d.mermaid.is_empty()),
                "fixture needs diagram"
            );
        } else {
            assert!(!direct.findings.is_empty());
            assert!(direct.findings.iter().all(|f| f.parse_quality.is_none()));
        }
        annotate_finding_parse_quality(&mut direct.findings, &inputs.files);
        let typed = run_algorithm_with_diagnostics(
            algorithm,
            &built.ctx,
            &inputs,
            &config,
            &AlgorithmParams::default(),
            &repo,
        )
        .unwrap();
        same_result(&direct, &typed.result);
        assert!(typed.diagnostics.is_none());
        same_result(
            &direct,
            &run_algorithm(
                algorithm,
                &built.ctx,
                &inputs,
                &config,
                &AlgorithmParams::default(),
                &repo,
            )
            .unwrap(),
        );
    }
}

#[test]
fn finding_filter_cannot_remove_boundary_diagnostics() {
    let source = "from transport import relay\ndef f(request, c):\n    query = request.GET['q']\n    if c:\n        query = clean(query)\n    else:\n        query = clean(query)\n    cursor.execute(query)\n    relay(query)\n";
    let (_tmp, repo, text) = write_repo(
        &[
            ("a.py", source),
            ("transport.py", "def relay(x):\n    pass\n"),
        ],
        &[("a.py", &[3, 9])],
    );
    let opts = ReviewOptions::new(&repo);
    let mut rows = Vec::new();
    for floor in [
        prism::finding_confidence::MinConfidence::NameOnly,
        prism::finding_confidence::MinConfidence::Exact,
    ] {
        let mut inputs = load_review_inputs(&opts, &text).unwrap();
        inputs.min_confidence = floor;
        let built = build_context(&inputs, &opts).unwrap();
        let (direct, trace) =
            taint::slice_with_trace(&built.ctx, &inputs.diff, &taint::TaintConfig::default())
                .unwrap();
        assert!(!direct.findings.is_empty(), "fixture needs sink");
        assert!(
            direct.evidence.iter().flatten().any(|e| {
                prism::finding_confidence::classify_with_evidence(
                    "Taint",
                    prism::finding_confidence::ParseQuality::Clean,
                    e,
                )
                .0 == prism::finding_confidence::FindingConfidence::NameOnly
            }),
            "fixture needs NameOnly"
        );
        assert!(
            trace.roots.iter().any(|r| r.location.line == 9
                && r.location.kind == VarAccessKind::Use
                && r.location.start_byte < r.location.end_byte),
            "fixture needs boundary Use"
        );
        let run = run_review(
            &built.ctx,
            &inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            &repo,
        );
        assert_eq!(run.findings.len(), run.evidence.len());
        rows.push((
            floor,
            run.findings.len(),
            run.diagnostics.as_ref().map(|d| d.located().len()),
        ));
    }
    eprintln!("FILTER rows={rows:?}");
    assert!(
        rows[0].1 > 0 && rows[1].1 == 0,
        "fixture must separate confidence floors: {rows:?}"
    );
    assert_eq!((rows[0].2, rows[1].2), (Some(1), Some(1)));
}

// RS15 (#602 S3a.1s): REAL, public API only. A compatibility control: it is expected to
// pass on the base too, because the explicit-source suppression predates source scopes.
#[test]
fn source_scope_preserves_high_level_explicit_suppression() {
    // Line 5 is the dangerous diff-line flow (`command` from line 4); line 8 is a safe source.
    let source = "import os\n\ndef dangerous(user):\n    command = user\n    os.system(command)\n\ndef safe(user):\n    value = user\n    return 42\n";
    let (_tmp, repo, text) = write_repo(&[("a.py", source)], &[("a.py", &[4][..])]);
    let opts = ReviewOptions::new(&repo);
    let inputs = load_review_inputs(&opts, &text).unwrap();
    let built = build_context(&inputs, &opts).unwrap();
    let config = SliceConfig::default();
    let sink_on = |result: &SliceResult, line: usize| {
        let sinks = result.findings.iter().enumerate();
        let mut sinks = sinks.filter(|(_, f)| {
            (f.file.as_str(), f.line, f.category.as_deref()) == ("a.py", line, Some("taint_sink"))
        });
        sinks.next().map(|(index, _)| index)
    };
    let low_level = |explicit: usize, from_diff: bool| {
        let taint_config = taint::TaintConfig {
            sources: vec![("a.py".to_string(), explicit)],
            taint_from_diff: from_diff,
            ..taint::TaintConfig::default()
        };
        let mut direct = taint::slice(&built.ctx, &inputs.diff, &taint_config).unwrap();
        // Normalized exactly as `typed_legacy_and_direct_result_have_only_note_delta`.
        direct.align_evidence();
        algorithms::finalize_diagrams(&mut direct, config.diagram_node_cap);
        annotate_finding_parse_quality(&mut direct.findings, &inputs.files);
        direct
    };

    // (2) low level, `taint_from_diff: true`: the dangerous sink is present with evidence.
    let both = low_level(8, true);
    let index = sink_on(&both, 5).expect("fixture needs the dangerous sink");
    let evidence = both.evidence[index].as_ref();
    assert!(
        evidence.is_some_and(|e| !e.hops.is_empty()),
        "fixture needs evidence"
    );

    // (1) safe explicit source, (3) dangerous explicit source: public API versus low level.
    for (explicit, dangerous) in [(8, false), (4, true)] {
        let mut params = AlgorithmParams::default();
        params.taint_sources = vec![format!("a.py:{explicit}")];
        let run = |algorithm| {
            run_algorithm_with_diagnostics(algorithm, &built.ctx, &inputs, &config, &params, &repo)
        };
        let typed = run(SlicingAlgorithm::Taint).unwrap();
        assert_eq!(
            sink_on(&typed.result, 5).is_some(),
            dangerous,
            "explicit source {explicit}"
        );
        let mut direct = low_level(explicit, false);
        same_result(&typed.result, &direct);
        let legacy = run_algorithm(
            SlicingAlgorithm::Taint,
            &built.ctx,
            &inputs,
            &config,
            &params,
            &repo,
        );
        if let Some(d) = typed.diagnostics.as_ref() {
            direct.warnings.extend(d.messages());
        }
        same_result(&legacy.unwrap(), &direct);
    }
}
