//! REAL driver attempts: independent producer evidence guards precede behavioral assertions.
use super::super::super::super::source_scope::admission::{slice_in_context, ScopedTaintOutcome};
use super::*;
use prism::api::{ContextReadOutcome, ReviewContextRound};
use prism::cpg::TaintCrossingPolicy;

fn union_analysis(r: &ExpansionRequest<'_>, path: &str) -> ScopedTaintOutcome {
    let mut files = r.inputs.files.clone();
    let file = match r.loader.read(path, 1 << 20).outcome {
        ContextReadOutcome::Loaded(file) => file,
        _ => panic!("inadmissible precondition: context parse"),
    };
    files.insert(file.path, file.parsed);
    let round = ReviewContextRound::new(r.loader, &files);
    let ctx = round.build_context(r.inputs.type_db.as_ref(), &r.baseline_ctx.types);
    let original = r.inputs.files.keys().cloned().collect();
    let context = [path.to_string()].into_iter().collect();
    let policy = TaintCrossingPolicy::new(&context, r.ceiling);
    slice_in_context(&ctx, &r.inputs.diff, r.config, &original, &policy).unwrap()
}
fn selected_evidence<'a>(
    analysis: &'a ScopedTaintOutcome,
    path: &str,
) -> Option<&'a prism::finding_confidence::EvidencePath> {
    analysis
        .result
        .findings
        .iter()
        .enumerate()
        .find(|(_, f)| f.file == path && f.category.as_deref() == Some("taint_sink"))
        .and_then(|(i, _)| analysis.result.evidence.get(i).and_then(Option::as_ref))
}
#[test]
fn m06_actual_stage_ceiling_weak_and_exact_routes() {
    use prism::cpg::{FlowConfidence, FlowDoubt};
    use prism::data_flow::VarAccessKind;
    use prism::finding_confidence::{EvidenceHop, EvidencePath};

    // Each edge is the physical argument occurrence entering the same f parameter.
    fn entry(evidence: &EvidencePath, weak: bool) -> bool {
        let (caller, line, start, confidence) = if weak {
            (
                "weak",
                3,
                92,
                FlowConfidence::NameOnly(FlowDoubt::CallNameOnly),
            )
        } else {
            ("strong", 4, 128, FlowConfidence::Exact)
        };
        evidence.hops.iter().any(|hop| match hop {
            EvidenceHop::DataFlow {
                from,
                to,
                confidence: actual,
            } => {
                from.file == "app.ts"
                    && from.function == caller
                    && from.line == line
                    && from.start_byte == start
                    && from.end_byte == start + 1
                    && from.kind == VarAccessKind::Use
                    && to.file == "x.ts"
                    && to.function == "f"
                    && to.line == 1
                    && to.start_byte == 18
                    && to.end_byte == 19
                    && to.kind == VarAccessKind::Def
                    && *actual == confidence
            }
            _ => false,
        })
    }
    fn target_evidence(analysis: &ScopedTaintOutcome) -> Option<&EvidencePath> {
        analysis
            .result
            .findings
            .iter()
            .enumerate()
            .find(|(_, finding)| {
                finding.file == "x.ts"
                    && finding.line == 1
                    && finding.category.as_deref() == Some("taint_sink")
            })
            .and_then(|(index, _)| analysis.result.evidence.get(index).and_then(Option::as_ref))
    }
    let main = "import * as N from './missing/../x';\nimport { f } from './x';\nexport function weak(x) { N.f(x); }\nexport function strong(x) { f(x); }\n";
    let f = named(
        &[
            ("app.ts", main),
            ("x.ts", "export function f(x) { eval(x); }\n"),
            ("other/x.ts", "export function f(x) { eval(x); }\n"),
        ],
        "app.ts",
        &[3, 4],
    );

    // Independent native reader-admitted union proves each selected entry, before
    // the actual driver ceiling rows exercise the policy.
    for (line, weak) in [(3, true), (4, false)] {
        let config = TaintConfig {
            sources: vec![("app.ts".into(), line)],
            taint_from_diff: false,
            ..Default::default()
        };
        variant(
            &f,
            &config,
            limits(2, 8),
            CrossingCeiling::NameOnly,
            &Guard::never(),
            |r| {
                assert_eq!(
                    first_plan(r).load_order,
                    ["x.ts"],
                    "inadmissible precondition: actual target hint"
                );
                let proof = union_analysis(r, "x.ts");
                let evidence = target_evidence(&proof)
                    .expect("inadmissible precondition: real aligned target sink evidence");
                assert!(!evidence.hops.is_empty() && !evidence.crossed_unlabeled && entry(evidence, weak),
                "inadmissible precondition: selected source reaches target by its physical labeled entry");
                assert!(proof.result.findings.iter().any(|x| x.file == "x.ts"
                    && x.line == 1
                    && x.category.as_deref() == Some("taint_sink")));
            },
        );
    }
    for lines in [&[3][..], &[4][..], &[3, 4][..]] {
        let config = TaintConfig {
            sources: lines.iter().map(|line| ("app.ts".into(), *line)).collect(),
            taint_from_diff: false,
            ..Default::default()
        };
        for ceiling in [CrossingCeiling::NameOnly, CrossingCeiling::Exact] {
            variant(&f, &config, limits(2, 8), ceiling, &Guard::never(), |r| {
                let out = run_owned(r);
                assert_eq!(
                    out.bundle.context_files,
                    ["x.ts".to_string()].into_iter().collect()
                );
                assert!(!out.bundle.files.contains_key("other/x.ts"));
                let analysis = out
                    .bundle
                    .analysis
                    .as_ref()
                    .expect("completed target analysis");
                let has_sink = analysis.result.findings.iter().any(|finding| {
                    finding.file == "x.ts"
                        && finding.line == 1
                        && finding.category.as_deref() == Some("taint_sink")
                });
                let expected = lines != [3] || ceiling == CrossingCeiling::NameOnly;
                assert_eq!(has_sink, expected);
                let found = target_evidence(analysis);
                assert_eq!(
                    found.is_some(),
                    expected,
                    "target sink evidence must be aligned"
                );
                if let Some(evidence) = found {
                    assert!(!evidence.hops.is_empty() && !evidence.crossed_unlabeled);
                    let valid_weak = lines.contains(&3)
                        && ceiling == CrossingCeiling::NameOnly
                        && entry(evidence, true);
                    let valid_strong = lines.contains(&4) && entry(evidence, false);
                    assert!(
                        valid_weak || valid_strong,
                        "sink witness must use a selected physical context entry"
                    );
                    assert!(analysis.result.findings.iter().any(|x| x.file == "x.ts"
                        && x.line == 1
                        && x.category.as_deref() == Some("taint_sink")));
                }
            });
        }
    }
}

#[test]
fn m08_return_flow_through_actual_driver_both_ceilings() {
    let main = "from a import helper\ndef run(user):\n    value = helper(user)\n    sink(value)\n";
    let f = named(
        &[
            ("main.py", main),
            ("a.py", "def helper(p):\n    q = p\n    return q\n"),
        ],
        "main.py",
        &[3],
    );
    let config = TaintConfig {
        sources: vec![("main.py".into(), 2)],
        taint_from_diff: false,
        return_flow: true,
        extra_sinks: vec!["=sink".into()],
        ..Default::default()
    };
    for ceiling in [CrossingCeiling::NameOnly, CrossingCeiling::Exact] {
        variant(&f, &config, limits(2, 8), ceiling, &Guard::never(), |r| {
            assert_eq!(
                first_plan(r).load_order,
                ["a.py"],
                "inadmissible precondition: helper eligible"
            );
            let proof = union_analysis(r, "a.py");
            let evidence = selected_evidence(&proof, "main.py")
                .expect("inadmissible precondition: real return sink with evidence");
            assert!(
                !evidence.hops.is_empty(),
                "inadmissible precondition: nonempty return witness"
            );
            let off = TaintConfig {
                return_flow: false,
                ..config.clone()
            };
            variant(&f, &off, limits(2, 8), ceiling, &Guard::never(), |other| {
                let negative = union_analysis(other, "a.py");
                assert!(
                    selected_evidence(&negative, "main.py").is_none(),
                    "inadmissible precondition: return-flow route independently distinguished"
                );
            });
            let out = run_owned(r);
            assert!(out.bundle.context_files.contains("a.py"));
            assert!(!sinks(&out, "main.py").is_empty());
        });
    }
}
