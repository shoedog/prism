//! Python attribution.
//!
//! *Names.* The call's root name must not be bound by an enclosing function,
//! lambda or comprehension, nor rebound at module level (`shadow.rs`). The
//! last import statement binding the root decides, except that unaliased dotted
//! imports (`import pkg.a`, `import pkg.b`) all bind one package object, so the
//! qualifier chooses among them by its longest module prefix.
//!
//! *Modules.* A relative import walks up from the caller's directory. An
//! absolute import searches from the *import root*: the nearest ancestor
//! directory of the caller that is not a package (no `__init__.py`), then each
//! parent up to the repo root. Per directory a package (`m/__init__.py`) wins
//! over a module (`m.py`), as in CPython's `FileFinder`. A standard-library
//! top-level name (a static 3.12 list) is `External` unless the import root
//! itself holds a same-named module: the script directory, or the directory
//! above a top-level package, precedes the standard library on `sys.path`; a
//! sibling or ancestor inside a package does not.
//!
//! *Attributes.* `from pkg import m` and `import pkg; pkg.sub.k()` name either
//! a package attribute (set by `pkg/__init__.py`, which is not read) or a
//! submodule, and the chain continues through every further submodule
//! (`pkg.a.b.f` reaches `pkg/a/b.py`). When an initializer and a submodule both
//! exist they are returned as candidates, the initializers first (Python tries
//! the attribute first) and the deepest submodule last, with no `member`; a
//! lone submodule of a package without an initializer is bound. A name that is
//! only an attribute of the file reached leaves `member` empty.
use super::shadow::{py_module_rebinds, py_scope_binds};
use super::stdlib_names::{is_builtin_or_frozen_module, is_stdlib_module};
use super::{
    bound, is_same_file_function, root_identifier, unresolvable, Attribution, Basis, ImportTargets,
    InRepo, Why,
};
use crate::ast::ParsedFile;
use crate::call_graph::{python_identifier, CallSite, ImportBinding, ImportBindingKind};

/// What a dotted module name reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Lookup {
    /// The file Python imports for the module (a single winner).
    File(String),
    /// A standard-library or third-party module.
    External,
    /// Looks in-repo (relative, or an in-repo top-level name) but names no file.
    Missing,
}

impl ImportTargets<'_> {
    pub(super) fn py_attribution(
        &mut self,
        file: &str,
        parsed: &ParsedFile,
        call: &CallSite,
    ) -> Attribution {
        let imports = self.imports_of(file, parsed);
        let (root, qualifier) = match call.qualifier.as_deref() {
            Some(q) => match root_identifier(q) {
                Some(root) => (root, Some(q)),
                None => return Attribution::Unknown,
            },
            None => (call.callee_name.as_str(), None),
        };
        if py_scope_binds(parsed, call, root) {
            return Attribution::Unknown;
        }
        let group: Vec<&ImportBinding> = imports
            .bindings
            .iter()
            .filter(|b| b.local == root && b.kind != ImportBindingKind::WildcardImport)
            .collect();
        let Some(last) = group.last() else {
            return if is_same_file_function(parsed, call) {
                Attribution::SameFile
            } else {
                Attribution::Unknown
            };
        };
        if py_module_rebinds(parsed, root) {
            return Attribution::Unknown;
        }
        let Some(qualifier) = qualifier else {
            // `from m import f; f()`: the attribute `f` of `m`.
            return match last.kind {
                ImportBindingKind::MemberImport => {
                    let lookup = self.py_lookup(file, &last.module_path);
                    self.py_attribution_of(lookup, &last.module_path, last.member.clone())
                }
                _ => Attribution::Unknown,
            };
        };
        let parts: Vec<&str> = qualifier.split('.').collect();
        if !parts.iter().all(|p| python_identifier(p)) {
            return Attribution::Unknown;
        }
        let member = Some(call.callee_name.clone());
        match last.kind {
            ImportBindingKind::AliasedModuleImport => {
                self.py_chain(file, &last.module_path, &parts[1..], member)
            }
            ImportBindingKind::ModuleImport => {
                // Unaliased imports after the last rebinding bind the same
                // package object: pick the one sharing the longest prefix.
                let start = group
                    .iter()
                    .rposition(|b| b.kind != ImportBindingKind::ModuleImport)
                    .map_or(0, |i| i + 1);
                let shared = group[start..]
                    .iter()
                    .map(|b| {
                        let common = b
                            .module_path
                            .split('.')
                            .zip(&parts)
                            .take_while(|(a, b)| a == *b)
                            .count();
                        common
                    })
                    .max();
                match shared {
                    Some(common) if common > 0 => {
                        let module = parts[..common].join(".");
                        self.py_chain(file, &module, &parts[common..], member)
                    }
                    _ => Attribution::Unknown,
                }
            }
            ImportBindingKind::MemberImport => {
                // `from pkg import m; m.f()`: `m` is an attribute of `pkg` or a
                // submodule; the chain continues below it.
                let mut rest: Vec<&str> = last.member.as_deref().into_iter().collect();
                rest.extend(&parts[1..]);
                self.py_chain(file, &last.module_path, &rest, member)
            }
            ImportBindingKind::WildcardImport => Attribution::Unknown,
        }
    }

    /// The module `module`, then each component of `rest` while it names an
    /// existing submodule (`pkg.a.b.f` reaches `pkg/a/b.py`). A component that
    /// is not a submodule is an attribute of the file reached so far, so the
    /// result is that file, with no `member`: the callee is a property of an
    /// attribute, not an export (as for a JS `obj.f()` on an imported `obj`).
    /// Every package initializer passed on the way is returned beside the final
    /// file as a candidate, because the initializer may set the attribute
    /// itself (Python tries the attribute before the submodule); a candidate
    /// list has no `member` (the callee name belongs to the last file only).
    fn py_chain(
        &self,
        file: &str,
        module: &str,
        rest: &[&str],
        member: Option<String>,
    ) -> Attribution {
        let mut current = module.to_string();
        let mut owner = self.py_lookup(file, module);
        let mut initializers: Vec<String> = Vec::new();
        let mut walked = 0;
        for next in rest {
            let submodule = if current.chars().all(|c| c == '.') {
                format!("{current}{next}")
            } else {
                format!("{current}.{next}")
            };
            match (&owner, self.py_lookup(file, &submodule)) {
                (Lookup::File(init), Lookup::File(sub)) if *init != sub => {
                    initializers.push(init.clone());
                    owner = Lookup::File(sub);
                }
                (Lookup::Missing, Lookup::File(sub)) => owner = Lookup::File(sub),
                _ => break,
            }
            current = submodule;
            walked += 1;
        }
        let member = if walked == rest.len() { member } else { None };
        let reached = self.py_attribution_of(owner, &current, member);
        match reached {
            Attribution::InRepo(mut hit) if !initializers.is_empty() => {
                initializers.append(&mut hit.files);
                Attribution::InRepo(InRepo {
                    files: initializers,
                    spec: module.to_string(),
                    member: None,
                    declaring: Vec::new(),
                    basis: Basis::Candidates,
                })
            }
            other => other,
        }
    }

    fn py_attribution_of(
        &self,
        lookup: Lookup,
        module: &str,
        member: Option<String>,
    ) -> Attribution {
        match lookup {
            Lookup::File(found) => bound(vec![found], module, member),
            Lookup::External => Attribution::External,
            Lookup::Missing => unresolvable(module, Why::NoFile),
        }
    }

    fn py_lookup(&self, file: &str, module: &str) -> Lookup {
        let caller_dir = file.rsplit_once('/').map_or("", |(dir, _)| dir);
        if module.starts_with('.') {
            let suffix = module.trim_start_matches('.');
            let dots = module.len() - suffix.len();
            if !suffix.is_empty() && !suffix.split('.').all(python_identifier) {
                return Lookup::Missing;
            }
            let mut parts: Vec<&str> = caller_dir.split('/').filter(|p| !p.is_empty()).collect();
            for _ in 1..dots {
                if parts.pop().is_none() {
                    return Lookup::Missing;
                }
            }
            return self
                .py_winner(&parts.join("/"), &suffix.replace('.', "/"))
                .map_or(Lookup::Missing, Lookup::File);
        }
        if !module.split('.').all(python_identifier) {
            return Lookup::External;
        }
        let top = module.split('.').next().unwrap_or(module);
        let relative = module.replace('.', "/");
        let root = self.py_import_root(caller_dir);
        // Built-in and frozen modules precede every path entry. Another
        // standard-library name yields only to a module or package at the
        // import root; a namespace-only directory (no `__init__.py`) does not.
        if is_builtin_or_frozen_module(top)
            || (is_stdlib_module(top) && self.py_winner(&root, top).is_none())
        {
            return Lookup::External;
        }
        let mut dir = root;
        let mut in_repo_name = false;
        loop {
            if let Some(found) = self.py_winner(&dir, &relative) {
                return Lookup::File(found);
            }
            in_repo_name |= self.py_dir_holds(&dir, top);
            if dir.is_empty() {
                break;
            }
            dir = dir
                .rsplit_once('/')
                .map_or("", |(parent, _)| parent)
                .to_string();
        }
        if in_repo_name {
            Lookup::Missing
        } else {
            Lookup::External
        }
    }

    /// The nearest ancestor directory of `caller_dir` (itself included) that is
    /// not a package: the directory `sys.path` holds for this file.
    fn py_import_root(&self, caller_dir: &str) -> String {
        let mut dir = caller_dir;
        while !dir.is_empty() && self.listing.contains(&join(dir, "__init__.py")) {
            dir = dir.rsplit_once('/').map_or("", |(parent, _)| parent);
        }
        dir.to_string()
    }

    /// Whether `base` holds module `name` (`name.py`, or a `name/` directory
    /// with a listed Python file below it).
    fn py_dir_holds(&self, base: &str, name: &str) -> bool {
        let module = join(base, &format!("{name}.py"));
        let prefix = join(base, &format!("{name}/"));
        self.listing.contains(&module)
            || self
                .listing
                .range(prefix.clone()..)
                .take_while(|p| p.starts_with(&prefix))
                .any(|p| p.ends_with(".py"))
    }

    /// `base/rel/__init__.py` (or `base/__init__.py` for an empty `rel`), else
    /// `base/rel.py`, when listed: the package before the module.
    fn py_winner(&self, base: &str, rel: &str) -> Option<String> {
        let candidates = if rel.is_empty() {
            vec![join(base, "__init__.py")]
        } else {
            vec![
                join(base, &format!("{rel}/__init__.py")),
                join(base, &format!("{rel}.py")),
            ]
        };
        candidates.into_iter().find(|c| self.listing.contains(c))
    }
}

fn join(base: &str, tail: &str) -> String {
    if base.is_empty() {
        tail.to_string()
    } else {
        format!("{base}/{tail}")
    }
}
