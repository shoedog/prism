//! `prism::api` is the core facade of the `prism` library: build identity (`build_info`), navigation
//! (`nav_session`, `callers`, `callees`), the owner options, the review inputs and graph context
//! (`load_review_inputs`, `build_context`, `ReviewOptions`), the finding-confidence classification,
//! and `with_build_pool`. The review runner (`review`, `run_review`, `run_algorithm`), SARIF
//! (`to_sarif`, `SarifInputs`) and targets (`project`, `TargetsDocument`, `TargetsMeta`) live in the
//! `prism-review` crate's `prism_review::api`, which re-exports everything in this module.
//!
//! Within a major version, every item of `prism::api` and `prism_review::api` keeps its name and
//! signature; a removal or signature change is preceded by a `#[deprecated]` release. 4.0 moved the
//! review runner, SARIF and targets items to `prism_review::api`.
//!
//! Every struct and enum reachable through `prism::api` defined in `api` or `finding_confidence`, and
//! every one reachable through `prism_review::api` defined in its own `api` module, `targets` or
//! `output::sarif`, is
//! `#[non_exhaustive]`: construct with `new`/`Default`/builders and assign public fields, never with a
//! struct literal or exhaustive `match` (`SarifInputs` has a builder; `TargetsMeta` is `Default` +
//! field assignment; `TargetsDocument` and its nested types are produced by `project` and
//! read/deserialized by consumers). Types from other modules that appear in these signatures
//! (`ParsedFile`, `TypeDatabase`, `CpgContext`, `SliceConfig`, `SlicingAlgorithm`, `SliceResult`,
//! `SliceFinding`, `NavigationSession`, `Evidence`, `QueryError`, `DiffInput`, `Language`,
//! `LanguageVersion`) are **stable as handles**: you may obtain them from the facades, pass them back
//! into the facades, and read the fields the facade docs name; their other fields and methods are
//! internal and may change. Everything else in the crates is internal. Output formats are versioned by
//! their own fields: multi-run `json`/`review` carry `version: "1.0"`; single-run `json`/`review`
//! shapes are unversioned and pinned by tests; SARIF carries `properties.mapping_version`; targets
//! carries `schema_version`.

mod build_info;
mod nav;
mod owner;
mod review;

#[doc(hidden)] // prism-mcp owner runtime; outside the api compatibility promise
pub use crate::executable_owner::admission::Failure as OwnerFailure;
pub use build_info::{build_info, BuildInfo};
pub use nav::{callees, callers, nav_session, NavOptions, Seed};
#[doc(hidden)]
// clap args shared by the prism and prism-mcp CLIs; outside the api compatibility promise
pub use owner::OwnerArgs;
pub use owner::OwnerOptions;
pub use review::{build_context, load_review_inputs, BuiltContext, ReviewInputs, ReviewOptions};

/// Runs `op` on prism's large-stack worker pool and returns its result.
///
/// Wrap a whole embedding session (or any call into the `prism` graph
/// constructors) in this to give prism's recursive syntax walks the
/// large-stack workers they need. Nesting is safe from the pool itself and from
/// ordinary threads: inside the pool the call runs inline.
///
/// Called from a worker of *another* rayon pool, `op` runs on the build pool
/// while that worker waits in a blocking join, so it never runs the calling
/// pool's queued jobs. `op` must therefore not wait on work that only the
/// calling pool can run, for example by calling that same pool's `install`
/// when all of its workers are waiting in this way. prism's own constructors
/// never do this. See [`crate::build_pool::install`] for the three routes.
pub use crate::build_pool::install as with_build_pool;
pub use crate::finding_confidence::{
    classify, classify_for_resolution, classify_with_evidence, evidence_files, EvidenceHop,
    EvidencePath, FindingConfidence, FindingTier, MinConfidence, ParseQuality, ResolutionMode,
    DEFAULT_MIN_CONFIDENCE, DEFAULT_RESOLUTION,
};
