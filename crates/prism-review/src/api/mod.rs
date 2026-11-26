//! The review facade: everything in `prism::api` plus the algorithm runner, SARIF and targets.
//! Same compatibility promise as `prism::api` (see its module doc).

#[cfg(test)]
mod context_matrix_tests;
pub(crate) mod context_options;
pub(crate) mod context_route;
#[allow(dead_code)] // S5 supplies the first request-bearing caller.
pub(crate) mod context_run;
pub(crate) mod context_stats;
pub(crate) mod context_view;
mod diagnostics;
pub(crate) mod map_view;
mod run;

pub use crate::algorithms::taint::boundary::{
    display::{BoundaryDisplay as ReviewBoundaryDiagnostics, LocatedBoundaryNote as ContextNote},
    records::{
        Availability as ReviewBoundaryAvailability, BoundaryKind as ReviewBoundaryKind,
        ContextRunNote, ContextRunNoteKind,
    },
};
pub use context_options::{
    TaintContextEdges, TaintContextMode, TaintContextOptions, DEFAULT_TAINT_CONTEXT_DEPTH,
    DEFAULT_TAINT_CONTEXT_MAX_FILES,
};
pub use context_route::{run_algorithm_authorized, ReviewAuthority};
pub use context_run::ReviewContext;
pub use context_view::{ResultView, ReviewAlgorithmRun};
pub use diagnostics::AlgorithmOutcome;

/// Run algorithms with loading authority borrowed from the same captured inputs.
///
/// `ctx` must have been built from these exact inputs. `repo` supplies history algorithms,
/// not loading authority. Options are validated before algorithms run. Defaults keep context
/// off; imports uses fixed 8 MiB and 20 s cooperative limits. Hints are loading advice,
/// and a diagnostic note is not a finding.
///
/// ```
/// use prism_review::api::{load_review_inputs, build_context, run_review_authorized,
///     AlgorithmParams, ReviewAuthority, ReviewOptions, TaintContextMode};
/// use prism::slice::{SliceConfig, SlicingAlgorithm};
/// use tempfile::TempDir;
/// let repo = TempDir::new()?;
/// std::fs::write(repo.path().join("main.js"),
///     "import { h } from './a';\nexport function baseline(z) { eval(z); }\nexport function main(x) { h(x); }\n")?;
/// std::fs::write(repo.path().join("a.js"),
///     "import { g } from './b';\nexport function h(x) { g(x); }\n")?;
/// std::fs::write(repo.path().join("b.js"), "export function g(x) { eval(x); }\n")?;
/// let mut options = ReviewOptions::new(repo.path());
/// options.no_cache = true;
/// let inputs = load_review_inputs(&options,
///     r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#)?;
/// let built = build_context(&inputs, &options)?;
/// let mut params = AlgorithmParams::default();
/// params.taint_context.mode = TaintContextMode::Imports;
/// let run = run_review_authorized(&built.ctx, ReviewAuthority::capture(&inputs),
///     &[SlicingAlgorithm::Taint], &SliceConfig::default(), &params, repo.path());
/// assert!(run.findings.iter().any(|f| f.file == "b.js"
///     && f.category.as_deref() == Some("taint_sink")));
/// assert!(run.context().is_some());
/// assert!(!inputs.sources.contains_key("b.js"));
/// # Ok::<(), anyhow::Error>(())
/// ```
pub use context_route::run_review_authorized;

pub use prism::api::*;

pub use run::{
    annotate_finding_parse_quality, filter_result_findings, parse_algorithms, run_algorithm,
    run_algorithm_with_diagnostics, run_review, AlgorithmParams, ReviewOutcome, ReviewRun,
    DEFAULT_BARRIER_DEPTH, DEFAULT_SPIRAL_MAX_RING, DEFAULT_TEMPORAL_DAYS,
};

pub use crate::output::sarif::{to_sarif, SarifInputs};
pub use crate::targets::inventory::{
    collect, function_bounds, load_python_files, Family, FunctionBounds, Grade, InventoryOptions,
    InventoryOutput, InventorySite, UnprovenCause, UnsupportedReason, UnsupportedSite,
};
pub use crate::targets::{
    inventory_document, merge_inventory, project, project_in_context, TargetsDocument, TargetsMeta,
};

/// Run a complete diff-driven review through the stable facade: load inputs, build the CPG,
/// run the requested algorithms, and return both the inputs and the annotated findings.
///
/// Taint-context options are validated before loading inputs, even when context is `Off`.
/// With `TaintContextMode::Imports` and taint selected, this entry point can load context
/// for import-directed taint expansion using authority from the captured review inputs.
///
/// This example is also the README "Library use" snippet (spec §2.5 item 6) — it is a running
/// doc-test, not `no_run`, so the sample the README shows is verified on every `cargo test`.
///
/// ```
/// use prism_review::api::{review, AlgorithmParams, ReviewOptions};
/// use prism::slice::{SliceConfig, SlicingAlgorithm};
/// use std::fs;
/// use tempfile::TempDir;
///
/// // `TempDir` removes its directory on drop — including on an assertion panic below — so
/// // nothing leaks even if this doc-test fails.
/// let repo = TempDir::new()?;
/// fs::write(repo.path().join("a.py"), "def read():\n    f = open(\"x\")\n    return f\n")?;
///
/// let diff_json = r#"{"files":[{"file_path":"a.py","modify_type":"Modified","diff_lines":[2]}]}"#;
/// let outcome = review(
///     &ReviewOptions::new(repo.path()),
///     diff_json,
///     &[SlicingAlgorithm::AbsenceSlice],
///     &SliceConfig::default(),
///     &AlgorithmParams::default(),
/// )?;
///
/// for f in &outcome.run.findings {
///     println!("{}:{} {} {}", f.file, f.line, f.algorithm, f.description);
/// }
/// assert!(!outcome.run.findings.is_empty());
/// assert_eq!(
///     outcome.run.findings[0].category.as_deref(),
///     Some("missing_counterpart")
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
pub use run::review;
