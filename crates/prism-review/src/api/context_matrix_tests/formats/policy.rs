//! F15-F20: REAL entry routes; CONTROLLED negative evidence and serializer-only language subrows are labeled.
use super::*;
use prism::finding_confidence::{EvidenceHop, EvidencePath};

fn ceiling_fixture() -> Fixture {
    Fixture::new(&[
        ("app.ts","import * as N from './missing/../x';\nimport { f } from './x';\nexport function weak(x) { N.f(x); }\nexport function strong(x) { f(x); }\n"),
        ("x.ts","export function f(x) { eval(x); }\n"),
        ("other/x.ts","export function f(x) { eval(x); }\n"),
    ],"app.ts",&[3,4])
}
fn rooted(lines: &[usize], edges: TaintContextEdges) -> AlgorithmParams {
    let mut p = imports();
    p.taint_context.edges = edges;
    p.taint_sources = lines.iter().map(|n| format!("app.ts:{n}")).collect();
    p
}
fn has_target(sample: &Sample, file: &str) -> bool {
    sample
        .run
        .findings
        .iter()
        .any(|f| f.file == file && f.category.as_deref() == Some("taint_sink"))
}
fn physical_entry(sample: &Sample, weak: bool) {
    use prism::cpg::{FlowConfidence, FlowDoubt};
    let result = &sample.run.results[0];
    let index = result
        .findings
        .iter()
        .position(|f| f.file == "x.ts" && f.category.as_deref() == Some("taint_sink"))
        .unwrap();
    let evidence = result.evidence[index].as_ref().unwrap();
    assert!(
        evidence.hops.iter().any(|h| match h {
            EvidenceHop::DataFlow {
                from,
                to,
                confidence,
            } =>
                from.file == "app.ts"
                    && from.function == if weak { "weak" } else { "strong" }
                    && from.line == if weak { 3 } else { 4 }
                    && to.file == "x.ts"
                    && to.function == "f"
                    && to.line == 1
                    && *confidence
                        == if weak {
                            FlowConfidence::NameOnly(FlowDoubt::CallNameOnly)
                        } else {
                            FlowConfidence::Exact
                        },
            _ => false,
        }),
        "actual selected producer pair has the expected physical weak/strong entry label"
    );
}

#[test]
fn f15_nominal_and_scoped_formats_keep_intentional_parity() {
    let mut f = Fixture::js(B);
    f.policy(ResolutionMode::Scoped, MinConfidence::NameOnly);
    let scoped = f.on(&taint());
    assert_expanded(&scoped, "b.js");
    let s = documents(&f, &scoped);
    assert_eq!(
        context_sarif(&s.sarif, "b.js")["properties"]["parse_quality"],
        "clean"
    );
    f.policy(ResolutionMode::Nominal, MinConfidence::NameOnly);
    let nominal = f.on(&taint());
    assert_expanded(&nominal, "b.js");
    let n = documents(&f, &nominal);
    same_result(&scoped.run.results[0], &nominal.run.results[0]);
    assert_eq!(
        s.json.bytes, n.json.bytes,
        "legacy JSON does not acquire emitter grades"
    );
    let sr = context_sarif(&s.sarif, "b.js");
    let nr = context_sarif(&n.sarif, "b.js");
    assert_eq!(sr["partialFingerprints"], nr["partialFingerprints"]);
    assert_eq!(nr["properties"]["confidence"], "unlabeled");
    assert_eq!(nr["properties"]["tier"], "candidate");
    assert_eq!(nr["properties"]["resolution_mode"], "nominal");
    assert_eq!(
        context_target(&n.targets, "b.js")["confidence"],
        "unlabeled"
    );
    assert_eq!(context_target(&n.targets, "b.js")["tier"], "candidate");
    assert_eq!(n.targets.value["producer"]["resolution_mode"], "nominal");
}

#[test]
fn f16_exact_floor_view_tier_parity_and_real_weak_negative() {
    let mut f = ceiling_fixture();
    f.policy(ResolutionMode::Scoped, MinConfidence::Exact);
    let strong = f.run(&taint(), &rooted(&[4], TaintContextEdges::NameOnly));
    assert_expanded(&strong, "x.ts");
    physical_entry(&strong, false);
    let doc = documents(&f, &strong);
    let r = context_sarif(&doc.sarif, "x.ts");
    assert_eq!(r["properties"]["confidence"], "exact");
    assert_eq!(r["properties"]["parse_quality"], "clean");
    assert_eq!(r["properties"]["tier"], "asserted");
    assert_eq!(context_target(&doc.targets, "x.ts")["tier"], "asserted");
    assert!(doc.json.value["results"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["file"] == "x.ts"));
    assert!(doc.review.value["results"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["file"] == "x.ts"));
    let original = output::to_sarif(
        &output::SarifInputs::new(&strong.run.findings)
            .evidence(&strong.run.evidence)
            .files(&f.inputs.files)
            .sources(&f.inputs.sources)
            .parse_quality(&f.inputs.parse_quality)
            .min_confidence(MinConfidence::Exact)
            .resolution(ResolutionMode::Scoped),
    );
    let original = Document::new(&original);
    let old = context_sarif(&original, "x.ts");
    assert_eq!(
        old["properties"]["confidence"], r["properties"]["confidence"],
        "quality does not change confidence/floor admission"
    );
    assert_eq!(old["properties"]["parse_quality"], "unknown");
    assert_eq!(old["properties"]["tier"], "candidate");
    f.policy(ResolutionMode::Scoped, MinConfidence::NameOnly);
    let weak = f.run(&taint(), &rooted(&[3], TaintContextEdges::NameOnly));
    assert_expanded(&weak, "x.ts");
    physical_entry(&weak, true);
    f.policy(ResolutionMode::Scoped, MinConfidence::Exact);
    let rejected = f.run(&taint(), &rooted(&[3], TaintContextEdges::NameOnly));
    assert!(!has_target(&rejected, "x.ts"));
    assert!(
        !documents(&f, &rejected).json.value["results"][0]["findings"]
            .as_array()
            .is_some_and(|fs| fs
                .iter()
                .any(|f| f["file"] == "x.ts" && f["category"] == "taint_sink"))
    );
    // CONTROLLED unlabeled evidence on real engine findings; no invented graph edge/hop.
    let mut controlled = strong;
    for evidence in &mut controlled.run.evidence {
        *evidence = Some(EvidencePath::unlabeled_for("taint"));
    }
    let rejected = documents(&f, &controlled);
    assert!(results(&rejected.sarif).is_empty());
    assert!(rejected.targets.value["targets"]
        .as_array()
        .unwrap()
        .is_empty());
    let mut dirty = Fixture::js(DEGRADED);
    dirty.policy(ResolutionMode::Scoped, MinConfidence::NameOnly);
    let sample = dirty.on(&taint());
    assert_expanded(&sample, "b.js");
    let doc = documents(&dirty, &sample);
    let notes = sample.run.diagnostics.as_ref().unwrap().run_notes();
    assert!(notes.iter().all(|n| n.kind()
        != crate::algorithms::taint::boundary::records::ContextRunNoteKind::ContextParse));
    let graded = context_sarif(&doc.sarif, "b.js");
    assert_eq!(graded["properties"]["parse_quality"], "degraded");
    assert_eq!(graded["properties"]["confidence"], "nameonly");
    assert_eq!(graded["properties"]["tier"], "candidate");
    dirty.policy(ResolutionMode::Scoped, MinConfidence::Exact);
    let rejected = dirty.on(&taint());
    let crate::api::context_run::ContextState::Expanded(context) = &rejected.run.context else {
        panic!("Exact floor must preserve actual expanded context");
    };
    assert!(context.context_files.contains("b.js"));
    assert_eq!(context.parse_quality["b.js"].quality, "degraded");
    assert!(rejected
        .run
        .diagnostics
        .as_ref()
        .unwrap()
        .run_notes()
        .iter()
        .all(|n| n.kind()
            != crate::algorithms::taint::boundary::records::ContextRunNoteKind::ContextParse));
    assert!(!has_target(&rejected, "b.js"));
    assert!(rejected.run.results.iter().all(|r| !r
        .findings
        .iter()
        .any(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink"))));
    let rejected_doc = documents(&dirty, &rejected);
    assert!(!rejected_doc.json.value["results"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["file"] == "b.js" && f["category"] == "taint_sink"));
    assert!(!results(&rejected_doc.sarif)
        .iter()
        .any(
            |r| r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"] == "b.js"
                && r["properties"]["category"] == "taint_sink"
        ));
    assert!(!rejected_doc.targets.value["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["site"]["file"] == "b.js" && t["category"] == "taint_sink"));
    let mut bound = imports();
    bound.taint_context.depth = 1;
    let bounded = dirty.run(&taint(), &bound);
    assert_depth_bound_once(&bounded, &documents(&dirty, &bounded));
}

#[test]
fn f17_actual_weak_and_exact_roots_to_same_sink_both_ceilings() {
    let f = ceiling_fixture();
    for lines in [&[3][..], &[4][..], &[3, 4][..]] {
        for edges in [TaintContextEdges::NameOnly, TaintContextEdges::Exact] {
            let sample = f.run(&taint(), &rooted(lines, edges));
            let context = match &sample.run.context {
                crate::api::context_run::ContextState::Expanded(c) => c,
                _ => panic!("actual target must be loaded"),
            };
            assert!(context.context_files.contains("x.ts"));
            assert!(!context.files.contains_key("other/x.ts"));
            let expected = lines != &[3][..] || edges == TaintContextEdges::NameOnly;
            assert_eq!(has_target(&sample, "x.ts"), expected);
            let doc = documents(&f, &sample);
            assert_eq!(
                results(&doc.sarif)
                    .iter()
                    .any(
                        |r| r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
                            == "x.ts"
                            && r["properties"]["category"] == "taint_sink"
                    ),
                expected
            );
            if expected && lines == &[3][..] {
                physical_entry(&sample, true);
            }
            if expected && lines == &[4][..] {
                physical_entry(&sample, false);
            }
        }
    }
}

#[test]
fn f18_single_annotated_multi_raw_and_aggregate_quality() {
    let f = Fixture::js(DEGRADED);
    let multi = f.on(&taint());
    assert_expanded(&multi, "b.js");
    let single = f.single(&imports());
    let context = match &multi.run.context {
        crate::api::context_run::ContextState::Expanded(c) => c,
        _ => unreachable!(),
    };
    let expected = context.parse_quality["b.js"].quality.as_str();
    let raw = multi.run.results[0]
        .findings
        .iter()
        .find(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink"))
        .unwrap();
    assert!(raw.parse_quality.is_none());
    let aggregate = multi
        .run
        .findings
        .iter()
        .find(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink"))
        .unwrap();
    let one = single
        .run
        .outcome
        .result
        .findings
        .iter()
        .find(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink"))
        .unwrap();
    assert_eq!(aggregate.parse_quality.as_deref(), Some(expected));
    assert_eq!(one.parse_quality.as_deref(), Some(expected));
    let mut unannotated = single.run.outcome.result.clone();
    for finding in &mut unannotated.findings {
        finding.parse_quality = None;
    }
    same_result(&unannotated, &multi.run.results[0]);
    let one = Documents::single(&single, &f.inputs, RenderSettings::default());
    let many = documents(&f, &multi);
    assert_eq!(
        one.json.value["parse_quality"],
        many.json.value["results"][0]["parse_quality"]
    );
    assert!(one.json.value["parse_quality"].get("b.js").is_some());
    assert_quality(&many.json.value, &f.inputs.parse_quality);
}

#[test]
fn f19_duplicate_pairs_aggregate_attempts_and_notes_once() {
    let f = Fixture::js(DEGRADED);
    let one = f.on(&taint());
    let two = f.on(&[SlicingAlgorithm::Taint, SlicingAlgorithm::Taint]);
    assert_expanded(&one, "b.js");
    assert_expanded(&two, "b.js");
    assert_eq!(two.run.results.len(), 2);
    same_result(&one.run.results[0], &two.run.results[0]);
    same_result(&two.run.results[0], &two.run.results[1]);
    let mut findings = one.run.findings.clone();
    findings.extend(one.run.findings.clone());
    assert_eq!(bytes(&two.run.findings), bytes(&findings));
    let mut evidence = one.run.evidence.clone();
    evidence.extend(one.run.evidence.clone());
    assert_eq!(format!("{:?}", two.run.evidence), format!("{evidence:?}"));
    let (a, b) = match (&one.run.context, &two.run.context) {
        (
            crate::api::context_run::ContextState::Expanded(a),
            crate::api::context_run::ContextState::Expanded(b),
        ) => (&a.attempts, &b.attempts),
        _ => panic!("actual expanded attempt facts"),
    };
    assert!(a.attempted_files > 0);
    assert_eq!(a.attempted_files, b.attempted_files);
    assert_eq!(a.attempted_read_bytes, b.attempted_read_bytes);
    assert_eq!(a.skips, b.skips);
    assert_eq!(
        one.run.diagnostics.as_ref().unwrap().messages(),
        two.run.diagnostics.as_ref().unwrap().messages()
    );
    let doc = documents(&f, &two);
    let notes = two.run.diagnostics.as_ref().unwrap().run_notes();
    assert!(
        notes.is_empty(),
        "this native 2.7% grade emits no parse warning"
    );
    assert!(notices(&doc.sarif.value).is_empty());
    assert_eq!(one.run.warnings, two.run.warnings);
    // Retained attempt equality is not independent engine-call instrumentation.
    let mut bound = imports();
    bound.taint_context.depth = 1;
    let one_bound = f.run(&taint(), &bound);
    let two_bound = f.run(&[SlicingAlgorithm::Taint, SlicingAlgorithm::Taint], &bound);
    assert_depth_bound_once(&one_bound, &documents(&f, &one_bound));
    assert_depth_bound_once(&two_bound, &documents(&f, &two_bound));
    assert_eq!(
        one_bound.run.diagnostics.as_ref().unwrap().messages(),
        two_bound.run.diagnostics.as_ref().unwrap().messages()
    );
    assert_eq!(one_bound.run.warnings, two_bound.run.warnings);
}

/// CONTROLLED sink/result over REAL admitted Go maps; no native Go sink/flow claim.
fn controlled_go_sink(sample: &mut Sample) {
    use prism::diff::{DiffBlock, ModifyType};
    let finding = prism::slice::SliceFinding {
        algorithm: "taint".into(),
        file: "svc/a.go".into(),
        line: 2,
        severity: "warning".into(),
        description: "CONTROLLED serializer sink; native Go context-sink reach remains UNVERIFIED"
            .into(),
        function_name: Some("H".into()),
        related_lines: vec![],
        related_files: vec![],
        category: Some("taint_sink".into()),
        parse_quality: None,
        diagrams: vec![],
    };
    let mut result = SliceResult::new(SlicingAlgorithm::Taint);
    let mut block = DiffBlock::new(17, "svc/a.go".into(), ModifyType::Modified);
    block.add_line("svc/a.go", 2, true);
    result.blocks.push(block);
    result.findings.push(finding);
    result
        .evidence
        .push(Some(EvidencePath::unlabeled_for("taint")));
    sample.run.findings = result.findings.clone();
    sample.run.evidence = result.evidence.clone();
    sample.run.results = vec![result];
}

#[test]
fn f20_python_native_go_admission_and_controlled_serializers() {
    let py=Fixture::new(&[("main.py","from a import helper\ndef lookup_view(request):\n    q = request.GET.get('q')\n    helper(q)\n"),("a.py","import os\ndef helper(p):\n    os.system(p)\n")],"main.py",&[4]);
    let on = py.on(&taint());
    assert_expanded(&on, "a.py");
    let doc = documents(&py, &on);
    assert_eq!(
        context_sarif(&doc.sarif, "a.py")["properties"]["parse_quality"],
        "clean"
    );
    assert!(
        doc.json.value["results"][0]["slices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["file"] == "a.py"
                && s["slice_text"].as_str().unwrap().contains("os.system(p)"))
    );
    // CONTROLLED original-anchor/U-contract serializer subrow. O has no extra_sinks option;
    // this is not proof of native routed M08 return-flow or an injected graph edge.
    let (negative, dropped) =
        super::evidence::controlled_original_anchor(&py, on, "a.py", "main.py", 4, false);
    assert_eq!(
        (
            dropped.additional_sinks,
            dropped.additional_sources,
            dropped.dropped_sinks,
            dropped.dropped_sources
        ),
        (0, 0, 1, 1)
    );
    assert!(
        negative.run.findings.is_empty(),
        "moving this selected empty witness loses context membership"
    );
    assert!(documents(&py, &negative).json.value["all_findings"]
        .as_array()
        .unwrap()
        .is_empty());
    let native = py.on(&taint());
    let (controlled, retained) =
        super::evidence::controlled_original_anchor(&py, native, "a.py", "main.py", 4, true);
    assert_eq!(
        (
            retained.additional_sinks,
            retained.additional_sources,
            retained.dropped_sinks,
            retained.dropped_sources
        ),
        (1, 1, 0, 0)
    );
    let doc = documents(&py, &controlled);
    assert!(doc.json.value["all_findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["description"]
            .as_str()
            .is_some_and(|d| d.contains("witness passes through taint context:"))));
    assert!(results(&doc.sarif).iter().any(|r| r["message"]["text"]
        .as_str()
        .is_some_and(|d| d.contains("witness passes through taint context:"))));
    let go = Fixture::new(
        &[
            ("go.mod", "module example.com/app\n"),
            (
                "main.go",
                "package main\nimport \"example.com/app/svc\"\nfunc main1(x string) { svc.H(x) }\n",
            ),
            ("svc/a.go", "package svc\nfunc H(x string) {}\n"),
            ("svc/b.go", "package svc\nfunc B() {}\n"),
        ],
        "main.go",
        &[3],
    );
    let off = go.off(&taint());
    let on = go.on(&taint());
    let context = match &on.run.context {
        crate::api::context_run::ContextState::Expanded(c) => c,
        _ => panic!("native Go declaring-file admission"),
    };
    assert_eq!(
        context
            .context_files
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["svc/a.go"]
    );
    assert!(!context.files.contains_key("svc/b.go"));
    assert!(on.run.findings.is_empty());
    assert!(off.run.findings.is_empty());
    assert_eq!(
        documents(&go, &off).json.value["results"][0]["findings"],
        documents(&go, &on).json.value["results"][0]["findings"]
    );
    assert!(results(&documents(&go, &on).sarif).is_empty());
    let mut controlled = on;
    controlled_go_sink(&mut controlled);
    let doc = documents(&go, &controlled);
    assert!(doc.json.value["results"][0]["slices"][0]["slice_text"]
        .as_str()
        .unwrap()
        .contains("func H(x string) {}"));
    assert_eq!(
        context_sarif(&doc.sarif, "svc/a.go")["properties"]["confidence"],
        "unlabeled"
    );
    eprintln!("F20 REAL Python Django sink; REAL Go admission/emptyfinding parity; CONTROLLED original-anchor and Go serializer sinks; native Go sink and native routed return-flow UNVERIFIED");
}
