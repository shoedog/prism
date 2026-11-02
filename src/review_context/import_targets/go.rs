//! Go attribution: a qualified call goes to its import path's package
//! directory (every non-test file, so same-package name pools stay complete),
//! with the files whose text declares the callee named in `declaring`; an
//! unqualified call goes to the same-package sibling that declares it.
//!
//! An import path with no matching directory is `External`, except that one
//! under the repo's own module path (`Unresolvable`: a generated or deleted
//! package) or under a `replace` the module graph could not prove
//! (`Unresolvable`, `ReplaceUnproven`: the replacement may be a repo
//! directory) is not claimed to be a third-party package. A local variable,
//! parameter or type-switch alias named like the qualifier or the callee makes
//! the site `Unknown`.
//!
//! Declaring evidence applies only to a direct package selector (`util.Run`).
//! A longer selector (`util.Obj.Run`) names the package files as `Candidates`
//! with no declaring file. A direct selector no scan places (a `var` function
//! value, a type conversion) is `Candidates` over the package files, and
//! `Unknown` once any package file is already loaded (that file may hold it).
//! Several declarers (build tags) are `Candidates`.
use super::go_scan::declared_funcs;
use super::shadow::go_scope_binds;
use super::{
    is_same_file_function, root_identifier, unresolvable, Attribution, Basis, ImportTargets,
    InRepo, Why,
};
use crate::ast::ParsedFile;
use crate::call_graph::CallSite;
use crate::go_module_graph::GoModuleGraph;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// Package directories of the repo, keyed both ways.
pub(super) struct GoPackages {
    /// Import path -> directories claiming it (more than one is ambiguous).
    by_import_path: BTreeMap<String, Vec<String>>,
    /// Directory -> its non-test `.go` files, sorted.
    files_by_dir: BTreeMap<String, Vec<String>>,
    /// Module paths of every `go.mod` the graph can name an identity for.
    own_modules: Vec<String>,
    /// Module paths with a `replace` the graph could not prove.
    unproven_replaces: BTreeSet<String>,
}

impl GoPackages {
    fn build(
        inputs: &crate::call_graph::ScopeGraphBuildInputs,
        listing: &BTreeSet<String>,
    ) -> Self {
        let mut files_by_dir: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for path in listing {
            let in_testdata = path.split('/').any(|part| part == "testdata");
            if path.ends_with(".go") && !path.ends_with("_test.go") && !in_testdata {
                let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
                files_by_dir
                    .entry(dir.to_string())
                    .or_default()
                    .push(path.clone());
            }
        }
        let mut graph = GoModuleGraph::new(&inputs.repo_root, &inputs.manifest_snapshot);
        let mut by_import_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for dir in files_by_dir.keys() {
            if let Ok(import_path) = graph.import_path_for_dir(dir) {
                by_import_path
                    .entry(import_path)
                    .or_default()
                    .push(dir.clone());
            }
        }
        let mut own_modules = Vec::new();
        for (path, _) in inputs.manifest_snapshot.entries() {
            let dir = match path.strip_suffix("go.mod") {
                Some("") => Some(""),
                Some(rest) => rest.strip_suffix('/'),
                None => None,
            };
            if let Some(Ok(module)) = dir.map(|d| graph.import_path_for_dir(d)) {
                own_modules.push(module);
            }
        }
        Self {
            by_import_path,
            files_by_dir,
            own_modules,
            unproven_replaces: graph.unproven_replace_paths().clone(),
        }
    }

    /// Whether `import_path` is `prefix` or below it.
    fn under(import_path: &str, prefix: &str) -> bool {
        import_path == prefix
            || import_path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    }
}

impl ImportTargets<'_> {
    pub(super) fn go_attribution(
        &mut self,
        file: &str,
        parsed: &ParsedFile,
        call: &CallSite,
    ) -> Attribution {
        let root = match call.qualifier.as_deref() {
            Some(qualifier) => match root_identifier(qualifier) {
                Some(root) => root,
                None => return Attribution::Unknown,
            },
            None => call.callee_name.as_str(),
        };
        if go_scope_binds(parsed, call, root) {
            return Attribution::Unknown;
        }
        if call.qualifier.is_none() {
            if is_same_file_function(parsed, call) {
                return Attribution::SameFile;
            }
            return self.go_sibling(file, &call.callee_name);
        }
        let imports = self.imports_of(file, parsed);
        let Some(import_path) = imports.map.get(root) else {
            return Attribution::Unknown;
        };
        let packages = self.go_packages();
        let Some(dirs) = packages.by_import_path.get(import_path.as_str()) else {
            let (own, unproven) = (
                packages
                    .own_modules
                    .iter()
                    .any(|m| GoPackages::under(import_path, m)),
                packages
                    .unproven_replaces
                    .iter()
                    .any(|m| GoPackages::under(import_path, m)),
            );
            return if unproven {
                unresolvable(import_path, Why::ReplaceUnproven)
            } else if own {
                unresolvable(import_path, Why::NoFile)
            } else {
                Attribution::External
            };
        };
        if dirs.len() != 1 {
            return unresolvable(import_path, Why::TooMany);
        }
        let files = packages.files_by_dir[&dirs[0]].clone();
        let qualifier = call.qualifier.as_deref().unwrap_or_default();
        if qualifier != root {
            // `util.Obj.Run`: a method or field of a package-level value. The
            // top-level function scan says nothing about it, so no declaring
            // file is named and the first selector after the package is the
            // export the call reaches.
            let export = qualifier[root.len()..]
                .trim_start_matches('.')
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .filter(|name| !name.is_empty())
                .map(str::to_string);
            return Attribution::InRepo(InRepo {
                files,
                spec: import_path.clone(),
                member: export,
                declaring: Vec::new(),
                basis: Basis::Candidates,
            });
        }
        let declaring: Vec<String> = files
            .iter()
            .filter(|f| self.go_declared_funcs(f).contains(&call.callee_name))
            .cloned()
            .collect();
        if declaring.is_empty() && files.iter().any(|f| self.loaded.contains(f)) {
            // A `var Run = func...`, a type conversion `pkg.T(x)`, a generated
            // declaration: no scanned declarer, and a loaded package file may
            // be the one that holds it, so no boundary is claimed.
            return Attribution::Unknown;
        }
        let basis = if declaring.len() > 1 || (declaring.is_empty() && files.len() > 1) {
            // Build-tag variants, or a callee no scan placed in one file.
            Basis::Candidates
        } else {
            Basis::Bound
        };
        Attribution::InRepo(InRepo {
            files,
            spec: import_path.clone(),
            member: Some(call.callee_name.clone()),
            declaring,
            basis,
        })
    }

    fn go_packages(&mut self) -> &GoPackages {
        let (inputs, listing) = (self.inputs, &self.listing);
        self.go_packages
            .get_or_insert_with(|| GoPackages::build(inputs, listing))
    }

    /// Same-directory non-test files other than `file` that declare
    /// `func <name>(`; read as text, never parsed.
    fn go_sibling(&mut self, file: &str, name: &str) -> Attribution {
        let dir = file.rsplit_once('/').map_or("", |(dir, _)| dir).to_string();
        let siblings: Vec<String> = self
            .go_packages()
            .files_by_dir
            .get(&dir)
            .map(|files| {
                files
                    .iter()
                    .filter(|f| f.as_str() != file)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let declaring: Vec<String> = siblings
            .into_iter()
            .filter(|sibling| self.go_declared_funcs(sibling).contains(name))
            .collect();
        match declaring.len() {
            0 => Attribution::Unknown,
            n => Attribution::InRepo(InRepo {
                files: declaring.clone(),
                spec: String::new(),
                member: Some(name.to_string()),
                declaring,
                // Several files declaring one name are build-tag variants.
                basis: if n == 1 {
                    Basis::Bound
                } else {
                    Basis::Candidates
                },
            }),
        }
    }

    fn go_declared_funcs(&mut self, path: &str) -> Rc<BTreeSet<String>> {
        if let Some(found) = self.go_decls.get(path) {
            return found.clone();
        }
        let text = std::fs::read_to_string(self.inputs.repo_root.join(path)).unwrap_or_default();
        let names = Rc::new(declared_funcs(&text));
        self.go_decls.insert(path.to_string(), names.clone());
        names
    }
}
