//! Python attribution: relative imports by dot-walk from the caller's
//! directory; absolute `a.b` under the nearest ancestor directory that holds
//! `a/b.py` or `a/b/__init__.py` (repo root included).
use super::{bounded, is_same_file_function, root_identifier, Attribution, ImportTargets};
use crate::ast::ParsedFile;
use crate::call_graph::{python_identifier, CallSite, ImportBindingKind};

impl ImportTargets<'_> {
    pub(super) fn py_attribution(
        &mut self,
        file: &str,
        parsed: &ParsedFile,
        call: &CallSite,
    ) -> Attribution {
        let imports = self.imports_of(file, parsed);
        // Python's last binding of a name wins.
        let binding_for = |local: &str| {
            imports
                .bindings
                .iter()
                .rev()
                .find(|b| b.local == local && b.kind != ImportBindingKind::WildcardImport)
        };
        let Some(qualifier) = call.qualifier.as_deref() else {
            return match binding_for(&call.callee_name) {
                Some(b) if b.kind == ImportBindingKind::MemberImport => {
                    self.py_module(file, &b.module_path)
                }
                Some(_) => Attribution::Unknown,
                None if is_same_file_function(parsed, call) => Attribution::Local,
                None => Attribution::Unknown,
            };
        };
        let Some(root) = root_identifier(qualifier) else {
            return Attribution::Unknown;
        };
        let Some(binding) = binding_for(root) else {
            return Attribution::Unknown;
        };
        match binding.kind {
            ImportBindingKind::AliasedModuleImport => self.py_module(file, &binding.module_path),
            ImportBindingKind::ModuleImport => {
                let module = &binding.module_path;
                if qualifier == module || module.starts_with(&format!("{qualifier}.")) {
                    self.py_module(file, qualifier)
                } else if qualifier.starts_with(&format!("{module}.")) {
                    self.py_module(file, module)
                } else {
                    Attribution::Unknown
                }
            }
            ImportBindingKind::MemberImport => {
                // `from pkg import m; m.f()`: a submodule when one exists.
                if let Some(member) = binding.member.as_deref() {
                    let sub = if binding.module_path.chars().all(|c| c == '.') {
                        format!("{}{member}", binding.module_path)
                    } else {
                        format!("{}.{member}", binding.module_path)
                    };
                    if !self.py_module_files(file, &sub).is_empty() {
                        return self.py_module(file, &sub);
                    }
                }
                self.py_module(file, &binding.module_path)
            }
            ImportBindingKind::WildcardImport => Attribution::Unknown,
        }
    }

    fn py_module(&self, file: &str, module: &str) -> Attribution {
        let files = self.py_module_files(file, module);
        if files.is_empty() && !module.starts_with('.') {
            return Attribution::External;
        }
        bounded(files, module)
    }

    /// Files a dotted module names from `file`: empty when it names none.
    fn py_module_files(&self, file: &str, module: &str) -> Vec<String> {
        let caller_dir = file.rsplit_once('/').map_or("", |(dir, _)| dir);
        if module.starts_with('.') {
            let suffix = module.trim_start_matches('.');
            let dots = module.len() - suffix.len();
            if !suffix.is_empty() && !suffix.split('.').all(python_identifier) {
                return Vec::new();
            }
            let mut parts: Vec<&str> = caller_dir.split('/').filter(|p| !p.is_empty()).collect();
            for _ in 1..dots {
                if parts.pop().is_none() {
                    return Vec::new();
                }
            }
            return self.py_hits(&parts.join("/"), &suffix.replace('.', "/"));
        }
        if !module.split('.').all(python_identifier) {
            return Vec::new();
        }
        let relative = module.replace('.', "/");
        let mut dir = caller_dir.to_string();
        loop {
            let hits = self.py_hits(&dir, &relative);
            if !hits.is_empty() || dir.is_empty() {
                return hits;
            }
            dir = dir
                .rsplit_once('/')
                .map_or("", |(parent, _)| parent)
                .to_string();
        }
    }

    /// `base/rel.py` and `base/rel/__init__.py` (or `base/__init__.py` for an
    /// empty `rel`) that exist in the listing.
    fn py_hits(&self, base: &str, rel: &str) -> Vec<String> {
        let under = |tail: String| {
            if base.is_empty() {
                tail
            } else {
                format!("{base}/{tail}")
            }
        };
        let candidates = if rel.is_empty() {
            vec![under("__init__.py".into())]
        } else {
            vec![
                under(format!("{rel}.py")),
                under(format!("{rel}/__init__.py")),
            ]
        };
        candidates
            .into_iter()
            .filter(|c| self.listing.contains(c))
            .collect()
    }
}
