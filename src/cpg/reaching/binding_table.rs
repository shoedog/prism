//! Declarative binding and capture classifications.

// Slice 1 populates this shared schema one language at a time.
#![allow(dead_code)]

use crate::ast::ParsedFile;
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use tree_sitter::Node;

mod bash_wave;
mod c;
mod c_wave;
mod capture_rows;
mod cpp;
mod cpp_wave;
mod go;
mod go_wave;
mod java;
mod java_wave;
mod javascript;
mod javascript_wave;
mod lua_wave;
mod python;
mod python_wave;
mod rust;
mod rust_wave;
mod scripting;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Predicate {
    FieldTextIs {
        field: &'static str,
        any_of: &'static [&'static str],
    },
    TypeScriptParameterPattern {
        kind: &'static str,
    },
    NestedFieldChildKindIs {
        field: &'static str,
        nested_field: &'static str,
        any_of: &'static [&'static str],
    },
    OperatorIs {
        any_of: &'static [&'static str],
    },
    ParentKindIs {
        kind: &'static str,
    },
    AncestorKindIs {
        kind: &'static str,
    },
    HasField {
        field: &'static str,
    },
    JavaScriptDestructuringDeclarator {
        lexical: bool,
    },
    JavaScriptBindingPattern,
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
pub(crate) struct GuardSpanRow {
    pub binder_kind: &'static str,
    pub guard_kind: &'static str,
    pub owner_kind: &'static str,
    pub condition_field: &'static str,
    pub positive_fields: &'static [&'static str],
    pub negative_fields: &'static [&'static str],
    pub positive_condition_tail: bool,
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
    pub(super) fn reuses_binding_in_scope(self, language: Language) -> bool {
        matches!(
            self,
            Self::GoShort | Self::JavaScriptVar | Self::PythonAssignment | Self::Reuse
        ) || (language == Language::Python && self == Self::Other)
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
    macro_rules! combined3 {
        ($cell:ident, $first:expr, $second:expr, $third:expr) => {{
            static $cell: OnceLock<Vec<BindingRow>> = OnceLock::new();
            $cell.get_or_init(|| {
                $first
                    .iter()
                    .chain($second)
                    .chain($third)
                    .copied()
                    .collect()
            })
        }};
    }
    match language {
        Language::Python => combined3!(
            PYTHON_ROWS,
            python::ROWS,
            python_wave::CURATED_ROWS,
            python_wave::ROWS
        ),
        Language::JavaScript => combined3!(
            JAVASCRIPT_ROWS,
            javascript::ROWS,
            javascript_wave::CLASSIFIED_ROWS,
            javascript_wave::ROWS
        ),
        Language::TypeScript => {
            static TYPESCRIPT_ROWS: OnceLock<Vec<BindingRow>> = OnceLock::new();
            TYPESCRIPT_ROWS.get_or_init(|| {
                typescript_wave::CURATED_ROWS
                    .iter()
                    .chain(
                        javascript::ROWS
                            .iter()
                            .filter(|row| !typescript_wave::replaces_javascript_row(row.kind)),
                    )
                    .chain(typescript_wave::ROWS)
                    .copied()
                    .collect()
            })
        }
        Language::Tsx => {
            static TSX_ROWS: OnceLock<Vec<BindingRow>> = OnceLock::new();
            TSX_ROWS.get_or_init(|| {
                tsx_wave::CURATED_ROWS
                    .iter()
                    .chain(
                        javascript::ROWS
                            .iter()
                            .filter(|row| !tsx_wave::replaces_javascript_row(row.kind)),
                    )
                    .chain(tsx_wave::ROWS)
                    .copied()
                    .collect()
            })
        }
        Language::Go => combined3!(GO_ROWS, go::ROWS, go_wave::CURATED_ROWS, go_wave::ROWS),
        Language::Java => combined3!(
            JAVA_ROWS,
            java::JAVA_ROWS,
            java_wave::CURATED_ROWS,
            java_wave::ROWS
        ),
        Language::C => combined!(C_ROWS, c::C_ROWS, c_wave::ROWS),
        Language::Cpp => combined!(CPP_ROWS, cpp::CPP_ROWS, cpp_wave::ROWS),
        Language::Rust => combined3!(
            RUST_ROWS,
            rust::ROWS,
            rust_wave::CURATED_ROWS,
            rust_wave::ROWS
        ),
        Language::Lua => combined!(LUA_ROWS, scripting::LUA_ROWS, lua_wave::ROWS),
        Language::Terraform => terraform_wave::ROWS,
        Language::Bash => bash_wave::ROWS,
    }
}

pub(crate) fn guard_span_row(
    language: Language,
    binder_kind: &str,
) -> Option<&'static GuardSpanRow> {
    match language {
        Language::Java => java_wave::PATTERN_GUARD_SPANS
            .iter()
            .find(|row| row.binder_kind == binder_kind),
        _ => None,
    }
}

pub(crate) fn java_binding_identifiers(node: Node<'_>) -> Vec<Node<'_>> {
    java_wave::binding_identifiers(node)
}

pub(crate) fn java_statement_cannot_complete_normally(statement: Option<Node<'_>>) -> bool {
    java_wave::statement_cannot_complete_normally(statement)
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
        Predicate::TypeScriptParameterPattern { kind } => {
            if parsed.language != Language::TypeScript
                || node.child_by_field_name("name").is_some()
                || node.child_by_field_name("decorator").is_some()
                || node.child_by_field_name("value").is_some()
                || node
                    .child_by_field_name("pattern")
                    .is_none_or(|pattern| pattern.kind() != *kind)
            {
                return false;
            }
            let mut cursor = node.walk();
            let has_parameter_property_modifier = node.children(&mut cursor).any(|child| {
                matches!(
                    child.kind(),
                    "accessibility_modifier" | "override_modifier" | "readonly"
                )
            });
            !has_parameter_property_modifier
        }
        Predicate::NestedFieldChildKindIs {
            field,
            nested_field,
            any_of,
        } => node
            .child_by_field_name(field)
            .and_then(|child| child.child_by_field_name(nested_field))
            .is_some_and(|nested| {
                let mut cursor = nested.walk();
                let matches = nested
                    .named_children(&mut cursor)
                    .any(|child| any_of.contains(&child.kind()));
                matches
            }),
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
        Predicate::AncestorKindIs { kind } => {
            let mut current = node.parent();
            let mut found = false;
            while let Some(ancestor) = current {
                if ancestor.kind() == *kind {
                    found = true;
                    break;
                }
                current = ancestor.parent();
            }
            found
        }
        Predicate::HasField { field } => node.child_by_field_name(field).is_some(),
        Predicate::JavaScriptDestructuringDeclarator { lexical } => {
            node.child_by_field_name("name")
                .is_some_and(|name| matches!(name.kind(), "object_pattern" | "array_pattern"))
                && node
                    .parent()
                    .is_some_and(|parent| (parent.kind() == "lexical_declaration") == *lexical)
        }
        Predicate::JavaScriptBindingPattern => {
            let mut current = node.parent();
            while let Some(ancestor) = current {
                match ancestor.kind() {
                    "variable_declarator" | "formal_parameters" => {
                        return true;
                    }
                    "for_in_statement" => {
                        return ancestor.child_by_field_name("kind").is_some_and(|kind| {
                            matches!(parsed.node_text(&kind), "let" | "const")
                        });
                    }
                    "catch_clause"
                    | "assignment_expression"
                    | "function_declaration"
                    | "function_expression"
                    | "arrow_function"
                    | "method_definition"
                    | "generator_function"
                    | "generator_function_declaration"
                    | "class_static_block" => return false,
                    _ => current = ancestor.parent(),
                }
            }
            false
        }
        Predicate::IsImmediatelyInvoked => {
            let mut callee = node;
            while callee
                .parent()
                .is_some_and(|parent| parent.kind() == "parenthesized_expression")
            {
                callee = callee.parent().expect("checked parent");
            }
            callee.parent().is_some_and(|call| {
                let call_kind = if parsed.language == Language::Python {
                    "call"
                } else {
                    "call_expression"
                };
                call.kind() == call_kind
                    && call
                        .child_by_field_name("function")
                        .is_some_and(|function| function == callee)
            })
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
