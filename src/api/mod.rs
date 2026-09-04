//! Within a major version, every item of `prism::api` keeps its name and signature; a removal or signature change is preceded by a `#[deprecated]` release. Every struct and enum reachable through `prism::api` — defined in `api`, `finding_confidence`, `targets`, or `output::sarif` — is `#[non_exhaustive]`: construct with `new`/`Default`/builders and assign public fields, never with a struct literal or exhaustive `match` (`SarifInputs` has a builder; `TargetsMeta` is `Default` + field assignment; `TargetsDocument` and its nested types are produced by `project` and read/deserialized by consumers). Types from other modules that appear in `prism::api` signatures (`ParsedFile`, `TypeDatabase`, `CpgContext`, `SliceConfig`, `SlicingAlgorithm`, `SliceResult`, `SliceFinding`, `NavigationSession`, `Evidence`, `QueryError`, `DiffInput`, `Language`, `LanguageVersion`) are **stable as handles**: you may obtain them from `prism::api`, pass them back into `prism::api`, and read the fields the `prism::api` docs name; their other fields and methods are internal and may change. Everything else in the crate is internal. Output formats are versioned by their own fields: multi-run `json`/`review` carry `version: "1.0"`; single-run `json`/`review` shapes are unversioned and pinned by tests; SARIF carries `properties.mapping_version`; targets carries `schema_version`.

mod build_info;
mod nav;
mod review;
mod run;

pub use build_info::{build_info, BuildInfo};
pub use nav::{callees, callers, nav_session, NavOptions, Seed};
pub use review::{build_context, load_review_inputs, BuiltContext, ReviewInputs, ReviewOptions};
pub use run::{
    annotate_finding_parse_quality, parse_algorithms, run_algorithm, run_review, AlgorithmParams,
    ReviewOutcome, ReviewRun, DEFAULT_BARRIER_DEPTH, DEFAULT_SPIRAL_MAX_RING,
    DEFAULT_TEMPORAL_DAYS,
};

/// Run a complete diff-driven review through the stable facade: load inputs, build the CPG,
/// run the requested algorithms, and return both the inputs and the annotated findings.
///
/// This example is also the README "Library use" snippet (spec §2.5 item 6) — it is a running
/// doc-test, not `no_run`, so the sample the README shows is verified on every `cargo test`.
///
/// ```
/// use prism::api::{review, AlgorithmParams, ReviewOptions};
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

pub use crate::build_pool::install as with_build_pool;
pub use crate::finding_confidence::{
    classify, evidence_files, FindingConfidence, FindingTier, ParseQuality, RESOLUTION_MODE,
};
pub use crate::output::sarif::{to_sarif, SarifInputs};
pub use crate::targets::{project, TargetsDocument, TargetsMeta};
