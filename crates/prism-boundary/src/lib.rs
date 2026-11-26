//! Typed identities for service boundary declarations.

pub mod element;
mod inventory;
pub mod revision;

pub use inventory::{
    definitions_of, index_schema, operations_reaching, signature, Coverage, DefinitionId,
    EnumValue, Field, FieldShape, JsonMediaSource, Lookup, MessageCoverage, Operation, Property,
    RegionLimit, RootKey, SchemaIndex, SchemaSource,
};
pub use revision::AdmittedSchemaInput;
pub use schema::SchemaError;

// The operation reader remains internal.
#[allow(dead_code, unused_imports)]
mod schema;
