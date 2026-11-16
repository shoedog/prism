//! REAL repositories and captured loaders; explicitly marked CONTROLLED planner inputs.
use super::super::boundary::records::PhysicalSite;
use super::super::context_budget::SkipReason;
use super::super::frontier::{CallProvenance, Disposition, Request};
use super::driver::settle;
use super::stage::{run_stage, StageStop, Staged};
use super::*;
use prism::api::{
    build_context, load_review_inputs, ReviewContextLoader, ReviewHintBasis, ReviewImportHint,
    ReviewOptions,
};
use prism::call_graph::FunctionId;
use prism::languages::Language;
use std::cell::Cell;

mod atomic;
mod bounds;
mod real;

const MAIN: &str = "import { f } from './a';\nimport { g } from './b';\nimport { h } from './c';\nexport function main(x) { f(x); g(x); h(x); }\n";
const MAIN_NO_C_CALL: &str = "import { f } from './a';\nimport { g } from './b';\nimport { h } from './c';\nexport function main(x) { f(x); g(x); }\n";
const BODY: &str = "export const value = 1;\n";
struct Guard {
    calls: Cell<usize>,
    at: usize,
    cause: StopCause,
}
impl Guard {
    fn never() -> Self {
        Self {
            calls: Cell::new(0),
            at: usize::MAX,
            cause: StopCause::Time,
        }
    }
    fn at(at: usize, cause: StopCause) -> Self {
        Self {
            calls: Cell::new(0),
            at,
            cause,
        }
    }
}
impl RunGuard for Guard {
    fn stop(&self) -> Option<StopCause> {
        let n = self.calls.get();
        self.calls.set(n + 1);
        (n >= self.at).then_some(self.cause)
    }
}
struct Fixture {
    _dir: tempfile::TempDir,
    inputs: ReviewInputs,
    options: ReviewOptions,
}
impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        Self::with_main(MAIN, files)
    }
    fn with_main(main: &str, files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.js"), main).unwrap();
        for (name, source) in files {
            std::fs::write(dir.path().join(name), source).unwrap();
        }
        let options = ReviewOptions::new(dir.path());
        let diff =
            r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[4]}]}"#;
        let inputs = load_review_inputs(&options, diff).unwrap();
        assert!(
            inputs.files.contains_key("main.js"),
            "inadmissible precondition: original main"
        );
        Self {
            _dir: dir,
            inputs,
            options,
        }
    }
    fn run<F, R>(&self, limits: ContextLimits, guard: &dyn RunGuard, f: F) -> R
    where
        F: FnOnce(&ExpansionRequest<'_>) -> R,
    {
        let built = build_context(&self.inputs, &self.options).unwrap();
        let loader = ReviewContextLoader::new(&self.inputs)
            .expect("inadmissible precondition: captured loader");
        let config = TaintConfig::default();
        let (_, trace) =
            super::super::slice_with_trace(&built.ctx, &self.inputs.diff, &config).unwrap();
        let request = ExpansionRequest {
            loader: &loader,
            inputs: &self.inputs,
            baseline_ctx: &built.ctx,
            baseline_trace: &trace,
            config: &config,
            limits,
            ceiling: CrossingCeiling::NameOnly,
            guard,
        };
        f(&request)
    }
}
fn limits(depth: u8, files: usize) -> ContextLimits {
    ContextLimits::new(depth, files).unwrap()
}
fn initial(req: &ExpansionRequest<'_>) -> CompletedBundle {
    CompletedBundle {
        round: 0,
        files: req.inputs.files.clone(),
        sources: req.inputs.sources.clone(),
        parse_quality: req.inputs.parse_quality.clone(),
        parse_warnings: req.inputs.parse_warnings.clone(),
        context_files: BTreeSet::new(),
        published: Published::default(),
        analysis: None,
        frontier: FrontierState::default(),
    }
}
// CONTROLLED: physical call and requests are injected; reads/builds/analysis use real providers.
fn pending(bundle: &mut CompletedBundle, targets: &[(&str, &str)]) {
    for (path, member) in targets {
        let i = bundle.frontier.provenance.len();
        let hint = ReviewImportHint::InRepo {
            files: vec![(*path).into()],
            spec: "./x".into(),
            member: Some((*member).into()),
            declaring: vec![],
            basis: ReviewHintBasis::Bound,
        };
        bundle.frontier.provenance.push(CallProvenance {
            site: PhysicalSite {
                caller: FunctionId {
                    file: "main.js".into(),
                    name: "controlled".into(),
                    start_line: 1,
                    end_line: 4,
                },
                start_byte: i,
                end_byte: i + 1,
            },
            line: 4,
            callee: (*member).into(),
            language: Language::JavaScript,
            hint,
            first_round: 0,
        });
        bundle.frontier.open.push(Request {
            target: (*path).into(),
            member: Some((*member).into()),
            depth: 1,
            origin: i,
            visited: vec![((*path).into(), Some((*member).into()))],
        });
        if !bundle.files.contains_key(*path) {
            bundle.frontier.load_order.push((*path).into());
        }
    }
}
fn snapshot(bundle: &CompletedBundle) -> String {
    let files: Vec<_> = bundle
        .files
        .iter()
        .map(|(path, file)| {
            (
                path,
                &file.source,
                file.language,
                file.parse_error_count,
                file.parse_node_count,
            )
        })
        .collect();
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        bundle.round,
        files,
        bundle.sources,
        bundle.parse_quality,
        bundle.parse_warnings,
        bundle.context_files,
        bundle.published,
        bundle.analysis,
        bundle.frontier
    )
}
fn outcome_snapshot(outcome: &ExpansionOutcome) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}",
        snapshot(&outcome.bundle),
        outcome.stop,
        outcome.attempts,
        outcome.stats
    )
}
fn run_owned(req: &ExpansionRequest<'_>) -> ExpansionOutcome {
    expand(ExpansionRequest {
        loader: req.loader,
        inputs: req.inputs,
        baseline_ctx: req.baseline_ctx,
        baseline_trace: req.baseline_trace,
        config: req.config,
        limits: req.limits,
        ceiling: req.ceiling,
        guard: req.guard,
    })
}
