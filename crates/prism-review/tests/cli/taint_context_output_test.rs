mod fixtures;
#[path = "../../src/cli_output.rs"]
mod presentation;
mod remaining;

use fixtures::{notifications, row, Fixture};
use serde_json::{json, Value};

#[test]
fn structured_single_multi_population_and_payloads() {
    let mut rows = Vec::new();
    for positive in [false, true] {
        for format in ["json", "review"] {
            for spec in ["taint", "taint,parentfunction", "taint,taint"] {
                if positive && spec == "taint,taint" {
                    continue;
                }
                row(
                    &mut rows,
                    format!("structured/{positive}/{format}/{spec}"),
                    || {
                        let fixture = Fixture::new(positive, false, false);
                        let diagnostics = fixture.admit(positive, false);
                        let messages = diagnostics.messages();
                        let (expected, captured) = fixture.expected(spec, format);
                        assert_eq!(
                            captured.unwrap(),
                            diagnostics,
                            "independent API capture precondition"
                        );
                        let (actual, stderr) = fixture.invoke(spec, format);
                        assert_eq!(
                            actual["warnings"],
                            json!(messages),
                            "one complete outer note population"
                        );
                        if spec.contains(',') {
                            let results = actual["results"].as_array().unwrap();
                            assert_eq!(results.len(), 2, "algorithm result multiplicity");
                            for result in results {
                                let nested: Vec<String> = serde_json::from_value(
                                    result.get("warnings").cloned().unwrap_or(json!([])),
                                )
                                .unwrap();
                                assert!(
                                    nested.iter().all(|w| !messages.contains(w)),
                                    "no nested diagnostic messages"
                                );
                            }
                        }
                        assert!(
                            messages.iter().all(|m| !stderr.contains(m)),
                            "no boundary stderr"
                        );
                        assert_eq!(
                            actual, expected,
                            "complete structured payload, order and diagrams"
                        );
                    },
                );
            }
        }
    }
    eprintln!("STRUCTURED rows={rows:?}");
    assert_eq!(rows.len(), 10);
    assert!(rows.iter().all(|r| r.1), "all ten structured rows");
}

#[test]
fn sarif_typed_notes_keep_original_channels() {
    let mut rows = Vec::new();
    for positive in [false, true] {
        for spec in ["taint", "taint,taint"] {
            row(&mut rows, format!("sarif/{positive}/{spec}"), || {
                let fixture = Fixture::new(positive, positive, false);
                let diagnostics = fixture.admit(positive, false);
                let built =
                    prism_review::api::build_context(&fixture.inputs, &fixture.options).unwrap();
                let counts = [
                    &fixture.inputs.parse_warnings,
                    &fixture.inputs.load_warnings,
                    &built.warnings,
                ]
                .map(|channel| channel.len());
                eprintln!(
                    "CHANNELS {positive}/{spec}: counts={counts:?} quality={:?}",
                    fixture.inputs.parse_quality
                );
                assert_eq!(
                    counts,
                    [usize::from(positive); 3],
                    "all ordinary warning channels"
                );
                let ordinary: Vec<Value> = fixture
                    .inputs
                    .parse_warnings
                    .iter()
                    .chain(&fixture.inputs.load_warnings)
                    .chain(&built.warnings)
                    .map(|message| json!({"level":"warning", "message":{"text":message}}))
                    .collect();
                let mut note_rows: Vec<Value> = diagnostics
                    .located()
                    .iter()
                    .map(|note| {
                        json!({
                            "level":"note", "message":{"text":note.message()},
                            "locations":[{"physicalLocation":{
                                "artifactLocation":{"uri":"a.js", "uriBaseId":"%SRCROOT%"},
                                "region":{"startLine":note.line()}
                            }}]
                        })
                    })
                    .collect();
                if let Some(summary) = diagnostics.summary() {
                    note_rows.push(json!({"level":"note", "message":{"text":summary}}));
                }
                let (expected, captured) = fixture.expected(spec, "sarif");
                assert_eq!(
                    captured.unwrap(),
                    diagnostics,
                    "SARIF producer capture precondition"
                );
                let expected_notifications: Vec<_> =
                    ordinary.iter().chain(&note_rows).cloned().collect();
                assert_eq!(
                    notifications(&expected),
                    expected_notifications,
                    "API notification channel precondition"
                );
                let (actual, stderr) = fixture.invoke(spec, "sarif");
                assert_eq!(
                    notifications(&actual),
                    expected_notifications,
                    "original channels then located notes and exact summary"
                );
                assert_eq!(
                    actual["runs"][0]["invocations"][0]["executionSuccessful"],
                    true
                );
                assert!(
                    actual["runs"][0]["properties"].get("errors").is_none(),
                    "unchanged empty-errors omission"
                );
                if !positive {
                    assert_eq!(actual["runs"][0]["results"], json!([]));
                    assert_eq!(actual["runs"][0]["tool"]["driver"]["rules"], json!([]));
                }
                assert!(
                    diagnostics.messages().iter().all(|m| !stderr.contains(m)),
                    "no boundary stderr"
                );
                assert_eq!(
                    actual, expected,
                    "complete SARIF payload, results and evidence"
                );
            });
        }
    }
    eprintln!("SARIF rows={rows:?}");
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r.1), "all four SARIF rows");
}

#[test]
fn callers_non_taint_and_confidence_flag_controls() {
    let mut rows = Vec::new();
    for loaded in [false, true] {
        for format in ["json", "review", "sarif"] {
            row(&mut rows, format!("negative/{loaded}/{format}"), || {
                let fixture = Fixture::new(true, false, loaded);
                fixture.admit(true, loaded);
                let spec = if loaded { "taint" } else { "parentfunction" };
                let (expected, diagnostics) = fixture.expected(spec, format);
                if loaded {
                    assert!(diagnostics.unwrap().messages().is_empty());
                } else {
                    assert!(
                        diagnostics.is_none(),
                        "non-Taint does not capture diagnostics"
                    );
                }
                let (actual, stderr) = fixture.invoke(spec, format);
                assert!(
                    !stderr.contains("taint-context:"),
                    "negative has no boundary stderr"
                );
                assert_eq!(actual, expected, "complete negative payload");
            });
        }
    }
    for spec in ["taint", "taint,taint"] {
        row(&mut rows, format!("callers/{spec}"), || {
            let fixture = Fixture::new(true, false, false);
            fixture.admit(true, false);
            let (expected, diagnostics) = fixture.expected(spec, "callers");
            assert!(diagnostics.is_none());
            let (actual, stderr) = fixture.invoke(spec, "callers");
            assert!(
                !stderr.contains("taint-context:"),
                "callers early return has no notes"
            );
            assert_eq!(
                actual, expected,
                "raw caller graph independent of algorithms"
            );
        });
    }
    // Existing min_confidence_admission_test retains pre-load rejection for disallowed formats.
    eprintln!("NEGATIVE rows={rows:?}");
    assert_eq!(rows.len(), 8);
    assert!(rows.iter().all(|r| r.1), "all eight negative rows");
}

#[test]
fn ordinary_message_composition_and_bytes() {
    let mut rows = Vec::new();
    for (label, parse, ordinary, notes, expected) in [
        ("empty", vec![], vec![], vec![], vec![]),
        (
            "ordinary-duplicates",
            vec!["same"],
            vec!["same", "same"],
            vec![],
            vec!["same", "same", "same"],
        ),
        (
            "notes-only",
            vec![],
            vec![],
            vec!["taint-context: note", "summary"],
            vec!["taint-context: note", "summary"],
        ),
        (
            "prefix-and-order",
            vec!["parse", "parse"],
            vec!["taint-context: ordinary", "ordinary"],
            vec!["taint-context: note", "summary"],
            vec![
                "parse",
                "parse",
                "taint-context: ordinary",
                "ordinary",
                "taint-context: note",
                "summary",
            ],
        ),
    ] {
        row(&mut rows, label.into(), || {
            let strings =
                |values: Vec<&str>| values.into_iter().map(String::from).collect::<Vec<_>>();
            let actual =
                presentation::compose_warnings(&strings(parse), &strings(ordinary), strings(notes));
            assert_eq!(
                actual,
                strings(expected.clone()),
                "ordered composition preserves ordinary duplicates and prefixes"
            );
            let mut bytes = Vec::new();
            presentation::write_messages(&mut bytes, &actual).unwrap();
            let expected_bytes: String = expected
                .iter()
                .map(|message| format!("WARNING: {message}\n"))
                .collect();
            assert_eq!(
                bytes,
                expected_bytes.as_bytes(),
                "exact supplied-writer bytes"
            );
        });
    }
    eprintln!("ORDINARY rows={rows:?}");
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r.1), "all four ordinary message rows");
}
