//! CONTROLLED: real parsed maps and selected evidence assembled by the test; no engine execution.
use super::*;
use crate::api::map_view::{
    tests::{finding, witness, Fixture, CONTEXT},
    MapView,
};
use prism::finding_confidence::{EvidencePath, MinConfidence, ResolutionMode};
use prism::resolution::ResolutionConfidence;

fn document(
    f: &Fixture,
    findings: &[SliceFinding],
    evidence: &[Option<EvidencePath>],
    context: bool,
    mode: ResolutionMode,
    floor: MinConfidence,
) -> Value {
    let mut input = SarifInputs::new(findings)
        .evidence(evidence)
        .files(&f.inputs.files)
        .sources(&f.inputs.sources)
        .parse_quality(&f.inputs.parse_quality)
        .resolution(mode)
        .min_confidence(floor);
    if context {
        input = input.context_maps(f.view());
    }
    to_sarif(&input)
}
fn scoped(
    f: &Fixture,
    findings: &[SliceFinding],
    evidence: &[Option<EvidencePath>],
    context: bool,
) -> Value {
    document(
        f,
        findings,
        evidence,
        context,
        ResolutionMode::Scoped,
        MinConfidence::NameOnly,
    )
}
fn single(f: &Fixture, context: bool) -> Value {
    scoped(
        f,
        &[finding("taint", "context.py")],
        &[Some(witness("context.py", ResolutionConfidence::Exact))],
        context,
    )
}
fn result(doc: &Value) -> &Value {
    &doc["runs"][0]["results"][0]
}
fn assert_fixture(f: &Fixture) {
    assert!(f.inputs.files.contains_key("main.py") && !f.inputs.files.contains_key("context.py"));
    assert!(f.files.contains_key("context.py"));
    assert!(f.sources["context.py"].lines().nth(2).is_some());
}
fn context_grade(doc: &Value) {
    assert_eq!(result(doc)["properties"]["parse_quality"], "clean");
    assert_eq!(result(doc)["properties"]["tier"], "asserted");
}
#[test]
fn no_context_is_byte_identical() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let findings = [finding("taint", "main.py"), finding("echo", "main.py")];
    let evidence = [Some(witness("main.py", ResolutionConfidence::Exact)), None];
    let input = SarifInputs::new(&findings)
        .evidence(&evidence)
        .files(&f.inputs.files)
        .sources(&f.inputs.sources)
        .parse_quality(&f.inputs.parse_quality)
        .resolution(ResolutionMode::Scoped);
    let bare = serde_json::to_vec(&to_sarif(&input)).unwrap();
    let original = MapView::original(&f.inputs);
    assert_eq!(
        bare,
        serde_json::to_vec(&to_sarif(&input.context_maps(original))).unwrap()
    );
    assert_eq!(
        bare,
        serde_json::to_vec(&document(
            &f,
            &findings,
            &evidence,
            false,
            ResolutionMode::Scoped,
            MinConfidence::NameOnly
        ))
        .unwrap()
    );
}
#[test]
fn context_finding_is_graded_from_the_bundle() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    assert!(f.files["context.py"].error_rate() <= 0.01);
    assert!(!f.quality.contains_key("context.py"));
    let off = single(&f, false);
    let on = single(&f, true);
    assert_eq!(result(&off)["properties"]["parse_quality"], "unknown");
    assert_eq!(result(&off)["properties"]["tier"], "candidate");
    context_grade(&on);
    assert_eq!(result(&on)["properties"]["confidence"], "exact");
}
#[test]
fn poor_context_parse_grades_down() {
    let f =
        Fixture::new("def contextual(value):\n    first = value\n    broken =\n    return first\n");
    assert_fixture(&f);
    let rate = f.files["context.py"].error_rate();
    assert!(
        rate > 0.01 && rate <= 0.3,
        "inadmissible precondition: admissible degraded parse {rate}"
    );
    let grade = &f.quality["context.py"].quality;
    let on = single(&f, true);
    assert_eq!(result(&on)["properties"]["parse_quality"], grade.as_str());
    assert_eq!(result(&on)["properties"]["tier"], "candidate");
}
#[test]
fn exact_floor_preserves_evidence_admission() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let findings = [finding("taint", "context.py")];
    let exact = [Some(witness("context.py", ResolutionConfidence::Exact))];
    let off = document(
        &f,
        &findings,
        &exact,
        false,
        ResolutionMode::Scoped,
        MinConfidence::Exact,
    );
    let on = document(
        &f,
        &findings,
        &exact,
        true,
        ResolutionMode::Scoped,
        MinConfidence::Exact,
    );
    assert_eq!(off["runs"][0]["results"].as_array().unwrap().len(), 1);
    assert_eq!(on["runs"][0]["results"].as_array().unwrap().len(), 1);
    assert_eq!(result(&off)["ruleId"], result(&on)["ruleId"]);
    assert_eq!(result(&off)["locations"], result(&on)["locations"]);
    assert_eq!(result(&off)["message"], result(&on)["message"]);
    assert_eq!(result(&off)["properties"]["confidence"], "exact");
    assert_eq!(result(&on)["properties"]["confidence"], "exact");
    assert_eq!(result(&off)["properties"]["tier"], "candidate");
    context_grade(&on);
    assert_ne!(
        result(&off)["partialFingerprints"],
        result(&on)["partialFingerprints"]
    );
    for evidence in [
        vec![Some(witness("context.py", ResolutionConfidence::NameOnly))],
        vec![None],
    ] {
        for ctx in [false, true] {
            let d = document(
                &f,
                &findings,
                &evidence,
                ctx,
                ResolutionMode::Scoped,
                MinConfidence::Exact,
            );
            assert!(d["runs"][0]["results"].as_array().unwrap().is_empty());
        }
    }
}
#[test]
fn fingerprint_uses_the_context_line() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let finding = finding("taint", "context.py");
    let off = single(&f, false);
    let on = single(&f, true);
    let key = "prism/finding/v1";
    assert_eq!(
        result(&off)["partialFingerprints"][key],
        fingerprint(&finding, "")
    );
    assert_eq!(
        result(&on)["partialFingerprints"][key],
        fingerprint(&finding, CONTEXT.lines().nth(2).unwrap().trim())
    );
    assert_ne!(
        result(&off)["partialFingerprints"][key],
        result(&on)["partialFingerprints"][key]
    );
}
#[test]
fn non_taint_finding_ignores_the_bundle() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let findings = [finding("echo", "context.py")];
    let evidence = [Some(witness("context.py", ResolutionConfidence::Exact))];
    let selected = evidence[0].as_ref().unwrap();
    let original_quality = prism::finding_confidence::parse_quality_for_selected_evidence(
        &findings[0],
        Some(selected),
        &f.inputs.parse_quality,
        &f.inputs.files,
    );
    let union_quality = prism::finding_confidence::parse_quality_for_selected_evidence(
        &findings[0],
        Some(selected),
        &f.quality,
        &f.files,
    );
    assert_ne!(
        prism::finding_confidence::classify_with_evidence("echo", original_quality, selected),
        prism::finding_confidence::classify_with_evidence("echo", union_quality, selected),
        "inadmissible precondition: selecting context would change this non-taint grade"
    );
    let off = scoped(&f, &findings, &evidence, false);
    let on = scoped(&f, &findings, &evidence, true);
    assert_eq!(result(&off), result(&on));
    assert_eq!(result(&on)["properties"]["parse_quality"], "unknown");
    assert_eq!(
        result(&on)["partialFingerprints"]["prism/finding/v1"],
        fingerprint(&findings[0], "")
    );
}
#[test]
fn baseline_taint_finding_is_unchanged() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let findings = [finding("taint", "main.py")];
    let evidence = [Some(witness("main.py", ResolutionConfidence::Exact))];
    assert_eq!(
        result(&scoped(&f, &findings, &evidence, false)),
        result(&scoped(&f, &findings, &evidence, true))
    );
}
#[test]
fn mixed_run_results_sort_and_rule_indexes() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let findings = [
        finding("echo", "main.py"),
        finding("taint", "context.py"),
        finding("absence", "main.py"),
    ];
    let evidence = [
        None,
        Some(witness("context.py", ResolutionConfidence::Exact)),
        None,
    ];
    let d = scoped(&f, &findings, &evidence, true);
    let results = d["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    let keys: Vec<_> = results
        .iter()
        .map(|r| {
            (
                r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
                    .as_str()
                    .unwrap(),
                r["locations"][0]["physicalLocation"]["region"]["startLine"]
                    .as_u64()
                    .unwrap(),
                r["ruleId"].as_str().unwrap(),
                r["message"]["text"].as_str().unwrap(),
            )
        })
        .collect();
    assert!(keys.windows(2).all(|p| p[0] <= p[1]));
    let rules = d["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
    for r in results {
        assert_eq!(
            rules[r["ruleIndex"].as_u64().unwrap() as usize]["id"],
            r["ruleId"]
        );
    }
    context_grade(&d);
}
#[test]
fn related_and_witness_files_are_locations() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let mut sink = finding("taint", "context.py");
    sink.related_files = vec!["main.py".into(), "context.py".into(), "main.py".into()];
    let mut source = finding("taint", "main.py");
    source.category = Some("taint_source".into());
    source.related_files = vec!["context.py".into()];
    let findings = [sink, source];
    let evidence = [
        Some(witness("context.py", ResolutionConfidence::Exact)),
        Some(witness("main.py", ResolutionConfidence::Exact)),
    ];
    let d = scoped(&f, &findings, &evidence, true);
    let rs = d["runs"][0]["results"].as_array().unwrap();
    assert_eq!(rs.len(), 2);
    context_grade(&d);
    let paths = |r: &Value| {
        r["relatedLocations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                v["physicalLocation"]["artifactLocation"]["uri"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(paths(&rs[0]), ["context.py", "main.py"]);
    assert_eq!(paths(&rs[1]), ["context.py"]);
}
#[test]
fn duplicate_taint_findings_keep_multiplicity() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let a = finding("taint", "context.py");
    let d = scoped(
        &f,
        &[a.clone(), a],
        &[
            Some(witness("context.py", ResolutionConfidence::Exact)),
            Some(witness("context.py", ResolutionConfidence::Exact)),
        ],
        true,
    );
    let rs = d["runs"][0]["results"].as_array().unwrap();
    assert_eq!(rs.len(), 2);
    assert_eq!(rs[0], rs[1]);
    context_grade(&d);
}
#[test]
fn nominal_and_scoped_resolution() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let findings = [finding("taint", "context.py")];
    let evidence = [Some(witness("context.py", ResolutionConfidence::Exact))];
    let nominal = document(
        &f,
        &findings,
        &evidence,
        true,
        ResolutionMode::Nominal,
        MinConfidence::NameOnly,
    );
    let off = document(
        &f,
        &findings,
        &evidence,
        false,
        ResolutionMode::Nominal,
        MinConfidence::NameOnly,
    );
    let scope = scoped(&f, &findings, &evidence, true);
    assert_eq!(result(&nominal)["properties"]["confidence"], "unlabeled");
    assert_eq!(result(&nominal)["properties"]["tier"], "candidate");
    assert_eq!(result(&off)["properties"]["tier"], "candidate");
    context_grade(&scope);
    assert_eq!(
        result(&nominal)["partialFingerprints"],
        result(&scope)["partialFingerprints"]
    );
    assert_ne!(
        result(&nominal)["partialFingerprints"],
        result(&off)["partialFingerprints"]
    );
}
#[test]
fn existing_notifications_are_unchanged() {
    let f = Fixture::new(CONTEXT);
    assert_fixture(&f);
    let findings = [finding("taint", "context.py")];
    let evidence = [Some(witness("context.py", ResolutionConfidence::Exact))];
    let parse = ["parse warning".to_string()];
    let load = ["load warning".to_string()];
    let build = ["build warning".to_string()];
    let errors = [AlgorithmError {
        algorithm: "other".into(),
        error: "failed".into(),
    }];
    let diagnostics = ReviewBoundaryDiagnostics {
        availability: ReviewBoundaryAvailability::ListingUnavailable("denied".into()),
        omitted_count: 1,
        ..Default::default()
    };
    let make = || {
        SarifInputs::new(&findings)
            .evidence(&evidence)
            .files(&f.inputs.files)
            .sources(&f.inputs.sources)
            .parse_quality(&f.inputs.parse_quality)
            .resolution(ResolutionMode::Scoped)
            .errors(&errors)
            .parse_warnings(&parse)
            .load_warnings(&load)
            .build_warnings(&build)
            .boundary_diagnostics(&diagnostics)
    };
    let off = to_sarif(&make());
    let on = to_sarif(&make().context_maps(f.view()));
    let key = "toolExecutionNotifications";
    assert_eq!(
        off["runs"][0]["invocations"][0][key],
        on["runs"][0]["invocations"][0][key]
    );
    assert!(
        on["runs"][0]["invocations"][0][key]
            .as_array()
            .unwrap()
            .len()
            >= 5
    );
}
