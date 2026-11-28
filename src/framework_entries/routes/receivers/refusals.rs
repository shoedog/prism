//! Counted refusals of the Python static import binding rule (DESIGN section 5.3).

use super::{ParsedFile, PythonBindings, Reason};
use std::{cell::RefCell, collections::BTreeMap};
use tree_sitter::Node;

#[derive(Clone, Copy)]
pub(super) enum Capture<'a> {
    Silent,
    Value,
    Callee(Node<'a>),
}

/// Counted reasons in predicate order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImportRefusalReason {
    GetattrModule,
    StarImport,
    GlobalWrite,
    MultiWrite,
    NestedSingle,
    SubmoduleShadow,
    ModuleDescriptorName,
    ClassPrivateName,
    ClassBodyImport,
}

impl From<ImportRefusalReason> for Reason {
    fn from(reason: ImportRefusalReason) -> Self {
        match reason {
            ImportRefusalReason::GetattrModule => Self::GetattrModule,
            ImportRefusalReason::StarImport => Self::StarImport,
            ImportRefusalReason::GlobalWrite => Self::GlobalWrite,
            ImportRefusalReason::MultiWrite => Self::MultiWrite,
            ImportRefusalReason::NestedSingle => Self::NestedSingle,
            ImportRefusalReason::SubmoduleShadow => Self::SubmoduleShadow,
            ImportRefusalReason::ModuleDescriptorName => Self::ModuleDescriptorName,
            ImportRefusalReason::ClassPrivateName => Self::ClassPrivateName,
            ImportRefusalReason::ClassBodyImport => Self::ClassBodyImport,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportRefusal {
    pub reason: ImportRefusalReason,
    pub file: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

type Span = (String, usize, usize);
pub(super) type ImportRefusals = RefCell<BTreeMap<Span, ImportRefusalReason>>;

impl PythonBindings<'_> {
    pub(super) fn record_import_refusal(
        &self,
        file: &ParsedFile,
        span: Node<'_>,
        reason: ImportRefusalReason,
    ) {
        self.refusals
            .borrow_mut()
            .entry((file.path.clone(), span.start_byte(), span.end_byte()))
            .and_modify(|earlier| *earlier = (*earlier).min(reason))
            .or_insert(reason);
    }

    /// After fact extraction, return one refusal per capture or no-effect call span.
    pub(crate) fn import_refusals(&self) -> Vec<ImportRefusal> {
        self.refusals
            .borrow()
            .iter()
            .map(|((file, start_byte, end_byte), reason)| ImportRefusal {
                file: file.clone(),
                start_byte: *start_byte,
                end_byte: *end_byte,
                reason: *reason,
            })
            .collect()
    }
}
