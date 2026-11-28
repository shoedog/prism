//! JavaScript/TypeScript/TSX parts of `ParsedFile`, and the JS/TS export and return fact models
//! (`exports`, `returns`).

pub(crate) mod component_route;
#[cfg(test)]
mod component_route_coverage_tests;
#[cfg(test)]
mod component_route_tests;
mod component_route_timing;
mod exact_reads;
mod export_facts;
#[cfg(test)]
mod export_facts_coverage_tests;
pub mod exports;
mod imports;
mod local_bindings;
mod param_alias;
mod params;
mod pattern_params;
pub(crate) use pattern_params::{formal_binding, PatternParam};
#[cfg(test)]
mod pattern_params_tests;
pub(crate) mod props_keys;
mod props_types;
pub mod proto_writes;
#[cfg(test)]
mod proto_writes_guard_assertion_member_tests;
#[cfg(test)]
mod proto_writes_guard_coverage_tests;
#[cfg(test)]
mod proto_writes_guard_function_tests;
#[cfg(test)]
mod proto_writes_guard_name_tests;
#[cfg(test)]
mod proto_writes_guard_own_tests;
#[cfg(test)]
mod proto_writes_guard_receiver_tests;
#[cfg(test)]
mod proto_writes_guard_sequence_tests;
#[cfg(test)]
mod proto_writes_guard_tests;
#[cfg(test)]
mod proto_writes_switch_tests;
#[cfg(test)]
mod proto_writes_tests;
mod receiver_evidence;
mod receiver_writes;
pub mod returns;
