use super::taint_reaches_test::fixture;
use prism::navigation::types::{Reachability, ReasoningWarning, WarningKind};
use prism::reasoning::seeds::SeedSpec;
use prism::reasoning::taint_reaches::taint_reaches;

#[test]
fn witness_boundary_warnings_keep_target_location_for_each_sink_verdict() {
    let fixture = fixture(&[(
        "a.js",
        "function main(req) {\n  const t = req.body;\n  h1(t);\n  observe(t);\n}\n\
         function h1(x) { h2(x); }\nfunction h2(y) { h3(y); }\n\
         function h3(p) {\n  exec(p);\n}\nfunction other(q) {\n  exec(q);\n}\n",
    )]);
    let seed = |line| SeedSpec::Loc {
        file: "a.js".into(),
        line,
    };
    for (line, verdict) in [
        (4, Reachability::Reached),
        (9, Reachability::BoundaryExited),
        (12, Reachability::NotReached),
    ] {
        let evidence = taint_reaches(&fixture.session, &[seed(2)], Some(&[seed(line)])).unwrap();
        let summary = evidence.reasoning.as_ref().unwrap();
        assert_eq!(summary.reachability, Some(verdict));
        let pair = &summary.per_sink[0].sources[0];
        let wire = serde_json::to_value(pair).unwrap();
        if verdict == Reachability::BoundaryExited {
            assert_eq!(wire["reason"], "depth_cap");
        } else {
            assert!(wire.get("reason").is_none());
        }
        let boundaries: Vec<_> = evidence
            .warnings
            .iter()
            .filter(|warning| {
                matches!(
                    warning.kind,
                    WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
                )
            })
            .collect();
        assert_eq!(boundaries.len(), 1, "sink verdict {verdict:?}");
        let warning = serde_json::to_value(boundaries[0]).unwrap();
        assert_eq!(
            warning["message"],
            "taint reached a cross-function boundary at p"
        );
        assert_eq!(warning["location"]["start_line"], 8);
        let details = &warning["kind"]["Reasoning"]["InterproceduralBoundary"];
        assert_eq!(details["sink"], "p");
        assert_eq!(details["reason"], "depth_cap");
    }
}

#[test]
fn text_render_shows_refusal_reason_beside_the_boundary_warning() {
    use prism::navigation::render::render;
    use prism::navigation::types::{Evidence, Warning};

    let fixture = fixture(&[(
        "a.js",
        "function main(req) {\n  const t = req.body;\n  h1(t);\n}\n\
         function h1(x) { h2(x); }\nfunction h2(y) { h3(y); }\n\
         function h3(p) {\n  exec(p);\n}\n",
    )]);
    let seed = |line| SeedSpec::Loc {
        file: "a.js".into(),
        line,
    };
    let evidence = taint_reaches(&fixture.session, &[seed(2)], Some(&[seed(8)])).unwrap();
    let text = render(&evidence, "text");
    let boundary: Vec<_> = text
        .lines()
        .filter(|line| line.contains("InterproceduralBoundary"))
        .collect();
    assert_eq!(
        boundary,
        ["  ! Reasoning(InterproceduralBoundary { sink: \"p\" }): \
             taint reached a cross-function boundary at p (depth_cap)"]
    );

    // Controls: no reason means no suffix, and other warnings keep the Debug rendering.
    let warning = |kind, message: &str| Warning {
        kind,
        message: message.into(),
        location: None,
    };
    let plain = Evidence {
        query: "q".into(),
        items: vec![],
        truncated: false,
        warnings: vec![
            warning(
                WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary {
                    sink: "p".into(),
                    reason: None,
                }),
                "boundary",
            ),
            warning(WarningKind::StaleIndex, "stale"),
        ],
        graph: None,
        reasoning: None,
    };
    assert_eq!(
        render(&plain, "text"),
        "q\n  ! Reasoning(InterproceduralBoundary { sink: \"p\" }): boundary\n  ! StaleIndex: stale\n"
    );
}
