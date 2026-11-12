//! Data flow graph construction for intraprocedural and interprocedural analysis.
//!
//! Builds def-use chains within functions and tracks flow through function
//! arguments and return values. Used by chopping, taint analysis, circular
//! slice, and delta slice.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, RdFileStats};
use std::collections::{BTreeMap, BTreeSet};
mod build;
mod compact;
mod queries;
mod storage;
pub mod terraform;
mod types;
mod views;
/// Test support for integration tests (production never constructs one): while held, the alias-twin
/// occurrence policy is off process-wide, so a frozen-reference receipt keeps main's twin behaviour.
#[doc(hidden)]
pub use build::PolicyOff as AliasTwinPolicyOff;
pub(crate) use compact::assembly::{AssemblyId, AssemblyView};
pub use compact::views::{IdIter, Locations};
pub use storage::DataFlowGraph;
pub(crate) use types::{ExactFlowEdge, ExactVarLocation};
pub use types::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
pub use views::{BindingKey, Edges, FlowEdgeRef};
#[cfg(test)]
pub mod test_support;
#[cfg(test)]
mod tests;
