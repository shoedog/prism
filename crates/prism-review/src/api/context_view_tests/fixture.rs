//! REAL parses; CONTROLLED map/state/finding assembly. No expansion or engine reach claim.
use super::*;
use crate::algorithms::taint::{
    context_budget::{AttemptCounters, Published},
    frontier::FrontierState,
    result_union::{UnionError, UnionStats},
    rounds::{RoundStats, Stop},
};
use prism::finding_confidence::{EvidenceHop, EvidencePath};
use prism::{
    ast::{parse_quality::check_parse_quality, ParsedFile},
    call_graph::FunctionId,
    languages::Language,
    resolution::{ResolutionConfidence, ResolutionKind, ResolvedCallEdge},
};
use std::fs;
use tempfile::TempDir;

const CLEAN: &str = "def contextual(value):\n    first = value\n    return first\n";
const DIRTY: &str = "def contextual(value):\n    first = value\n    broken =\n    return first\n";
pub(super) struct Fixture {
    pub _repo: TempDir,
    pub inputs: ReviewInputs,
}
impl Fixture {
    pub fn new() -> Self {
        let repo = TempDir::new().unwrap();
        fs::write(repo.path().join("main.py"), CLEAN).unwrap();
        fs::write(repo.path().join("dirty.py"), DIRTY).unwrap();
        let mut options = crate::api::ReviewOptions::new(repo.path());
        options.no_cache = true;
        let inputs = crate::api::load_review_inputs(&options,
            r#"{"files":[{"file_path":"main.py","modify_type":"Modified","diff_lines":[2]},{"file_path":"dirty.py","modify_type":"Modified","diff_lines":[2]}]}"#).unwrap();
        assert!(
            inputs.files.contains_key("main.py") && inputs.files.contains_key("dirty.py"),
            "fixture input guard: both original files parsed"
        );
        assert!(
            inputs.parse_quality.contains_key("dirty.py"),
            "fixture input guard: dirty original"
        );
        Self {
            _repo: repo,
            inputs,
        }
    }
    pub fn context(&self) -> ReviewContext {
        let mut files = self.inputs.files.clone();
        let mut sources = self.inputs.sources.clone();
        for (name, source) in [("context.py", CLEAN), ("dirty_context.py", DIRTY)] {
            files.insert(
                name.into(),
                ParsedFile::parse(name, source, Language::Python).unwrap(),
            );
            sources.insert(name.into(), source.into());
        }
        let (_, parse_quality) = check_parse_quality(&files);
        assert!(
            files["context.py"].error_rate() <= 0.01,
            "fixture input guard: clean context"
        );
        assert!(
            parse_quality.contains_key("dirty_context.py"),
            "fixture input guard: dirty context"
        );
        ReviewContext {
            files,
            sources,
            parse_quality,
            parse_warnings: vec!["controlled warning".into()],
            context_files: BTreeSet::from(["context.py".into(), "dirty_context.py".into()]),
            round: 2,
            published: Published {
                files: 2,
                bytes: (CLEAN.len() + DIRTY.len()) as u64,
            },
            stats: RoundStats {
                rounds_started: 2,
                rounds_completed: 2,
            },
            stop: Stop::FrontierExhausted,
            attempts: AttemptCounters::default(),
            frontier: FrontierState::default(),
            union: UnionState::Applied {
                stats: UnionStats::default(),
                added_block_ids: vec![17],
            },
        }
    }
}
pub(super) fn finding(algorithm: &str) -> SliceFinding {
    SliceFinding {
        algorithm: algorithm.into(),
        file: "context.py".into(),
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
pub(super) fn witness(confidence: ResolutionConfidence) -> EvidencePath {
    EvidencePath::from_hops(vec![EvidenceHop::Call {
        edge: ResolvedCallEdge {
            caller: FunctionId {
                file: "context.py".into(),
                name: "contextual".into(),
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
pub(super) fn result() -> SliceResult {
    let mut result = SliceResult::new(SlicingAlgorithm::Taint);
    result.findings = vec![finding("taint")];
    result.evidence = vec![Some(witness(ResolutionConfidence::Exact))];
    result.warnings = vec!["original warning".into()];
    result
}
pub(super) fn single(result: SliceResult, state: ContextState) -> ReviewAlgorithmRun {
    let mut outcome = AlgorithmOutcome::new(result);
    outcome.diagnostics = Some(super::super::super::ReviewBoundaryDiagnostics::default());
    ReviewAlgorithmRun::new(outcome, state)
}
pub(super) fn multi(state: ContextState) -> super::super::super::ReviewRun {
    super::super::super::ReviewRun {
        context: state,
        diagnostics: None,
        results: vec![],
        findings: vec![],
        evidence: vec![],
        errors: vec![],
        warnings: vec![],
        algorithms_run: vec![],
        min_confidence: MinConfidence::NameOnly,
        resolution: ResolutionMode::Scoped,
    }
}
pub(super) fn original(view: ResultView<'_>, inputs: &ReviewInputs) {
    assert!(std::ptr::eq(view.maps().files, &inputs.files));
    assert!(std::ptr::eq(view.sources(), &inputs.sources));
    assert!(std::ptr::eq(
        view.maps().parse_quality,
        &inputs.parse_quality
    ));
    assert!(view.context_files.is_none());
    assert!(view.context_parse_quality().is_empty());
}
pub(super) fn snapshot(result: &SliceResult) -> (Vec<u8>, String) {
    (
        serde_json::to_vec(result).unwrap(),
        format!("{:?}", result.evidence),
    )
}
pub(super) fn refused(context: &mut ReviewContext) {
    context.union = UnionState::Refused(UnionError::BlockIdOverflow);
}
