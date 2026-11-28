//! AST inventory of Python dependency calls with statically proven identity.

#[cfg(any(test, feature = "inventory-evidence"))]
pub mod evidence;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_receiver_evidence;

mod load;
mod py_receiver;
mod py_tables;
mod python;
mod rows;

pub use load::load_python_files;
pub use python::function_bounds;
pub(super) use rows::{inventory_document, merge_inventory};

use prism::ast::ParsedFile;
use prism::finding_confidence::ParseQuality;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Family {
    Http,
    Filesystem,
    Parse,
    Config,
}

impl Family {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Filesystem => "filesystem",
            Self::Parse => "parse",
            Self::Config => "config",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [Self::Http, Self::Filesystem, Self::Parse, Self::Config]
            .into_iter()
            .find(|family| family.as_str() == s)
    }

    pub fn supported() -> &'static [Self] {
        &[Self::Http, Self::Filesystem, Self::Parse, Self::Config]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Grade {
    Exact,
    AnnotationTyped,
}

#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct InventoryOptions {
    pub families: BTreeSet<Family>,
}

#[non_exhaustive]
pub struct InventorySite {
    pub file: String,
    pub line: usize,
    pub symbol: String,
    pub function_start_line: usize,
    pub function_end_line: usize,
    pub family: Family,
    pub resolved_callee: String,
    pub source_callee: String,
    pub grade: Grade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum UnsupportedReason {
    ParseErrorInFunction,
    ModuleLevel,
    ClassBodyFrame,
    LambdaFrame,
    MultilineCallee,
    NotLineAnchor,
}

impl UnsupportedReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParseErrorInFunction => "parse_error_in_function",
            Self::ModuleLevel => "module_level",
            Self::ClassBodyFrame => "class_body_frame",
            Self::LambdaFrame => "lambda_frame",
            Self::MultilineCallee => "multiline_callee",
            Self::NotLineAnchor => "not_line_anchor",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum UnprovenCause {
    StarImport,
    ShadowedRoot,
    NotImported,
    RepositoryLocalModule,
    BindingUnavailable,
    MemberRebound,
    ReceiverNotUnique,
    ReceiverIsNone,
    AnnotationConflict,
    HelperUnannotated,
    HelperNotAdmissible,
    ReceiverValueUnproven,
}

impl UnprovenCause {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StarImport => "star_import",
            Self::ShadowedRoot => "shadowed_root",
            Self::NotImported => "not_imported",
            Self::RepositoryLocalModule => "repository_local_module",
            Self::BindingUnavailable => "binding_unavailable",
            Self::MemberRebound => "member_rebound",
            Self::ReceiverNotUnique => "receiver_not_unique",
            Self::ReceiverIsNone => "receiver_is_none",
            Self::AnnotationConflict => "annotation_conflict",
            Self::HelperUnannotated => "helper_unannotated",
            Self::HelperNotAdmissible => "helper_not_admissible",
            Self::ReceiverValueUnproven => "receiver_value_unproven",
        }
    }
}

#[non_exhaustive]
pub struct UnsupportedSite {
    pub file: String,
    pub line: usize,
    pub symbol: Option<String>,
    pub family: Family,
    pub resolved_callee: String,
    pub reason: UnsupportedReason,
    pub grade: Grade,
}

#[derive(Default)]
#[non_exhaustive]
pub struct InventoryOutput {
    pub sites: Vec<InventorySite>,
    pub unsupported: Vec<UnsupportedSite>,
    pub parse_quality: BTreeMap<String, ParseQuality>,
    pub unproven: BTreeMap<String, BTreeMap<UnprovenCause, usize>>,
    pub skipped: Vec<(String, String)>,
    pub files: Vec<String>,
    #[cfg(feature = "inventory-evidence")]
    pub occurrences: Vec<evidence::Occurrence>,
    #[cfg(all(test, not(feature = "inventory-evidence")))]
    occurrences: Vec<evidence::Occurrence>,
}

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FunctionBounds {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn collect(
    files: &BTreeMap<String, ParsedFile>,
    repo_root: &Path,
    opts: &InventoryOptions,
) -> InventoryOutput {
    let mut output = InventoryOutput::default();
    let selected = Family::supported()
        .iter()
        .copied()
        .filter(|family| opts.families.is_empty() || opts.families.contains(family))
        .collect();
    let mut selected_files = BTreeMap::new();
    for (file, parsed) in files {
        if parsed.language != prism::languages::Language::Python {
            continue;
        }
        if std::path::MAIN_SEPARATOR != '\\' && file.contains('\\') {
            output
                .skipped
                .push((file.clone(), "UnrepresentablePath".to_owned()));
            continue;
        }
        selected_files.insert(normalize_path(file), parsed);
    }
    for (file, parsed) in selected_files {
        python::collect_file(&file, parsed, repo_root, &selected, &mut output);
        output.files.push(file);
    }
    output
}

fn normalize_path(file: &str) -> String {
    file.replace('\\', "/").trim_start_matches("./").to_owned()
}
