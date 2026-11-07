//! Go resolution facts: build profiles, owner partitions, promoted selectors, receivers, satisfiers.

pub mod build_profile;
pub(crate) mod concrete_receiver;
pub(crate) mod func_type;
pub mod owner_partition;
pub(crate) mod owner_partition_s4;
pub mod promoted_snapshot;
pub(crate) mod satisfiers;
pub(crate) mod selector_supply;
