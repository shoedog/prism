//! Attribute a call site in a loaded file to the repo files its callee lives in
//! (#602, Layer A). Uses import bindings and the memoized repo file listing
//! only: no file outside the caller's own is parsed. Language rules live in the
//! `python` and `go` children; JS/TS is here because it is the shared shape.
//!
//! Known limits, kept as documented false negatives rather than guessed:
//! method calls on instances of imported classes, callbacks passed as values,
//! dynamic `require(expr)`, Python wildcard imports, and Rust/C/C++ (later PR).
//! A barrel (`index.js` re-export) is attributed to the barrel file itself;
//! `follow_barrel` takes the one extra hop when the caller already holds the
//! barrel's parse.
use crate::ast::ParsedFile;
use crate::call_graph::{
    is_js_ts_relative_specifier, js_ts_relative_module_candidates, CallSite, ImportBinding,
    ScopeGraphBuildInputs,
};
use crate::js_exports::JsExportTarget;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

mod go;
mod python;

/// More candidate files than this for one specifier is treated as unresolvable.
const MAX_AMBIGUOUS: usize = 4;

/// Where a call's callee lives, judged from import bindings and the repo file
/// listing alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Attribution {
    /// Repo files, sorted, none of them loaded.
    InRepo(Vec<String>),
    /// A bare package, std, or third-party import.
    External,
    /// An in-repo-looking specifier that matches no file or too many.
    Unresolvable { spec: String },
    /// The callee is defined in the caller's file or only in loaded files.
    Local,
    /// No binding explains the call.
    Unknown,
}

/// One re-export step through a barrel module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BarrelHop {
    pub file: String,
    pub member: String,
}

/// Per-file import data, extracted once per caller.
struct FileImports {
    /// Local name -> module path (JS/TS specifier, Python module, Go import path).
    map: BTreeMap<String, String>,
    /// Python only: the typed bindings (module vs member vs alias).
    bindings: Vec<ImportBinding>,
}

pub(crate) struct ImportTargets<'a> {
    inputs: &'a ScopeGraphBuildInputs,
    listing: Arc<BTreeSet<String>>,
    loaded: BTreeSet<String>,
    js_resolver: crate::js_paths::Resolver<'a>,
    imports: BTreeMap<String, Rc<FileImports>>,
    go_packages: Option<go::GoPackages>,
    go_decls: BTreeMap<String, Rc<BTreeSet<String>>>,
}

impl<'a> ImportTargets<'a> {
    /// `loaded` is the set of files already analysed; targets inside it are
    /// not boundaries. `None` when the repo listing is unavailable.
    pub(crate) fn new(inputs: &'a ScopeGraphBuildInputs, loaded: BTreeSet<String>) -> Option<Self> {
        Some(Self {
            listing: inputs.source_listing()?,
            js_resolver: crate::js_paths::Resolver::new(&inputs.js_paths_snapshot),
            inputs,
            loaded,
            imports: BTreeMap::new(),
            go_packages: None,
            go_decls: BTreeMap::new(),
        })
    }

    pub(crate) fn targets_for_call(
        &mut self,
        file: &str,
        parsed: &ParsedFile,
        call: &CallSite,
    ) -> Attribution {
        let raw = match parsed.language {
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                self.js_attribution(file, parsed, call)
            }
            Language::Python => self.py_attribution(file, parsed, call),
            Language::Go => self.go_attribution(file, parsed, call),
            _ => Attribution::Unknown,
        };
        self.finish(file, raw)
    }

    /// Take the re-export step a barrel declares for `member`, using the
    /// barrel's own parse. `None` for anything but a single-file named
    /// re-export (star re-exports, conflicts and local definitions stay put).
    pub(crate) fn follow_barrel(
        &mut self,
        barrel: &str,
        parsed: &ParsedFile,
        member: &str,
    ) -> Option<BarrelHop> {
        let facts = parsed.extract_js_ts_export_facts();
        if facts.conflicted.contains(member) {
            return None;
        }
        let (JsExportTarget::ReExport {
            module_path,
            imported,
        }
        | JsExportTarget::ImportForward {
            module_path,
            imported,
        }) = facts.named.get(member)?
        else {
            return None;
        };
        match self.js_spec(barrel, module_path) {
            Attribution::InRepo(files) if files.len() == 1 => Some(BarrelHop {
                file: files.into_iter().next()?,
                member: imported.clone(),
            }),
            _ => None,
        }
    }

    /// Drop targets that are already analysed (or the caller itself); a call
    /// whose every target is analysed is `Local`. Order is sorted and unique.
    fn finish(&self, file: &str, raw: Attribution) -> Attribution {
        let Attribution::InRepo(targets) = raw else {
            return raw;
        };
        let outside: BTreeSet<String> = targets
            .into_iter()
            .filter(|t| t != file && !self.loaded.contains(t))
            .collect();
        if outside.is_empty() {
            Attribution::Local
        } else {
            Attribution::InRepo(outside.into_iter().collect())
        }
    }

    fn imports_of(&mut self, file: &str, parsed: &ParsedFile) -> Rc<FileImports> {
        if let Some(found) = self.imports.get(file) {
            return found.clone();
        }
        let built = Rc::new(FileImports {
            map: parsed.extract_imports(),
            bindings: if parsed.language == Language::Python {
                parsed.extract_import_bindings()
            } else {
                Vec::new()
            },
        });
        self.imports.insert(file.to_string(), built.clone());
        built
    }

    fn js_attribution(&mut self, file: &str, parsed: &ParsedFile, call: &CallSite) -> Attribution {
        let imports = self.imports_of(file, parsed);
        let key = match call.qualifier.as_deref() {
            Some(qualifier) => match root_identifier(qualifier) {
                Some(root) => root,
                None => return Attribution::Unknown,
            },
            None => call.callee_name.as_str(),
        };
        match imports.map.get(key) {
            Some(spec) => self.js_spec(file, spec),
            None if is_same_file_function(parsed, call) => Attribution::Local,
            None => Attribution::Unknown,
        }
    }

    /// A JS/TS specifier from `file`: relative via the shared candidate list,
    /// non-relative via tsconfig `paths`, else a package.
    fn js_spec(&mut self, file: &str, spec: &str) -> Attribution {
        let spec = spec.trim();
        if is_js_ts_relative_specifier(spec) {
            let found =
                js_ts_relative_module_candidates(spec, file, &self.listing).unwrap_or_default();
            return bounded(found, spec);
        }
        match self.js_resolver.resolve(file, spec, &self.listing) {
            Some(target) => Attribution::InRepo(vec![target]),
            None if self.js_resolver.declares_alias(file, spec) => Attribution::Unresolvable {
                spec: spec.to_string(),
            },
            None => Attribution::External,
        }
    }
}

/// An unqualified call to a function the caller's own file defines.
fn is_same_file_function(parsed: &ParsedFile, call: &CallSite) -> bool {
    call.qualifier.is_none() && parsed.find_function_by_name(&call.callee_name).is_some()
}

/// Candidate files for one specifier: none, or more than the bound, is
/// unresolvable.
fn bounded(found: Vec<String>, spec: &str) -> Attribution {
    if found.is_empty() || found.len() > MAX_AMBIGUOUS {
        Attribution::Unresolvable {
            spec: spec.to_string(),
        }
    } else {
        Attribution::InRepo(found)
    }
}

/// The leading identifier of a qualifier expression when it is a plain dotted
/// (or optional-chained) path: `ns` of `ns.sub`; `None` for `f().x`, `a[0].y`.
fn root_identifier(qualifier: &str) -> Option<&str> {
    let end = qualifier
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(qualifier.len());
    let (root, rest) = qualifier.split_at(end);
    (!root.is_empty() && (rest.is_empty() || rest.starts_with('.') || rest.starts_with("?.")))
        .then_some(root)
}
