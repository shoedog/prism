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
mod types;
mod views;
pub use compact::views::{IdIter, Locations};
pub use storage::DataFlowGraph;
pub(crate) use types::{ExactFlowEdge, ExactVarLocation};
pub use types::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
pub use views::{BindingKey, Edges, FlowEdgeRef};
#[cfg(test)]
pub mod test_support;
#[cfg(test)]
mod tests;
