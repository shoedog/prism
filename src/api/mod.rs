//! Within a major version, every item of `prism::api` keeps its name and signature; a removal or signature change is preceded by a `#[deprecated]` release. Every struct and enum defined in `prism::api` is `#[non_exhaustive]`: construct with `new`/`Default` and assign public fields, never with a struct literal or exhaustive `match`. Types from other modules that appear in `prism::api` signatures (`ParsedFile`, `TypeDatabase`, `CpgContext`, `SliceConfig`, `SlicingAlgorithm`, `SliceResult`, `SliceFinding`, `NavigationSession`, `Evidence`, `QueryError`, `DiffInput`, `Language`, `LanguageVersion`) are **stable as handles**: you may obtain them from `prism::api`, pass them back into `prism::api`, and read the fields the `prism::api` docs name; their other fields and methods are internal and may change. Everything else in the crate is internal. Output formats are versioned by their own fields: multi-run `json`/`review` carry `version: "1.0"`; single-run `json`/`review` shapes are unversioned and pinned by tests; SARIF carries `properties.mapping_version`; targets carries `schema_version`.

mod build_info;
mod nav;
mod review;
mod run;

pub use build_info::{build_info, BuildInfo};
pub use nav::{callees, callers, nav_session, NavOptions, Seed};
pub use review::{build_context, load_review_inputs, BuiltContext, ReviewInputs, ReviewOptions};
pub use run::{
    annotate_finding_parse_quality, parse_algorithms, review, run_algorithm, run_review,
    AlgorithmParams, ReviewOutcome, ReviewRun, DEFAULT_BARRIER_DEPTH, DEFAULT_SPIRAL_MAX_RING,
    DEFAULT_TEMPORAL_DAYS,
};

pub use crate::build_pool::install as with_build_pool;
pub use crate::finding_confidence::{
    classify, evidence_files, FindingConfidence, FindingTier, ParseQuality, RESOLUTION_MODE,
};
pub use crate::output::sarif::{to_sarif, SarifInputs};
pub use crate::targets::{project, TargetsDocument, TargetsMeta};
