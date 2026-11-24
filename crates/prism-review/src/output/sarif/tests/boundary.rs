use super::*;
use crate::api::{self, AlgorithmParams, ReviewBoundaryAvailability, ReviewBoundaryDiagnostics};
use prism::slice::{SliceConfig, SlicingAlgorithm};

fn positive() -> api::AlgorithmOutcome {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.js"), "import { relay } from './transport';\nfunction h(input) {\n const command = input;\n eval(command);\n relay(command);\n}").unwrap();
    std::fs::write(
        dir.path().join("transport.js"),
        "export function relay(x) {}",
    )
    .unwrap();
    let opts = api::ReviewOptions::new(dir.path());
    let diff = r#"{"files":[{"file_path":"a.js","modify_type":"Modified","diff_lines":[3,5]}]}"#;
    let inputs = api::load_review_inputs(&opts, diff).unwrap();
    let built = api::build_context(&inputs, &opts).unwrap();
    let actual = api::run_algorithm_with_diagnostics(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
    )
    .unwrap();
    assert!(
        actual
            .result
            .findings
            .iter()
            .any(|f| f.category.as_deref() == Some("taint_sink")),
        "real sink precondition"
    );
    assert!(
        actual
            .result
            .evidence
            .iter()
            .flatten()
            .any(|e| !e.hops.is_empty()),
        "real evidence precondition"
    );
    assert_eq!(
        actual.diagnostics.as_ref().unwrap().located().len(),
        1,
        "real boundary precondition"
    );
    actual
}

fn notices(doc: &Value) -> Vec<Value> {
    doc["runs"][0]["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn empty_diagnostics_preserve_default_and_ordinary_notification_shape() {
    let empty = ReviewBoundaryDiagnostics::default();
    let warnings = ["ordinary".into()];
    for present in [false, true] {
        let slice = if present { &warnings[..] } else { &[] };
        let inputs = SarifInputs::new(&[])
            .parse_warnings(slice)
            .load_warnings(slice)
            .build_warnings(slice);
        assert!(inputs.boundary_diagnostics.is_none());
        let baseline = to_sarif(&inputs);
        assert_eq!(baseline, to_sarif(&inputs.boundary_diagnostics(&empty)));
        let invocation = &baseline["runs"][0]["invocations"][0];
        assert_eq!(
            invocation.get("toolExecutionNotifications").is_some(),
            present
        );
        assert_eq!(
            notices(&baseline),
            if present {
                vec![serde_json::json!({"level":"warning","message":{"text":"ordinary"}}); 3]
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn availability_uses_load_channel_once_without_mutating_inputs() {
    let actual = positive();
    let raw = "denied\n\"\\\u{202e}";
    let rendered = "taint-context: boundary diagnostics unavailable: denied\\n\\\"\\\\\\u{202e}";
    let unavailable = ReviewBoundaryDiagnostics {
        availability: ReviewBoundaryAvailability::ListingUnavailable(raw.into()),
        ..Default::default()
    };
    // AVAILABLE serializer-only path control: retain a real note, alter only its raw caller path.
    let mut escaped = actual.diagnostics.as_ref().unwrap().clone();
    escaped.located[0].record.site.caller.file = "/a.js".into();
    let mut findings = actual.result.findings.clone();
    for f in &mut findings {
        f.file = "/z.js".into();
    }
    let parse = ["repeat".into(), "repeat".into()];
    let load = [
        "load".into(),
        "taint-context: ordinary".into(),
        rendered.into(),
    ];
    let build = ["build".into()];
    let errors = [AlgorithmError {
        algorithm: "Chop".into(),
        error: "supplied error".into(),
    }];
    let before = format!(
        "{:?}",
        (
            &actual.result,
            &findings,
            &unavailable,
            &escaped,
            &parse,
            &load,
            &build,
            &errors
        )
    );
    let mut rows = Vec::new();
    for failed in [false, true] {
        for aligned in [false, true] {
            for missing in [false, true] {
                let diagnostics = if missing { &unavailable } else { &escaped };
                let input = SarifInputs::new(&findings)
                    .evidence(if aligned {
                        &actual.result.evidence
                    } else {
                        &[]
                    })
                    .errors(if failed { &errors } else { &[] })
                    .parse_warnings(&parse)
                    .load_warnings(&load)
                    .build_warnings(&build);
                let baseline = to_sarif(&input);
                assert!(
                    !baseline["runs"][0]["results"]
                        .as_array()
                        .unwrap()
                        .is_empty(),
                    "actual finding admission precondition"
                );
                let doc = to_sarif(&input.boundary_diagnostics(diagnostics));
                let mut expected = Vec::new();
                if failed {
                    expected.push(("error", "Chop: supplied error".to_string()));
                }
                expected.extend(parse.iter().chain(&load).map(|s| ("warning", s.clone())));
                if missing {
                    expected.push(("warning", rendered.into()));
                }
                expected.push(("warning", "build".into()));
                if !aligned {
                    expected.push(("warning", format!("evidence alignment mismatch: {} findings, 0 artifacts; unmatched findings are unlabeled", findings.len())));
                }
                if !missing {
                    expected.push(("warning", "path escapes repo root: /a.js".into()));
                }
                expected.push(("warning", "path escapes repo root: /z.js".into()));
                let mut expected: Vec<_> = expected
                    .into_iter()
                    .map(|(level, text)| serde_json::json!({"level":level,"message":{"text":text}}))
                    .collect();
                if !missing {
                    expected.push(serde_json::json!({
                    "level":"note", "message":{"text":escaped.located()[0].message()},
                    "locations":[{"physicalLocation":{"artifactLocation":{"uri":"/a.js","uriBaseId":"%SRCROOT%"},"region":{"startLine":5}}}]
                }));
                }
                let observed = notices(&doc);
                let mut non_notes = doc.clone();
                non_notes["runs"][0]["invocations"][0]["toolExecutionNotifications"] =
                    serde_json::json!(notices(&baseline));
                rows.push((
                    failed,
                    aligned,
                    missing,
                    observed == expected,
                    non_notes == baseline,
                    doc["runs"][0]["invocations"][0]["executionSuccessful"] == !failed,
                ));
                eprintln!("CHANNEL failed={failed} aligned={aligned} unavailable={missing} observed={observed:?} expected={expected:?}");
            }
        }
    }
    assert_eq!(
        before,
        format!(
            "{:?}",
            (
                &actual.result,
                &findings,
                &unavailable,
                &escaped,
                &parse,
                &load,
                &build,
                &errors
            )
        )
    );
    assert_eq!(
        unavailable.availability(),
        &ReviewBoundaryAvailability::ListingUnavailable(raw.into())
    );
    eprintln!("CHANNEL rows={rows:?}");
    assert!(
        rows.iter().all(|r| r.3 && r.4 && r.5),
        "all eight channels/results/exit rows"
    );
}
