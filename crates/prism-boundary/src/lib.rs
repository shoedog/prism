//! Typed identities for service boundary declarations.

pub mod element;
mod inventory;
pub mod revision;

pub use inventory::{
    definitions_of, index_schema, operations_reaching, signature, Coverage, DefinitionId,
    EnumValue, Field, FieldShape, JsonMediaSource, Lookup, MessageCoverage, Operation,
    OperationSite, Property, RegionLimit, RootKey, SchemaDiagnostic, SchemaIndex, SchemaSite,
    SchemaSource,
};
pub use revision::AdmittedSchemaInput;
pub use schema::SchemaError;

// The operation reader remains internal.
#[allow(dead_code, unused_imports)]
mod schema;
