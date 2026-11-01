//! Go attribution: a qualified call goes to its import path's package
//! directory (every non-test file), an unqualified call to the same-package
//! sibling that declares it.
use super::{is_same_file_function, root_identifier, Attribution, ImportTargets};
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
        Self {
            by_import_path,
            files_by_dir,
        }
    }
}

impl ImportTargets<'_> {
    pub(super) fn go_attribution(
        &mut self,
        file: &str,
        parsed: &ParsedFile,
        call: &CallSite,
    ) -> Attribution {
        let Some(qualifier) = call.qualifier.as_deref() else {
            if is_same_file_function(parsed, call) {
                return Attribution::Local;
            }
            return self.go_sibling(file, &call.callee_name);
        };
        let Some(root) = root_identifier(qualifier) else {
            return Attribution::Unknown;
        };
        let imports = self.imports_of(file, parsed);
        let Some(import_path) = imports.map.get(root) else {
            return Attribution::Unknown;
        };
        let packages = self.go_packages();
        match packages.by_import_path.get(import_path.as_str()) {
            None => Attribution::External,
            Some(dirs) if dirs.len() == 1 => {
                Attribution::InRepo(packages.files_by_dir[&dirs[0]].clone())
            }
            Some(_) => Attribution::Unresolvable {
                spec: import_path.clone(),
            },
        }
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
        if declaring.is_empty() {
            Attribution::Unknown
        } else {
            Attribution::InRepo(declaring)
        }
    }

    fn go_declared_funcs(&mut self, path: &str) -> Rc<BTreeSet<String>> {
        if let Some(found) = self.go_decls.get(path) {
            return found.clone();
        }
        let text = std::fs::read_to_string(self.inputs.repo_root.join(path)).unwrap_or_default();
        let mut names = BTreeSet::new();
        for line in text.lines() {
            // Top-level function declarations start at column 0 (`func (r R) m(` has no name here).
            let Some(rest) = line.strip_prefix("func ") else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && rest[name.len()..].starts_with(['(', '[']) {
                names.insert(name);
            }
        }
        let names = Rc::new(names);
        self.go_decls.insert(path.to_string(), names.clone());
        names
    }
}
