//! Declarative binding and capture classifications.

// Slice 1 populates this shared schema one language at a time.
#![allow(dead_code)]

use crate::ast::ParsedFile;
use crate::languages::Language;
use sha2::{Digest, Sha256};
use tree_sitter::Node;

mod capture_rows;
mod go;
mod javascript;
mod other;
mod python;
mod rust;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Scope,
    Binding,
    NotBinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RoleSet(u8);

impl RoleSet {
    pub(crate) const fn of(roles: &[Role]) -> Self {
        let mut bits = 0;
        let mut index = 0;
        while index < roles.len() {
            bits |= match roles[index] {
                Role::Scope => 1,
                Role::Binding => 2,
                Role::NotBinding => 4,
            };
            index += 1;
        }
        Self(bits)
    }

    pub(crate) const fn has(self, role: Role) -> bool {
        let bit = match role {
            Role::Scope => 1,
            Role::Binding => 2,
            Role::NotBinding => 4,
        };
        self.0 & bit != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Predicate {
    FieldTextIs {
        field: &'static str,
        any_of: &'static [&'static str],
    },
    OperatorIs {
        any_of: &'static [&'static str],
    },
    ParentKindIs {
        kind: &'static str,
    },
    HasField {
        field: &'static str,
    },
    IsImmediatelyInvoked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Visibility {
    WholeScope,
    AfterIntroduction,
    Header,
    Comprehension,
    Custom(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ruling {
    Classified,
    Uncertain {
        reason: &'static str,
        revisit: &'static str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Timing {
    Deferred,
    Immediate,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeclarationKind {
    Parameter,
    GoShort,
    JavaScriptVar,
    PythonAssignment,
    Reuse,
    Other,
    Pattern,
    CaptureCopy,
}

impl DeclarationKind {
    pub(super) fn reuses_binding_in_scope(self) -> bool {
        matches!(
            self,
            Self::GoShort | Self::JavaScriptVar | Self::PythonAssignment | Self::Reuse
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingRow {
    pub language: Language,
    pub kind: &'static str,
    pub variant: Option<Predicate>,
    pub roles: RoleSet,
    pub fields: &'static [&'static str],
    pub declaration: Option<DeclarationKind>,
    pub visibility: Visibility,
    pub ruling: Ruling,
    pub regression: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CaptureRow {
    pub language: Language,
    pub kind: &'static str,
    pub variant: Option<Predicate>,
    pub timing: Timing,
    pub regression: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BindingScopeRule {
    pub(super) creates_scope: bool,
    pub(super) declaration: Option<DeclarationKind>,
}

pub(crate) fn rows(language: Language) -> &'static [BindingRow] {
    match language {
        Language::Go => go::ROWS,
        Language::JavaScript | Language::TypeScript | Language::Tsx => javascript::ROWS,
        Language::Python => python::ROWS,
        Language::Rust => rust::ROWS,
        Language::Java => other::JAVA_ROWS,
        Language::C => other::C_ROWS,
        Language::Cpp => other::CPP_ROWS,
        Language::Lua => other::LUA_ROWS,
        Language::Terraform | Language::Bash => &[],
    }
}

pub(crate) fn capture_rows(language: Language) -> &'static [CaptureRow] {
    capture_rows::rows(language)
}

pub(crate) fn select_binding_row(
    parsed: &ParsedFile,
    node: Node<'_>,
) -> Option<&'static BindingRow> {
    let mut candidates = rows(parsed.language)
        .iter()
        .filter(|row| row.kind == node.kind());
    candidates
        .clone()
        .find(|row| {
            row.variant
                .as_ref()
                .is_some_and(|predicate| predicate_matches(parsed, node, predicate))
        })
        .or_else(|| candidates.find(|row| row.variant.is_none()))
}

pub(crate) fn select_capture_row(
    parsed: &ParsedFile,
    node: Node<'_>,
) -> Option<&'static CaptureRow> {
    let mut candidates = capture_rows(parsed.language)
        .iter()
        .filter(|row| row.kind == node.kind());
    candidates
        .clone()
        .find(|row| {
            row.variant
                .as_ref()
                .is_some_and(|predicate| predicate_matches(parsed, node, predicate))
        })
        .or_else(|| candidates.find(|row| row.variant.is_none()))
}

pub(crate) fn predicate_matches(parsed: &ParsedFile, node: Node<'_>, p: &Predicate) -> bool {
    match p {
        Predicate::FieldTextIs { field, any_of } => node
            .child_by_field_name(field)
            .is_some_and(|child| any_of.contains(&parsed.node_text(&child))),
        Predicate::OperatorIs { any_of } => {
            let mut cursor = node.walk();
            let matches = node
                .children(&mut cursor)
                .any(|child| any_of.contains(&child.kind()));
            matches
        }
        Predicate::ParentKindIs { kind } => {
            node.parent().is_some_and(|parent| parent.kind() == *kind)
        }
        Predicate::HasField { field } => node.child_by_field_name(field).is_some(),
        Predicate::IsImmediatelyInvoked => {
            // Task 19 adds call-site-aware matching for immediately invoked constructs.
            false
        }
    }
}

pub(crate) fn grammar_digest(language: Language) -> String {
    format!(
        "{:x}",
        Sha256::digest(language.node_types_json().as_bytes())
    )
}

pub(crate) fn pinned_digest(_language: Language) -> &'static str {
    ""
}
