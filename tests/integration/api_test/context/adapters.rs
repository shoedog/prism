//! Public adapter/ownership controls on REAL output; explicit metadata controls are labeled.
use super::*;
use prism::finding_confidence::{FindingConfidence, FindingTier};
use prism::slice::SliceResult;

fn documents(
    result: &SliceResult,
    inputs: &ReviewInputs,
    context: Option<&ReviewContext>,
) -> Vec<Value> {
    let view = ResultView::for_result(context, result, inputs);
    let review = output::to_review_output_for(result, view);
    let compact = output::to_compact_review_output_for(result, view, 0, false, false);
    let mut sarif = SarifInputs::new(&result.findings)
        .evidence(&result.evidence)
        .files(&inputs.files)
        .sources(&inputs.sources)
        .parse_quality(&inputs.parse_quality)
        .min_confidence(inputs.min_confidence)
        .resolution(inputs.resolution);
    if let Some(context) = context {
        sarif = sarif.context(context);
    }
    let mut meta = TargetsMeta::default();
    meta.min_confidence = inputs.min_confidence;
    meta.resolution = inputs.resolution;
    vec![
        value(review),
        value(compact),
        value(to_sarif(&sarif)),
        value(project_in_context(
            &result.findings,
            &result.evidence,
            inputs,
            context,
            &meta,
        )),
        value(output::format_slice_result(&result.blocks, view.sources())),
    ]
}
fn b_sarif(doc: &Value) -> &Value {
    doc["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"] == "b.js"
                && r["properties"]["category"] == "taint_sink"
        })
        .expect("behavior: serialized b.js sink")
}
fn assert_context_documents(docs: &[Value], quality: &str) {
    for d in &docs[..2] {
        assert!(d["slices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["file"] == "b.js" && s["slice_text"].as_str().unwrap().contains("eval(x)")));
        if quality == "clean" {
            assert!(d.get("parse_quality").is_none());
        } else {
            assert_eq!(d["parse_quality"]["b.js"]["quality"], quality);
        }
    }
    let finding = b_sarif(&docs[2]);
    assert_eq!(finding["properties"]["parse_quality"], quality);
    assert_eq!(finding["properties"]["confidence"], "nameonly");
    assert_eq!(finding["properties"]["tier"], "candidate");
    assert!(!finding["partialFingerprints"]["prism/finding/v1"]
        .as_str()
        .unwrap()
        .is_empty());
    assert!(docs[3]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["site"]["file"] == "b.js"
            && t["site"]["symbol"] == "g"
            && t["confidence"] == "nameonly"
            && t["tier"] == "candidate"));
    assert!(docs[4]
        .as_str()
        .unwrap()
        .contains("export function g(x) { eval(x); }"));
}

#[test]
fn a07_public_rendering_adapters() {
    let f = Fixture::new(DEGRADED);
    let outcome = f.review(&imports());
    let r = &outcome.run.results[0];
    let index = sink(r, "b.js");
    let context = outcome
        .run
        .context()
        .expect("behavior: context owned by authorized result");
    let docs = documents(r, &outcome.inputs, Some(context));
    assert_context_documents(&docs, "degraded");
    let view = ResultView::for_finding(Some(context), &r.findings[index], &outcome.inputs);
    assert_eq!(view.sources()["b.js"], DEGRADED);
    // CONTROLLED algorithm tags only: do not claim a native non-taint context-sensitive finding.
    let mut other = r.clone();
    other.algorithm = SlicingAlgorithm::AbsenceSlice;
    assert!(std::ptr::eq(
        ResultView::for_result(Some(context), &other, &outcome.inputs).sources(),
        &outcome.inputs.sources
    ));
    let mut other_finding = r.findings[index].clone();
    other_finding.algorithm = "absence".into();
    assert!(std::ptr::eq(
        ResultView::for_finding(Some(context), &other_finding, &outcome.inputs).sources(),
        &outcome.inputs.sources
    ));
    let none = documents(r, &outcome.inputs, None);
    assert_eq!(
        none[0],
        value(output::to_review_output(r, &outcome.inputs.sources))
    );
    assert_eq!(
        none[1],
        value(output::to_compact_review_output(
            r,
            &outcome.inputs.sources,
            0,
            false,
            false
        ))
    );
    let sarif = SarifInputs::new(&r.findings)
        .evidence(&r.evidence)
        .files(&outcome.inputs.files)
        .sources(&outcome.inputs.sources)
        .parse_quality(&outcome.inputs.parse_quality)
        .resolution(outcome.inputs.resolution)
        .min_confidence(outcome.inputs.min_confidence);
    assert_eq!(none[2], value(to_sarif(&sarif)));
    let mut meta = TargetsMeta::default();
    meta.resolution = outcome.inputs.resolution;
    meta.min_confidence = outcome.inputs.min_confidence;
    assert_eq!(
        none[3],
        value(project(&r.findings, &r.evidence, &outcome.inputs, &meta))
    );
    assert_eq!(
        none[4],
        value(output::format_slice_result(
            &r.blocks,
            &outcome.inputs.sources
        ))
    );
    assert_eq!(b_sarif(&none[2])["properties"]["confidence"], "nameonly");
    assert_eq!(b_sarif(&none[2])["properties"]["parse_quality"], "unknown");
    assert_eq!(
        b_sarif(&none[2])["partialFingerprints"]["prism/finding/v1"],
        output::sarif::fingerprint(&r.findings[index], "")
    );
    assert_eq!(
        b_sarif(&docs[2])["partialFingerprints"]["prism/finding/v1"],
        output::sarif::fingerprint(&r.findings[index], DEGRADED.lines().next().unwrap())
    );

    // REAL accepted canonical TS strong route: unchanged evidence confidence, quality changes tier.
    let dir = TempDir::new().unwrap();
    for (file, source) in [("app.ts", "import * as N from './missing/../x';\nimport { f } from './x';\nexport function weak(x) { N.f(x); }\nexport function strong(x) { f(x); }\n"),
        ("x.ts", "export function f(x) { eval(x); }\n"), ("other/x.ts", "export function f(x) { eval(x); }\n")] {
        let path = dir.path().join(file); fs::create_dir_all(path.parent().unwrap()).unwrap(); fs::write(path, source).unwrap();
    }
    let mut opts = ReviewOptions::new(dir.path());
    opts.no_cache = true;
    opts.resolution = ResolutionMode::Scoped;
    opts.min_confidence = MinConfidence::Exact;
    let mut p = imports();
    p.taint_sources = vec!["app.ts:4".into()];
    let strong = review(
        &opts,
        r#"{"files":[{"file_path":"app.ts","modify_type":"Modified","diff_lines":[3,4]}]}"#,
        &[SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &p,
    )
    .unwrap();
    assert_eq!(
        strong
            .inputs
            .files
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["app.ts"],
        "inadmissible precondition: original TS input scope"
    );
    let r = &strong.run.results[0];
    let i = sink(r, "x.ts");
    let context = strong
        .run
        .context()
        .expect("behavior: canonical TS context");
    let with = documents(r, &strong.inputs, Some(context));
    let without = documents(r, &strong.inputs, None);
    let entry = |d: &Value| {
        d["runs"][0]["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"] == "x.ts")
            .cloned()
            .unwrap()
    };
    assert_eq!(entry(&with[2])["properties"]["confidence"], "exact");
    assert_eq!(entry(&without[2])["properties"]["confidence"], "exact");
    assert_eq!(entry(&with[2])["properties"]["tier"], "asserted");
    assert_eq!(entry(&without[2])["properties"]["tier"], "candidate");
    let grade = prism::finding_confidence::classify_with_evidence(
        "taint",
        prism::finding_confidence::ParseQuality::Clean,
        r.evidence[i].as_ref().unwrap(),
    );
    assert_eq!(grade, (FindingConfidence::Exact, FindingTier::Asserted));
}

#[test]
fn a08_take_context_then_views() {
    let f = Fixture::new(B);
    let mut outcome = f.review(&imports());
    let before = documents(
        &outcome.run.results[0],
        &outcome.inputs,
        outcome.run.context(),
    );
    let context = outcome
        .run
        .take_context()
        .expect("behavior: eligible context is moved out");
    assert!(outcome.run.context().is_none());
    assert!(outcome.run.take_context().is_none());
    // The public owner can now be consumed field by field, as the future binary requires.
    let results = outcome.run.results;
    let findings = outcome.run.findings;
    let evidence = outcome.run.evidence;
    let warnings = outcome.run.warnings;
    assert!(!findings.is_empty());
    assert_eq!(findings.len(), evidence.len());
    assert_eq!(
        documents(&results[0], &outcome.inputs, Some(&context)),
        before
    );
    assert_context_documents(&before, "clean");
    assert!(warnings.iter().all(|w| !w.contains("ignored")));
    let mut off = f.review(&AlgorithmParams::default());
    assert!(off.run.take_context().is_none());
    assert!(off.run.take_context().is_none());
    let (dir, options, inputs) = unavailable_inputs();
    let built = build_context(&inputs, &options).unwrap();
    let mut unavailable = run_review_authorized(
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &[SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &imports(),
        dir.path(),
    );
    assert!(unavailable.context().is_none());
    assert!(unavailable.take_context().is_none());
    assert!(unavailable.take_context().is_none());
    assert!(unavailable.findings.is_empty());
}

#[test]
fn a09_single_run_filter_through_the_public_type() {
    let f = Fixture::new(DEGRADED);
    let inputs = f.inputs();
    let built = build_context(&inputs, &f.options).unwrap();
    let run = || {
        run_algorithm_authorized(
            SlicingAlgorithm::Taint,
            &built.ctx,
            ReviewAuthority::capture(&inputs),
            &SliceConfig::default(),
            &imports(),
            f.dir.path(),
        )
        .unwrap()
    };
    let mut single = run();
    single.filter_findings(&inputs, MinConfidence::NameOnly, ResolutionMode::Scoped);
    let expected = single_outcome_result(&f);
    let before = value(output::to_review_output_for(
        &expected,
        single.view(&inputs),
    ));
    let (outcome, context) = single.into_parts();
    sink(&outcome.result, "b.js");
    pair(&outcome.result, &expected);
    assert!(context.is_some());
    assert_eq!(outcome.result.findings.len(), outcome.result.evidence.len());
    assert_eq!(
        value(output::to_review_output_for(
            &outcome.result,
            ResultView::for_result(context.as_ref(), &outcome.result, &inputs)
        )),
        before
    );
    let mut rejected = run();
    rejected.filter_findings(&inputs, MinConfidence::Exact, ResolutionMode::Scoped);
    assert!(
        rejected.context().is_some(),
        "filtering does not revoke eligible maps"
    );
    let (outcome, context) = rejected.into_parts();
    assert!(context.is_some());
    assert!(!outcome
        .result
        .findings
        .iter()
        .any(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink")));
    assert_eq!(outcome.result.findings.len(), outcome.result.evidence.len());
    let off = run_algorithm_authorized(
        SlicingAlgorithm::Taint,
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        f.dir.path(),
    )
    .unwrap();
    assert!(off.into_parts().1.is_none());
    let (dir, options, inputs) = unavailable_inputs();
    let built = build_context(&inputs, &options).unwrap();
    let unavailable = run_algorithm_authorized(
        SlicingAlgorithm::Taint,
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &SliceConfig::default(),
        &imports(),
        dir.path(),
    )
    .unwrap();
    assert!(unavailable.context().is_none());
    let (outcome, context) = unavailable.into_parts();
    assert!(context.is_none());
    assert!(outcome.result.findings.is_empty());
}
fn single_outcome_result(f: &Fixture) -> SliceResult {
    // Separate REAL call only supplies the public outcome; no private result accessor is exposed.
    let inputs = f.inputs();
    let built = build_context(&inputs, &f.options).unwrap();
    run_algorithm_authorized(
        SlicingAlgorithm::Taint,
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &SliceConfig::default(),
        &imports(),
        f.dir.path(),
    )
    .unwrap()
    .into_parts()
    .0
    .result
}
