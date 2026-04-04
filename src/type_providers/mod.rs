//! Per-language type provider implementations.
//!
//! Each provider extracts type information from tree-sitter ASTs (and
//! optionally external sources like `compile_commands.json` or `.d.ts` files)
//! and implements the `TypeProvider` trait from `crate::type_provider`.

pub mod cpp;
pub mod go;
