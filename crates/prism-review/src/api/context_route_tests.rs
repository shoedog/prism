//! REAL captured repositories unless a row explicitly says CONTROLLED.
use super::*;
use crate::algorithms::taint::{
    self,
    context_budget::{ContextLimits, TimeGuard},
    rounds::{expand, ExpansionOutcome, ExpansionRequest},
};
use crate::api::{
    self,
    context_options::{TaintContextEdges, TaintContextMode, TaintContextOptions},
    context_run::ContextState,
    run::{
        review, run_algorithm, run_algorithm_prepared, run_algorithm_with_diagnostics, run_review,
        run_review_prepared,
    },
    AlgorithmParams, ReviewInputs, ReviewOptions, ReviewRun,
};
use prism::{
    api::{build_context, load_review_inputs, ReviewContextLoader, ReviewContextRound},
    cpg::{CpgContext, CrossingCeiling},
    finding_confidence::ResolutionMode,
    slice::{SliceConfig, SliceResult, SlicingAlgorithm},
};
use std::path::Path;
#[path = "context_route_tests/authority.rs"]
mod authority;
#[path = "context_route_tests/defaults.rs"]
mod defaults;
#[path = "context_route_tests/fixture.rs"]
mod fixture;
#[path = "context_route_tests/routing.rs"]
mod routing;
use fixture::*;
