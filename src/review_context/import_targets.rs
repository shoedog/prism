//! Attribute a call site in a loaded file to the repo files its callee lives in
//! (#602, Layer A). Uses import bindings and the repo file listing only: no
//! file outside the caller's own is parsed (Go reads sibling files as text).
//! Language rules live in the `js`, `python` and `go` children.
//!
//! # Contract: a best-effort loading hint, not a proof
//!
//! An attribution says where a callee *probably* lives so the review can load
//! those files (Layer A notes, Layer B loading). It is never evidence. A wrong
//! attribution costs an extra loaded file (time), a file not loaded (a catch
//! that is not reported today either), or the wording of a note; it can never
//! produce a wrong finding, because every finding comes from prism's own
//! resolution over the files that end up loaded. Consumers must not treat a
//! result as a binding proof, and must word it as a hint (PR-2):
//! - `InRepo` with `Basis::Bound`: "likely in X";
//! - `InRepo` with `Basis::Candidates`: "one of X, Y"; `files[0]` is the
//!   preferred candidate (the file the language itself tries first), and
//!   `member` is `None` whenever the list mixes files the callee name does not
//!   apply to;
//! - after `finish` drops entries that are already loaded, a `Candidates` list
//!   may hold a single file: the others were loaded. Word it like `Bound`;
//! - `Unresolvable`: "looks in-repo, file unknown"; `Unknown`: say nothing;
//!   `External`: "outside the repository".
//!
//! Option 2 (attribution backed by the navigation index, for MR-stage and
//! preloaded reviews that have no import-only view) is tracked separately as
//! #623 and is not implemented here.
//!
//! Three things are kept apart, never inferred from one another:
//! - a *candidate* attribution (`InRepo` with `Basis::Candidates`): several
//!   files could own the name and the import statement does not decide;
//! - a *bound* attribution (`InRepo` with `Basis::Bound`): the import binds the
//!   callee name to exactly those files, by static reading (it does not assert
//!   runtime truth);
//! - *unavailable proof* (`Unresolvable`): the specifier looks in-repo but no
//!   resolver result exists. A strict resolver's refusal is never `External`;
//!   a config that cannot be read in full (tsconfig `extends` chain) is
//!   `Unknown`, never `External`.
//!
//! A call whose root name is declared in a nearer scope than the import (a
//! parameter, a local, a nested function, a Go type-switch alias, a Python
//! comprehension target at the site) is `Unknown`: the import spelling is not
//! what the site calls. JS/TS reads the spelling from the program-scope import
//! index only, so a nested `require` of the same name cannot rebind it.
//!
//! PR-2 should keep one `JsBindingCache` per file across the sites it
//! attributes; each `targets_for_call` builds one per site today.
//!
//! Known limits, kept as documented false negatives rather than guessed:
//! method calls on instances of imported classes, callbacks passed as values,
//! dynamic `require(expr)`, a JS/TS `require` declared inside a function or
//! block (a nearer scope; a top-level `var` inside a block is not indexed),
//! Python wildcard imports and class-body calls, Go packages whose name
//! differs from the last import-path element, a `_test.go` caller reaching a
//! helper in a sibling `_test.go` (test files are not scanned: `Unknown`), and
//! Rust/C/C++ (later PR). tsconfig `paths` fallbacks with more than one
//! surviving target are `Candidates` with the first fallback's file preferred,
//! even when the first one exists; two wildcard `paths` keys tied on their
//! literal prefix are `Unknown` (TypeScript takes the first in source order,
//! which the JSON reader does not keep); a package `extends` is ignored. A
//! barrel (`index.js` re-export) is attributed to the barrel file itself;
//! `follow_barrel` takes the one extra hop when the caller already holds the
//! barrel's parse.
use crate::ast::ParsedFile;
use crate::call_graph::{CallSite, ImportBinding};
use crate::ast::js_ts::exports::JsExportTarget;
use crate::languages::Language;
use crate::repo_loader::{ListingError, SourceListing};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

mod go;
mod go_scan;
mod js;
mod js_alias;
mod python;
mod shadow;
mod stdlib_names;

/// More candidate files than this for one specifier is treated as unresolvable.
pub(super) const MAX_AMBIGUOUS: usize = 4;

/// How firmly the import statement pins the files of an `InRepo` result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Basis {
    /// The binding names these files (an exact module, or a whole Go package):
    /// "likely in X".
    Bound,
    /// Several files could own the name and the statement does not decide
    /// (a Python package attribute against a same-named submodule or a deeper
    /// chain, tsconfig `paths` fallbacks, Go build-tag variants, a Go callee no
    /// scan placed in one file): "one of X, Y". `files` is in the order the
    /// language itself tries them, so `files[0]` is the preferred candidate.
    Candidates,
}

/// Why a looks-in-repo specifier has no usable file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Why {
    /// Nothing in the repo listing matches (or only a skipped directory does).
    NoFile,
    /// More candidate files or directories than `MAX_AMBIGUOUS`.
    TooMany,
    /// A Go `replace` the module graph could not prove: the import may live in
    /// a repo directory whose identity is unknown.
    ReplaceUnproven,
}

/// Repo files outside the analysed set that may hold a call's callee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InRepo {
    /// Files none of which is loaded or the caller. For one specifier: the
    /// precedence winner (JS/TS, Python), the per-fallback winners in order
    /// (tsconfig `paths`), or every non-test file of the package, sorted (Go).
    pub files: Vec<String>,
    /// The import specifier, Python module or Go import path ("" for a Go
    /// same-package call, which has none).
    pub spec: String,
    /// The exported name the call reaches (`f` of `import { f as g }`, `ns.f`,
    /// `default`), when the binding says.
    pub member: Option<String>,
    /// Go: the files among `files` whose text declares `member` as a top-level
    /// function. Empty when not known.
    pub declaring: Vec<String>,
    pub basis: Basis,
}

/// Where a call's callee lives, judged from import bindings and the repo file
/// listing alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Attribution {
    InRepo(InRepo),
    /// A bare package, std, or third-party import.
    External,
    /// A specifier that looks in-repo (relative, a declared tsconfig alias, an
    /// in-repo top-level Python package, the repo's own Go module path) for
    /// which no single file (or a bounded few) can be named.
    Unresolvable {
        spec: String,
        why: Why,
    },
    /// The callee is defined in the caller's own file.
    SameFile,
    /// The callee's files are all already analysed (loaded diff files).
    Loaded(Vec<String>),
    /// No binding explains the call, or a nearer scope shadows the import.
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
    /// Typed bindings (module vs member vs alias), JS/TS and Python.
    bindings: Vec<ImportBinding>,
}

pub(crate) struct ImportTargets<'a> {
    inputs: &'a crate::call_graph::ScopeGraphBuildInputs,
    listing: Arc<BTreeSet<String>>,
    loaded: BTreeSet<String>,
    js_alias: js_alias::AliasConfigs,
    imports: BTreeMap<String, Rc<FileImports>>,
    go_packages: Option<go::GoPackages>,
    go_decls: BTreeMap<String, Rc<BTreeSet<String>>>,
}

impl<'a> ImportTargets<'a> {
    /// `loaded` is the set of files already analysed; targets inside it are
    /// not boundaries. Fails with the reason when the repo listing could not
    /// be built, so the caller can surface a warning instead of silence.
    pub(crate) fn new(
        inputs: &'a crate::call_graph::ScopeGraphBuildInputs,
        listing: &SourceListing,
        loaded: BTreeSet<String>,
    ) -> Result<Self, ListingError> {
        Ok(Self {
            listing: listing.clone()?,
            js_alias: js_alias::AliasConfigs::default(),
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
        match self.js_spec(barrel, module_path, None) {
            Attribution::InRepo(hit) if hit.files.len() == 1 => Some(BarrelHop {
                file: hit.files.into_iter().next()?,
                member: imported.clone(),
            }),
            _ => None,
        }
    }

    /// Separate targets that are already analysed (or the caller itself) from
    /// those that are not. Where a declaring file is known (Go) it decides:
    /// every declarer analysed means the callee is not a boundary. Order is
    /// preserved.
    fn finish(&self, file: &str, raw: Attribution) -> Attribution {
        let Attribution::InRepo(mut hit) = raw else {
            return raw;
        };
        let partition = |paths: &[String]| {
            let (mut outside, mut loaded, mut own) = (Vec::new(), Vec::new(), false);
            for path in paths {
                if path == file {
                    own = true;
                } else if self.loaded.contains(path) {
                    loaded.push(path.clone());
                } else {
                    outside.push(path.clone());
                }
            }
            (outside, loaded, own)
        };
        let (declared_out, declared_loaded, declared_own) = partition(&hit.declaring);
        if !hit.declaring.is_empty() && declared_out.is_empty() {
            return if declared_loaded.is_empty() && declared_own {
                Attribution::SameFile
            } else {
                Attribution::Loaded(declared_loaded)
            };
        }
        let (outside, loaded, own) = partition(&hit.files);
        if outside.is_empty() {
            return if loaded.is_empty() && own {
                Attribution::SameFile
            } else {
                Attribution::Loaded(loaded)
            };
        }
        hit.files = outside;
        hit.declaring = declared_out;
        Attribution::InRepo(hit)
    }

    fn imports_of(&mut self, file: &str, parsed: &ParsedFile) -> Rc<FileImports> {
        if let Some(found) = self.imports.get(file) {
            return found.clone();
        }
        let built = Rc::new(FileImports {
            map: if matches!(
                parsed.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            ) {
                // Program scope only: a nested `require` must not overwrite the
                // spelling of the import the site walk found.
                parsed.js_ts_program_imports()
            } else {
                parsed.extract_imports()
            },
            bindings: if matches!(
                parsed.language,
                Language::Python | Language::JavaScript | Language::TypeScript | Language::Tsx
            ) {
                parsed.extract_import_bindings()
            } else {
                Vec::new()
            },
        });
        self.imports.insert(file.to_string(), built.clone());
        built
    }
}

/// A proven single-file result.
pub(super) fn bound(files: Vec<String>, spec: &str, member: Option<String>) -> Attribution {
    Attribution::InRepo(InRepo {
        files,
        spec: spec.to_string(),
        member,
        declaring: Vec::new(),
        basis: Basis::Bound,
    })
}

pub(super) fn unresolvable(spec: &str, why: Why) -> Attribution {
    Attribution::Unresolvable {
        spec: spec.to_string(),
        why,
    }
}

/// An unqualified call to a function the caller's own file defines.
pub(super) fn is_same_file_function(parsed: &ParsedFile, call: &CallSite) -> bool {
    call.qualifier.is_none() && parsed.find_function_by_name(&call.callee_name).is_some()
}

/// The leading identifier of a qualifier expression when it is a plain dotted
/// (or optional-chained) path: `ns` of `ns.sub`; `None` for `f().x`, `a[0].y`.
pub(super) fn root_identifier(qualifier: &str) -> Option<&str> {
    let end = qualifier
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(qualifier.len());
    let (root, rest) = qualifier.split_at(end);
    (!root.is_empty() && (rest.is_empty() || rest.starts_with('.') || rest.starts_with("?.")))
        .then_some(root)
}
