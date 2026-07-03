//! Tier-2 reasoning layer (always compiled; only MCP tool registration is `mcp`-gated).
//! Ephemeral, read-only computation over the production CPG - no overlay data structure.

pub mod order;
pub mod sanitizer_walk;
pub mod scope_honesty;
pub mod seeds;
pub mod shape;
pub mod taint_reaches;
pub mod types;
