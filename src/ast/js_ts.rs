//! JavaScript/TypeScript/TSX parts of `ParsedFile`, and the JS/TS export and return fact models
//! (`exports`, `returns`).

mod exact_reads;
mod export_facts;
pub mod exports;
mod imports;
mod local_bindings;
mod param_alias;
mod params;
mod props_types;
pub mod proto_writes;
#[cfg(test)]
mod proto_writes_guard_tests;
#[cfg(test)]
mod proto_writes_tests;
mod receiver_evidence;
mod receiver_writes;
pub mod returns;
