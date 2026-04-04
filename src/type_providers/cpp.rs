//! C/C++ type provider wrapping the existing `TypeDatabase`.
//!
//! This is a thin adapter that implements `TypeProvider` and `DispatchProvider`
//! by delegating to the existing `TypeDatabase` infrastructure. No behavioral
//! change — all existing C/C++ type queries continue to work identically.

use crate::call_graph::FunctionId;
use crate::languages::Language;
use crate::type_db::TypeDatabase;
use crate::type_provider::{
    DispatchProvider, ResolvedType, ResolvedTypeKind, TypeFieldInfo, TypeProvider,
};
use std::collections::BTreeSet;
use std::sync::Arc;

/// C/C++ type provider backed by `TypeDatabase`.
///
/// Wraps the existing clang-based type extraction (from `compile_commands.json`)
/// and tree-sitter fallback. Uses `Arc<TypeDatabase>` so the same backing data
/// can be shared when this provider is registered as both `TypeProvider` and
/// `DispatchProvider` in the registry.
#[derive(Clone)]
pub struct CppTypeProvider {
    /// The underlying type database (shared via Arc).
    pub db: Arc<TypeDatabase>,
}

impl CppTypeProvider {
    /// Create a new provider wrapping an existing TypeDatabase.
    pub fn new(db: TypeDatabase) -> Self {
        CppTypeProvider { db: Arc::new(db) }
    }

    /// Create a new provider sharing an existing Arc<TypeDatabase>.
    pub fn from_arc(db: Arc<TypeDatabase>) -> Self {
        CppTypeProvider { db }
    }

    /// Get a reference to the underlying TypeDatabase.
    ///
    /// Used by `CodePropertyGraph` to preserve backward-compatible access for
    /// virtual dispatch enrichment during CPG construction.
    pub fn type_db(&self) -> &TypeDatabase {
        &self.db
    }
}

impl TypeProvider for CppTypeProvider {
    fn resolve_type(&self, _file: &str, expr: &str, _line: usize) -> Option<ResolvedType> {
        // For C/C++, try to resolve the expression as a type name or typedef.
        let canonical = self.db.resolve_typedef(expr);
        if canonical == expr && !self.db.records.contains_key(expr) {
            return None;
        }
        let kind = if let Some(record) = self.db.resolve_record(&canonical) {
            match record.kind {
                crate::type_db::RecordKind::Struct | crate::type_db::RecordKind::Class => {
                    ResolvedTypeKind::Concrete
                }
                crate::type_db::RecordKind::Union => ResolvedTypeKind::Concrete,
            }
        } else {
            ResolvedTypeKind::Alias
        };
        Some(ResolvedType {
            name: canonical,
            kind,
            type_params: Vec::new(),
        })
    }

    fn field_layout(&self, type_name: &str) -> Option<Vec<TypeFieldInfo>> {
        let record = self.db.resolve_record(type_name)?;
        Some(
            record
                .fields
                .iter()
                .map(|f| TypeFieldInfo {
                    name: f.name.clone(),
                    type_str: f.type_str.clone(),
                })
                .collect(),
        )
    }

    fn subtypes_of(&self, type_name: &str) -> Vec<String> {
        // Return all subclasses (classes that inherit from type_name).
        self.db
            .records
            .keys()
            .filter(|name| self.db.is_subclass_of(name, type_name))
            .cloned()
            .collect()
    }

    fn resolve_alias(&self, type_name: &str) -> String {
        self.db.resolve_typedef(type_name)
    }

    fn languages(&self) -> Vec<Language> {
        vec![Language::C, Language::Cpp]
    }
}

impl DispatchProvider for CppTypeProvider {
    fn resolve_dispatch(
        &self,
        receiver_type: &str,
        method: &str,
        live_types: &BTreeSet<String>,
    ) -> Vec<FunctionId> {
        let targets = self
            .db
            .virtual_dispatch_targets_rta(receiver_type, method, live_types);
        targets
            .into_iter()
            .map(|class_name| FunctionId {
                name: method.to_string(),
                file: self
                    .db
                    .records
                    .get(&class_name)
                    .map(|r| r.file.clone())
                    .unwrap_or_default(),
                start_line: 0,
                end_line: 0,
            })
            .collect()
    }
}
