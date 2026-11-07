//! JavaScript/TypeScript taint sources, sinks and sanitizers.

pub(super) mod alias_edges;
pub(super) mod bindings;
pub(super) mod exec_sql;
pub(super) mod framework_params;
pub(super) mod guards;
pub(super) mod imports_ssrf;
pub(super) mod request_aliases;
pub(super) mod request_targets;
pub(super) mod sanitizers;
pub(super) mod sink_tables;
pub(super) mod sinks;
pub(super) mod source_access;
pub(super) mod source_ranges;
