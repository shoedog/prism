use super::*;
use crate::algorithms::taint::{
    self,
    boundary::{collect, display, records::Availability},
};
use crate::api::{build_context, load_review_inputs, ReviewOptions};

fn positive() -> (SliceResult, BoundaryCollection) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.js"), "import { send } from './transport';\nfunction h(req) {\n const copy = req;\n eval(copy);\n send(copy);\n}").unwrap();
    std::fs::write(
        dir.path().join("transport.js"),
        "export function send(x) {}",
    )
    .unwrap();
    let opts = ReviewOptions::new(dir.path());
    let diff = r#"{"files":[{"file_path":"a.js","modify_type":"Modified","diff_lines":[3,5]}]}"#;
    let inputs = load_review_inputs(&opts, diff).unwrap();
    let built = build_context(&inputs, &opts).unwrap();
    let (result, trace) =
        taint::slice_with_trace(&built.ctx, &inputs.diff, &taint::TaintConfig::default()).unwrap();
    assert!(!result.findings.is_empty(), "fixture needs actual sink");
    assert!(
        result.evidence.iter().flatten().any(|e| !e.hops.is_empty()),
        "fixture needs actual evidence"
    );
    let records = collect::collect(&inputs, &built.ctx, &trace);
    assert_eq!(records.records.len(), 1, "fixture needs actual boundary");
    (result, records)
}

#[test]
fn capture_once_retains_complete_available_and_unavailable_states() {
    let (_, actual) = positive();
    // Pure owner-state/closure control: these twelve records are synthetic identities.
    let twelve = (0..12)
        .map(|i| {
            let mut record = actual.records[0].clone();
            record.site.start_byte += i;
            record.site.end_byte += i;
            record
        })
        .collect();
    let mut rows = Vec::new();
    let default = RunDiagnostics::default();
    assert!(default.project().is_none());
    let initially_needed = default.needs_collection();
    for complete in [
        BoundaryCollection {
            availability: Availability::Available,
            records: vec![],
        },
        BoundaryCollection {
            availability: Availability::Available,
            records: twelve,
        },
        BoundaryCollection {
            availability: Availability::ListingUnavailable("denied".into()),
            records: vec![],
        },
    ] {
        let mut owner = RunDiagnostics::default();
        let mut calls = 0;
        owner.capture_once(|| {
            calls += 1;
            complete.clone()
        });
        owner.capture_once(|| panic!("second capture must not execute"));
        let projected = owner.project();
        rows.push((
            calls,
            owner.needs_collection(),
            owner.complete.as_ref() == Some(&complete),
            projected.as_ref().map(|d| {
                (
                    d.located().len(),
                    d.omitted_count(),
                    d.availability().clone(),
                )
            }),
        ));
    }
    eprintln!("OWNER initially_needed={initially_needed} rows={rows:?}");
    assert!(initially_needed);
    assert_eq!(
        rows,
        [
            (1, false, true, Some((0, 0, Availability::Available))),
            (1, false, true, Some((10, 2, Availability::Available))),
            (
                1,
                false,
                true,
                Some((0, 0, Availability::ListingUnavailable("denied".into())))
            ),
        ]
    );
}

#[test]
fn availability_and_ordinary_warnings_preserve_a_real_result() {
    let (mut result, _) = positive();
    result.warnings = ["repeat", "repeat", "taint-context: ordinary", "sentinel"]
        .map(str::to_owned)
        .to_vec();
    let raw = "denied\n\"x\"\\\t\r\u{202e}é";
    let expected =
        "taint-context: boundary diagnostics unavailable: denied\\n\\\"x\\\"\\\\\\t\\r\\u{202e}é";
    let mut rows = Vec::new();
    let empty = ReviewBoundaryDiagnostics::default();
    assert!(empty.located().is_empty() && empty.summary().is_none() && empty.omitted_count() == 0);
    assert_eq!(empty.availability(), &Availability::Available);
    assert!(AlgorithmOutcome::new(result.clone()).diagnostics.is_none());
    // Injected composition states, not an OS listing-error or algorithm-failure claim.
    for diagnostics in [
        None,
        Some(empty),
        Some(display::project(&BoundaryCollection {
            availability: Availability::ListingUnavailable(raw.into()),
            records: vec![],
        })),
    ] {
        let unavailable = diagnostics
            .as_ref()
            .is_some_and(|d| matches!(d.availability(), Availability::ListingUnavailable(_)));
        if unavailable {
            let d = diagnostics.as_ref().unwrap();
            assert_eq!(
                d.availability(),
                &Availability::ListingUnavailable(raw.into())
            );
            assert!(d.located().is_empty() && d.summary().is_none());
        }
        let messages = diagnostics
            .as_ref()
            .map(|d| d.messages())
            .unwrap_or_default();
        let output = AlgorithmOutcome {
            result: result.clone(),
            diagnostics,
        }
        .into_legacy_result();
        assert!(output.warnings.starts_with(&result.warnings));
        assert_eq!(
            format!("{:?}", output.evidence),
            format!("{:?}", result.evidence)
        );
        let mut expected_result = result.clone();
        if unavailable {
            expected_result.warnings.push(expected.into());
        }
        rows.push((
            messages,
            serde_json::to_value(output).unwrap() == serde_json::to_value(expected_result).unwrap(),
        ));
    }
    eprintln!("AVAILABILITY rows={rows:?}");
    assert_eq!(
        rows,
        [
            (vec![], true),
            (vec![], true),
            (vec![expected.into()], true)
        ]
    );
}
