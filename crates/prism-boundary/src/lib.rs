//! Typed identities for service boundary declarations.
//!
//! PROVISIONAL: admission is certified only for the request shape the real-pair
//! replay exercises (historical review, two inputs with one blob receipt each, two
//! schema claims). Every other mode and shape is accepted but not certified; see
//! the crate README, section "P1d admission claim".

mod diff;
pub mod element;
mod inventory;
pub mod model;
pub mod revision;

pub use diff::*;
pub use inventory::{
    definitions_of, index_schema, operations_reaching, signature, Coverage, DefinitionId,
    EnumValue, Field, FieldShape, JsonMediaSource, Lookup, MediaDeclarationSource, MessageCoverage,
    Operation, OperationSite, Property, RegionLimit, RootKey, SchemaDiagnostic, SchemaIndex,
    SchemaSite, SchemaSource, StatusInventory,
};
pub use model::{Binding, BindingStatus, BindingsDocument, ImpactDocument, Unbound};
pub use revision::{admit_inputs, admit_schema_input, AdmittedSchemaInput, SnapshotInput};
pub use schema::SchemaError;

// The operation reader remains internal.
#[allow(dead_code, unused_imports)]
mod schema;
