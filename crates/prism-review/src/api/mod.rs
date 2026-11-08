//! The review facade: everything in `prism::api` plus the algorithm runner, SARIF and targets.
//! Same compatibility promise as `prism::api` (see its module doc).

mod run;

pub use prism::api::*;

pub use run::{
    annotate_finding_parse_quality, filter_result_findings, parse_algorithms, run_algorithm,
    run_review, AlgorithmParams, ReviewOutcome, ReviewRun, DEFAULT_BARRIER_DEPTH,
    DEFAULT_SPIRAL_MAX_RING, DEFAULT_TEMPORAL_DAYS,
};

pub use crate::output::sarif::{to_sarif, SarifInputs};
pub use crate::targets::{project, TargetsDocument, TargetsMeta};
/// Run a complete diff-driven review through the stable facade: load inputs, build the CPG,
/// run the requested algorithms, and return both the inputs and the annotated findings.
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
