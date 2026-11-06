//! JS/TS/TSX write census (#660 S0): reviewed tables, the grammar certificate, the table-driven
//! walker and the same-node order relation. S0 added no production caller; S1b-1's `completion` is the first. See
//! `docs/superpowers/plans/2026-11-05-660-js-def-census/DESIGN.md`, sections 4, 7 and 18.
//!
//! S1b-1 wires it in through `completion` (called from `cpg::reaching::census_defs`).
//!
//! The module is registered at the crate root (`#[path]` in `src/lib.rs`, because `src/ast.rs` is off limits for
//! this slice): use `crate::js_write_census`, and test names are `js_write_census::...` (mutation lane selectors).
#![allow(dead_code)]

#[cfg(test)]
#[path = "js_write_census/certificate.rs"]
mod certificate;
#[cfg(test)]
#[path = "js_write_census/certificate_tests.rs"]
mod certificate_tests;
#[path = "js_write_census/completion.rs"]
pub(crate) mod completion;
#[cfg(test)]
#[path = "js_write_census/harness.rs"]
mod harness;
#[path = "js_write_census/identity.rs"]
pub(crate) mod identity;
#[cfg(test)]
#[path = "js_write_census/mapping_tests.rs"]
mod mapping_tests;
#[cfg(test)]
#[path = "js_write_census/matrix_tests.rs"]
pub(crate) mod matrix_tests;
#[path = "js_write_census/order.rs"]
pub(crate) mod order;
#[cfg(test)]
#[path = "js_write_census/region_tests.rs"]
mod region_tests;
#[path = "js_write_census/schema.rs"]
pub(crate) mod schema;
#[cfg(test)]
#[path = "js_write_census/semantic_tests.rs"]
mod semantic_tests;
#[path = "js_write_census/tables.rs"]
pub(crate) mod tables;
#[path = "js_write_census/walk.rs"]
pub(crate) mod walk;
