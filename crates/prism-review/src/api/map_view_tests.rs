//! REAL parsing/loading, CONTROLLED union-map assembly and finding/evidence pairs.
//! These adapters do not run the expansion engine or discover taint flow.

use super::MapView;
use crate::api::run::filter_result_findings_in;
use crate::api::{filter_result_findings, load_review_inputs, ReviewInputs, ReviewOptions};
use crate::targets::{project, project_with_context, TargetsMeta};
use prism::ast::{parse_quality::check_parse_quality, ParsedFile};
use prism::call_graph::FunctionId;
use prism::finding_confidence::{
    parse_quality_for_selected_evidence, EvidenceHop, EvidencePath, FindingConfidence, FindingTier,
    MinConfidence, ParseQuality, ResolutionMode,
};
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolvedCallEdge};
use prism::slice::{FileParseQuality, SliceFinding, SliceResult, SlicingAlgorithm};
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

pub(crate) const CONTEXT: &str = "def contextual(value):\n    first = value\n    return first\n";
const MAIN: &str = "import requests\ndef baseline(value):\n    return requests.get(value)\n";

pub(crate) struct Fixture {
    pub(crate) repo: TempDir,
    pub(crate) inputs: ReviewInputs,
    pub(crate) files: BTreeMap<String, ParsedFile>,
    pub(crate) sources: BTreeMap<String, String>,
    pub(crate) quality: BTreeMap<String, FileParseQuality>,
}

impl Fixture {
    pub(crate) fn new(context_source: &str) -> Self {
        let repo = TempDir::new().unwrap();
        fs::write(repo.path().join("main.py"), MAIN).unwrap();
        let mut options = ReviewOptions::new(repo.path());
        options.no_cache = true;
        let inputs = load_review_inputs(
            &options,
            r#"{"files":[{"file_path":"main.py","modify_type":"Modified","diff_lines":[3]}]}"#,
        )
        .unwrap();
        assert!(
            inputs.files.contains_key("main.py") && !inputs.files.contains_key("context.py"),
            "inadmissible precondition: baseline must parse main.py without context.py"
        );
        assert!(
            inputs.files["main.py"].error_rate() <= 0.01,
            "inadmissible precondition: original file must parse cleanly"
        );
        let mut files = inputs.files.clone();
        files.insert(
            "context.py".into(),
            ParsedFile::parse("context.py", context_source, Language::Python).unwrap(),
        );
        let mut sources = inputs.sources.clone();
        sources.insert("context.py".into(), context_source.into());
        let (_, quality) = check_parse_quality(&files);
        Self {
            repo,
            inputs,
            files,
            sources,
            quality,
        }
    }

    pub(crate) fn view(&self) -> MapView<'_> {
        MapView::new(&self.files, &self.sources, &self.quality)
    }

    pub(crate) fn meta(&self) -> TargetsMeta {
        let mut meta = TargetsMeta::default();
        meta.repo_root = self.repo.path().to_path_buf();
        meta.resolution = ResolutionMode::Scoped;
        meta
    }
}

pub(crate) fn finding(algorithm: &str, file: &str) -> SliceFinding {
    SliceFinding {
        algorithm: algorithm.into(),
        file: file.into(),
        line: 3,
        severity: "warning".into(),
        description: "controlled finding".into(),
        function_name: None,
        related_lines: vec![],
        related_files: vec![],
        category: Some("taint_sink".into()),
        parse_quality: None,
        diagrams: vec![],
    }
}

pub(crate) fn witness(file: &str, confidence: ResolutionConfidence) -> EvidencePath {
    EvidencePath::from_hops(vec![EvidenceHop::Call {
        edge: ResolvedCallEdge {
            caller: FunctionId {
                file: file.into(),
                name: "controlled_caller".into(),
                start_line: 1,
                end_line: 3,
            },
            call_site_line: 3,
            confidence,
            kind: ResolutionKind::FreeSingle,
        },
        confidence,
    }])
}

pub(crate) fn result(
    findings: Vec<SliceFinding>,
    evidence: Vec<Option<EvidencePath>>,
) -> SliceResult {
    let mut result = SliceResult::new(SlicingAlgorithm::Taint);
    result.findings = findings;
    result.evidence = evidence;
    result
}

pub(crate) fn bytes(value: &impl serde::Serialize) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

#[test]
fn v01_wrappers_equal_the_view_forms() {
    let fixture = Fixture::new(CONTEXT);
    let original = MapView::original(&fixture.inputs);
    assert!(std::ptr::eq(original.files, &fixture.inputs.files));
    assert!(std::ptr::eq(original.sources, &fixture.inputs.sources));
    assert!(std::ptr::eq(
        original.parse_quality,
        &fixture.inputs.parse_quality
    ));
    let findings = vec![finding("taint", "main.py"), finding("echo", "main.py")];
    let evidence = vec![Some(witness("main.py", ResolutionConfidence::Exact)), None];
    for floor in [MinConfidence::NameOnly, MinConfidence::Exact] {
        let mut wrapped = result(findings.clone(), evidence.clone());
        let mut viewed = result(findings.clone(), evidence.clone());
        filter_result_findings(&mut wrapped, &fixture.inputs, floor, ResolutionMode::Scoped);
        filter_result_findings_in(&mut viewed, original, floor, ResolutionMode::Scoped);
        assert_eq!(bytes(&wrapped), bytes(&viewed));
        assert_eq!(
            format!("{:?}", wrapped.evidence),
            format!("{:?}", viewed.evidence)
        );
        assert_eq!(
            wrapped.findings.len(),
            if floor == MinConfidence::Exact { 1 } else { 2 }
        );
    }
    let mut meta = fixture.meta();
    meta.run_warnings.push("ordinary warning".into());
    assert_eq!(
        bytes(&project(&findings, &evidence, &fixture.inputs, &meta)),
        bytes(&project_with_context(
            &findings,
            &evidence,
            &fixture.inputs,
            None,
            &meta
        ))
    );
}

#[test]
fn v02_clean_context_witness_survives_an_exact_floor() {
    let fixture = Fixture::new(CONTEXT);
    let finding = finding("taint", "context.py");
    let evidence = witness("context.py", ResolutionConfidence::Exact);
    assert_eq!(
        parse_quality_for_selected_evidence(
            &finding,
            Some(&evidence),
            &fixture.inputs.parse_quality,
            &fixture.inputs.files
        ),
        ParseQuality::Unknown,
        "inadmissible precondition: original maps must lack the context parse"
    );
    assert!(
        fixture.files["context.py"].error_rate() <= 0.01,
        "inadmissible precondition: clean context parse required"
    );
    let mut meta = fixture.meta();
    meta.min_confidence = MinConfidence::Exact;
    let original = project(
        &[finding.clone()],
        &[Some(evidence.clone())],
        &fixture.inputs,
        &meta,
    );
    assert_eq!(original.targets.len(), 1);
    assert_eq!(original.targets[0].tier, FindingTier::Candidate);
    let viewed = project_with_context(
        &[finding.clone()],
        &[Some(evidence.clone())],
        &fixture.inputs,
        Some(fixture.view()),
        &meta,
    );
    assert_eq!(viewed.targets[0].parse_quality.as_deref(), Some("clean"));
    assert_eq!(viewed.targets[0].confidence, FindingConfidence::Exact);
    assert_eq!(viewed.targets[0].tier, FindingTier::Asserted);
    let mut wrapped = result(vec![finding.clone()], vec![Some(evidence.clone())]);
    let mut filtered = result(vec![finding.clone()], vec![Some(evidence.clone())]);
    filter_result_findings(
        &mut wrapped,
        &fixture.inputs,
        MinConfidence::Exact,
        ResolutionMode::Scoped,
    );
    filter_result_findings_in(
        &mut filtered,
        fixture.view(),
        MinConfidence::Exact,
        ResolutionMode::Scoped,
    );
    assert_eq!(wrapped.findings.len(), 1);
    assert_eq!(filtered.findings.len(), 1);
    meta.min_tier = FindingTier::Asserted;
    assert!(project(
        &[finding.clone()],
        &[Some(evidence.clone())],
        &fixture.inputs,
        &meta
    )
    .targets
    .is_empty());
    assert_eq!(
        project_with_context(
            &[finding],
            &[Some(evidence)],
            &fixture.inputs,
            Some(fixture.view()),
            &meta
        )
        .targets
        .len(),
        1
    );
}

#[test]
fn v03_poor_context_parse_is_graded_from_the_bundle() {
    let fixture =
        Fixture::new("def contextual(value):\n    first = value\n    broken =\n    return first\n");
    let rate = fixture.files["context.py"].error_rate();
    assert!(
        rate > 0.01 && rate <= 0.3,
        "inadmissible precondition: admitted degraded/poor real parse required, rate={rate}"
    );
    let quality = &fixture.quality["context.py"].quality;
    assert!(
        matches!(quality.as_str(), "degraded" | "poor"),
        "inadmissible precondition: authoritative grader must report degraded/poor"
    );
    let finding = finding("taint", "context.py");
    let exact = witness("context.py", ResolutionConfidence::Exact);
    let mut meta = fixture.meta();
    meta.min_confidence = MinConfidence::Exact;
    let original = project(
        &[finding.clone()],
        &[Some(exact.clone())],
        &fixture.inputs,
        &meta,
    );
    assert_eq!(
        original.targets[0].parse_quality.as_deref(),
        Some("unknown"),
        "inadmissible precondition: baseline must lack context quality"
    );
    let viewed = project_with_context(
        &[finding.clone()],
        &[Some(exact.clone())],
        &fixture.inputs,
        Some(fixture.view()),
        &meta,
    );
    assert_eq!(
        viewed.targets[0].parse_quality.as_deref(),
        Some(quality.as_str())
    );
    assert_eq!(viewed.targets[0].confidence, FindingConfidence::Exact);
    assert_eq!(viewed.targets[0].tier, FindingTier::Candidate);
    let mut retained = result(vec![finding.clone()], vec![Some(exact.clone())]);
    filter_result_findings_in(
        &mut retained,
        fixture.view(),
        MinConfidence::Exact,
        ResolutionMode::Scoped,
    );
    assert_eq!(retained.findings.len(), 1);
    let mut rejected = result(
        vec![finding.clone()],
        vec![Some(witness("context.py", ResolutionConfidence::NameOnly))],
    );
    filter_result_findings_in(
        &mut rejected,
        fixture.view(),
        MinConfidence::Exact,
        ResolutionMode::Scoped,
    );
    assert!(rejected.findings.is_empty());
    meta.min_tier = FindingTier::Asserted;
    assert!(project_with_context(
        &[finding],
        &[Some(exact)],
        &fixture.inputs,
        Some(fixture.view()),
        &meta
    )
    .targets
    .is_empty());
}

#[test]
fn v09_nameonly_keeps_malformed_evidence_untouched() {
    let fixture = Fixture::new(CONTEXT);
    let mut value = result(
        vec![finding("taint", "context.py"), finding("echo", "main.py")],
        vec![],
    );
    value.warnings.push("alignment remains caller-owned".into());
    let before = bytes(&value);
    filter_result_findings_in(
        &mut value,
        fixture.view(),
        MinConfidence::NameOnly,
        ResolutionMode::Scoped,
    );
    assert_eq!(bytes(&value), before);
    assert!(value.evidence.is_empty());
}

#[test]
fn v08_duplicate_taint_findings_keep_multiplicity() {
    let fixture = Fixture::new(CONTEXT);
    let findings = vec![finding("taint", "main.py"); 2];
    for (confidence, expected) in [
        (ResolutionConfidence::Exact, 2),
        (ResolutionConfidence::NameOnly, 0),
    ] {
        let mut filtered = result(
            findings.clone(),
            vec![Some(witness("main.py", confidence)); 2],
        );
        filter_result_findings_in(
            &mut filtered,
            fixture.view(),
            MinConfidence::Exact,
            ResolutionMode::Scoped,
        );
        assert_eq!(filtered.findings.len(), expected);
        assert_eq!(filtered.evidence.len(), expected);
        if expected == 2 {
            assert_eq!(bytes(&filtered.findings[0]), bytes(&filtered.findings[1]));
        }
    }
    let evidence = vec![Some(witness("main.py", ResolutionConfidence::Exact)); 2];
    let meta = fixture.meta();
    let original = project(&findings, &evidence, &fixture.inputs, &meta);
    let viewed = project_with_context(
        &findings,
        &evidence,
        &fixture.inputs,
        Some(fixture.view()),
        &meta,
    );
    assert_eq!(viewed.targets.len(), 1);
    assert_eq!(bytes(&original), bytes(&viewed));
    assert_eq!(viewed.warnings.len(), 1);
    assert_eq!(
        viewed.warnings[0],
        format!(
            "targets: duplicate id {} dropped (taint/taint_sink main.py:3)",
            viewed.targets[0].id
        )
    );
}
