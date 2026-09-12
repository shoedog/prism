//! Declarative binding and capture classifications.

// Slice 1 populates this shared schema one language at a time.
#![allow(dead_code)]

use crate::ast::ParsedFile;
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use tree_sitter::Node;

mod bash_wave;
mod c_wave;
mod capture_rows;
mod cpp_wave;
mod go;
mod go_wave;
mod java_wave;
mod javascript;
mod javascript_wave;
mod lua_wave;
mod other;
mod python;
mod python_wave;
mod rust;
mod rust_wave;
mod terraform_wave;
mod tsx_wave;
mod typescript_wave;

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

macro_rules! provisional_rows {
    ($language:expr, $revisit:literal, [$(($kind:literal, $regression:literal)),* $(,)?]) => {
        &[$(BindingRow {
            language: $language,
            kind: $kind,
            variant: None,
            roles: super::RoleSet::of(&[super::Role::Binding]),
            fields: &[],
            declaration: None,
            visibility: super::Visibility::WholeScope,
            ruling: super::Ruling::Uncertain {
                reason: "not yet curated",
                revisit: $revisit,
            },
            regression: $regression,
        }),*]
    };
}
pub(super) use provisional_rows;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CaptureRow {
    pub language: Language,
    pub kind: &'static str,
    pub variant: Option<Predicate>,
    pub timing: Timing,
    pub regression: &'static str,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct CensusKind {
    pub kind: String,
    pub named: bool,
    pub heuristic_flags: Vec<String>,
    pub table_row: bool,
    pub candidate: bool,
    pub grammar_only: bool,
    pub corpus_occurrences: usize,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Census {
    pub digest: String,
    pub kinds: Vec<CensusKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BindingScopeRule {
    pub(super) creates_scope: bool,
    pub(super) declaration: Option<DeclarationKind>,
}

pub(crate) fn rows(language: Language) -> &'static [BindingRow] {
    macro_rules! combined {
        ($cell:ident, $base:expr, $wave:expr) => {{
            static $cell: OnceLock<Vec<BindingRow>> = OnceLock::new();
            $cell.get_or_init(|| $base.iter().chain($wave).copied().collect())
        }};
    }
    match language {
        Language::Python => combined!(PYTHON_ROWS, python::ROWS, python_wave::ROWS),
        Language::JavaScript => combined!(JAVASCRIPT_ROWS, javascript::ROWS, javascript_wave::ROWS),
        Language::TypeScript => combined!(TYPESCRIPT_ROWS, javascript::ROWS, typescript_wave::ROWS),
        Language::Tsx => combined!(TSX_ROWS, javascript::ROWS, tsx_wave::ROWS),
        Language::Go => combined!(GO_ROWS, go::ROWS, go_wave::ROWS),
        Language::Java => combined!(JAVA_ROWS, other::JAVA_ROWS, java_wave::ROWS),
        Language::C => combined!(C_ROWS, other::C_ROWS, c_wave::ROWS),
        Language::Cpp => combined!(CPP_ROWS, other::CPP_ROWS, cpp_wave::ROWS),
        Language::Rust => combined!(RUST_ROWS, rust::ROWS, rust_wave::ROWS),
        Language::Lua => combined!(LUA_ROWS, other::LUA_ROWS, lua_wave::ROWS),
        Language::Terraform => terraform_wave::ROWS,
        Language::Bash => bash_wave::ROWS,
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

pub(crate) fn pinned_digest(language: Language) -> &'static str {
    match language {
        Language::Python => python_wave::DIGEST,
        Language::JavaScript => javascript_wave::DIGEST,
        Language::TypeScript => typescript_wave::DIGEST,
        Language::Tsx => tsx_wave::DIGEST,
        Language::Go => go_wave::DIGEST,
        Language::Java => java_wave::DIGEST,
        Language::C => c_wave::DIGEST,
        Language::Cpp => cpp_wave::DIGEST,
        Language::Rust => rust_wave::DIGEST,
        Language::Lua => lua_wave::DIGEST,
        Language::Terraform => terraform_wave::DIGEST,
        Language::Bash => bash_wave::DIGEST,
    }
}

pub(crate) fn census_artifact_digest(language: Language) -> String {
    let source = match language {
        Language::Python => include_str!("binding_table/census/python.json"),
        Language::JavaScript => include_str!("binding_table/census/javascript.json"),
        Language::TypeScript => include_str!("binding_table/census/typescript.json"),
        Language::Tsx => include_str!("binding_table/census/tsx.json"),
        Language::Go => include_str!("binding_table/census/go.json"),
        Language::Java => include_str!("binding_table/census/java.json"),
        Language::C => include_str!("binding_table/census/c.json"),
        Language::Cpp => include_str!("binding_table/census/cpp.json"),
        Language::Rust => include_str!("binding_table/census/rust.json"),
        Language::Lua => include_str!("binding_table/census/lua.json"),
        Language::Terraform => include_str!("binding_table/census/terraform.json"),
        Language::Bash => include_str!("binding_table/census/bash.json"),
    };
    format!("{:x}", Sha256::digest(source.as_bytes()))
}

pub(crate) fn pinned_census_digest(language: Language) -> &'static str {
    match language {
        Language::Python => python_wave::CENSUS_DIGEST,
        Language::JavaScript => javascript_wave::CENSUS_DIGEST,
        Language::TypeScript => typescript_wave::CENSUS_DIGEST,
        Language::Tsx => tsx_wave::CENSUS_DIGEST,
        Language::Go => go_wave::CENSUS_DIGEST,
        Language::Java => java_wave::CENSUS_DIGEST,
        Language::C => c_wave::CENSUS_DIGEST,
        Language::Cpp => cpp_wave::CENSUS_DIGEST,
        Language::Rust => rust_wave::CENSUS_DIGEST,
        Language::Lua => lua_wave::CENSUS_DIGEST,
        Language::Terraform => terraform_wave::CENSUS_DIGEST,
        Language::Bash => bash_wave::CENSUS_DIGEST,
    }
}

pub(crate) fn census(language: Language) -> &'static Census {
    macro_rules! load {
        ($cell:ident, $file:literal) => {{
            static $cell: OnceLock<Census> = OnceLock::new();
            $cell.get_or_init(|| {
                serde_json::from_str(include_str!($file))
                    .expect(concat!("valid binding census: ", $file))
            })
        }};
    }

    match language {
        Language::Python => load!(PYTHON, "binding_table/census/python.json"),
        Language::JavaScript => load!(JAVASCRIPT, "binding_table/census/javascript.json"),
        Language::TypeScript => load!(TYPESCRIPT, "binding_table/census/typescript.json"),
        Language::Tsx => load!(TSX, "binding_table/census/tsx.json"),
        Language::Go => load!(GO, "binding_table/census/go.json"),
        Language::Java => load!(JAVA, "binding_table/census/java.json"),
        Language::C => load!(C, "binding_table/census/c.json"),
        Language::Cpp => load!(CPP, "binding_table/census/cpp.json"),
        Language::Rust => load!(RUST, "binding_table/census/rust.json"),
        Language::Lua => load!(LUA, "binding_table/census/lua.json"),
        Language::Terraform => load!(TERRAFORM, "binding_table/census/terraform.json"),
        Language::Bash => load!(BASH, "binding_table/census/bash.json"),
    }
}
