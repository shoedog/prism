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

pub mod algorithms;
pub mod ast;
pub mod call_graph;
pub mod data_flow;
pub mod diff;
pub mod languages;
pub mod output;
pub mod slice;
