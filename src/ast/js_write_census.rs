//! JS/TS/TSX write census (#660 S0): reviewed tables, the grammar certificate, the table-driven
//! walker and the same-node order relation. No production caller yet (S1b-1 wires it in), so this
//! slice changes no label, edge, counter, cache format or output. See
//! `docs/superpowers/plans/2026-11-05-660-js-def-census/DESIGN.md`, sections 4, 7 and 18.
#![allow(dead_code)]

#[cfg(test)]
#[path = "js_write_census/certificate.rs"]
mod certificate;
#[cfg(test)]
#[path = "js_write_census/certificate_tests.rs"]
mod certificate_tests;
#[cfg(test)]
#[path = "js_write_census/harness.rs"]
mod harness;
#[path = "js_write_census/identity.rs"]
pub(crate) mod identity;
#[cfg(test)]
#[path = "js_write_census/matrix_tests.rs"]
mod matrix_tests;
#[path = "js_write_census/order.rs"]
pub(crate) mod order;
#[path = "js_write_census/schema.rs"]
pub(crate) mod schema;
#[cfg(test)]
#[path = "js_write_census/semantic_tests.rs"]
mod semantic_tests;
#[path = "js_write_census/tables.rs"]
pub(crate) mod tables;
#[path = "js_write_census/walk.rs"]
pub(crate) mod walk;
