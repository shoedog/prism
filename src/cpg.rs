//! Code Property Graph — unified graph merging AST, DFG, call graph, and (future) CFG.
//!
//! Built on `petgraph`, this module provides:
//! - **Schema types:** `CpgNode`, `CpgEdge` — node and edge types for the unified graph
//! - **Builder:** `CodePropertyGraph::build()` — constructs the graph from parsed files
//! - **Query methods:** edge-filtered reachability, SCC, shortest paths, subgraph views
//!
//! Algorithms can query the CPG instead of separately accessing `DataFlowGraph`,
//! `CallGraph`, and `ast.rs`. Edge-filtered traversals let each algorithm select
//! which relationship types to follow.
//!
//! See `docs/features/cpg/architecture.md` for the full design.
//!
//! This module is a façade: its implementation lives in the `src/cpg/`
//! submodules (`context`, `types`, `build`, `query`, `cfg_queries`). The
//! `pub use` re-exports below preserve the original `crate::cpg::*` API surface.

mod build;
mod cfg_queries;
mod context;
#[cfg(test)]
mod multiline_call_arg_parity_tests;
#[cfg(test)]
mod multiline_call_arg_tests;
pub mod query;
#[cfg(test)]
mod tests;
mod trace;
mod types;

// Items the test module reaches via `use super::*;`. On the original
// single-file module these were ordinary top-of-file imports; re-export them
// here so the moved (verbatim) test module keeps compiling.
#[cfg(test)]
use crate::access_path::AccessPath;
#[cfg(test)]
use std::collections::{BTreeMap, BTreeSet};

pub use build::CodePropertyGraph;
pub use context::{CpgContext, CpgScope};
pub use trace::{
    BoundaryEdge, BoundaryKind, OrderingDecision, OrderingUnavailableReason, OrderingWarning,
    Relation, SameLineOrderView, Trace,
};
pub use types::{CpgEdge, CpgNode, StmtKind, VarAccess};
