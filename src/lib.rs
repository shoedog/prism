//! # Slicing
//!
//! Code slicing algorithms for defect-focused automated code review.
//!
//! ## Paper Algorithms (arXiv:2505.17928)
//!
//! - **OriginalDiff**: Raw diff lines only (baseline)
//! - **ParentFunction**: Entire enclosing function of each diff line
//! - **LeftFlow**: Backward data-flow from L-values on diff lines
//! - **FullFlow**: LeftFlow + forward tracing from R-values
//!
//! ## Established Taxonomy (Section 4)
//!
//! - **ThinSlice**: Data deps only, no control flow context
//! - **BarrierSlice**: Interprocedural with depth/boundary controls
//! - **Chop**: All paths between source and sink
//! - **Taint**: Forward trace of untrusted values
//! - **RelevantSlice**: Backward + alternate branch paths
//! - **ConditionedSlice**: Slice under a value assumption
//! - **DeltaSlice**: Behavioral diff between program versions
//!
//! ## Theoretical Extensions (Section 5)
//!
//! - **SpiralSlice**: Adaptive-depth concentric rings
//! - **CircularSlice**: Data flow cycle detection
//! - **QuantumSlice**: Concurrent state enumeration
//! - **HorizontalSlice**: Peer pattern consistency
//! - **VerticalSlice**: End-to-end feature path
//! - **AngleSlice**: Cross-cutting concern trace
//! - **ThreeDSlice**: Temporal-structural risk integration

// P7: `navigation::queries::call_stats`'s `serde_json::json!({...})` literal grew
// past the default macro recursion limit (128) with the new property-access
// telemetry keys (three more object entries alongside P5's Go func-value
// counters). Bump rather than restructure the call-stats literal.
#![recursion_limit = "256"]

pub mod access_path;
pub mod algorithms;
pub mod ast;
pub mod build_pool;
pub mod call_graph;
pub mod cfg;
pub mod cpg;
pub mod cpg_cache;
pub mod data_flow;
pub mod diff;
pub mod framework_entries;
pub mod frameworks;
pub mod go_alias_index;
pub mod go_build_profile;
mod go_mod;
mod go_module_graph;
pub mod go_owner_partition;
mod go_owner_partition_s4;
mod go_receiver_index;
mod go_receiver_index_visibility;
pub mod js_exports;
pub mod languages;
pub mod live_types;
mod manifest_snapshot;
pub use manifest_snapshot::ManifestSnapshot;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod name_resolution;
pub mod navigation;
pub mod output;
mod parameter_slots;
pub mod queries;
pub mod react_hooks;
pub mod reasoning;
mod receiver_index;
pub mod repo_loader;
pub mod resolution;
pub mod resolution_disproof;
pub mod resolution_identity;
pub mod resolution_receiver;
pub mod rust_macro_args;
pub mod sanitizers;
pub mod slice;
pub mod terraform;
pub mod type_db;
pub mod type_provider;
pub mod type_providers;
