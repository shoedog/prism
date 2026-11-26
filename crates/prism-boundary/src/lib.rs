//! Typed identities for service boundary declarations.

mod diff;
pub mod element;
mod inventory;
pub mod model;
pub mod revision;

pub use diff::*;
pub use inventory::{
    definitions_of, index_schema, operations_reaching, signature, Coverage, DefinitionId,
    EnumValue, Field, FieldShape, JsonMediaSource, Lookup, MessageCoverage, Operation,
    OperationSite, Property, RegionLimit, RootKey, SchemaDiagnostic, SchemaIndex, SchemaSite,
    SchemaSource, StatusInventory,
};
pub use model::{Binding, BindingStatus, BindingsDocument, ImpactDocument, Unbound};
pub use revision::{admit_inputs, admit_schema_input, AdmittedSchemaInput, SnapshotInput};
pub use schema::SchemaError;

// The operation reader remains internal.
#[allow(dead_code, unused_imports)]
mod schema;
