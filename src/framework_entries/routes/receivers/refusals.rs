// STUB: replaced by the implementation
//! Counted refusals of the Python static import binding rule (DESIGN section 5.3).
//!
//! Contract the implementation fulfils: `import_refusals` is valid after `flask_facts` ran on
//! the bindings and returns the exact multiset of counted refusals, one per distinct refused
//! capture span (first applicable reason in predicate order) and one per distinct whole-call
//! span for a refused no-effect callee. Spans are UTF-8 byte ranges of the written text.
#![allow(dead_code)]

use super::PythonBindings;

/// Counted reasons, declared in predicate order (`class_private_name` before `class_body_import`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImportRefusalReason {
    GetattrModule,
    StarImport,
    GlobalWrite,
    MultiWrite,
    NestedSingle,
    SubmoduleShadow,
    ModuleDescriptorName,
    ClassBodyImport,
    ClassPrivateName,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportRefusal {
    pub reason: ImportRefusalReason,
    pub file: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl PythonBindings<'_> {
    pub(crate) fn import_refusals(&self) -> Vec<ImportRefusal> {
        Vec::new()
    }
}
