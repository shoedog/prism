//! Scope-local proof of imported roots and the lookup slots they require.

mod census;
mod class_slots;
mod lookup;
mod slots;
mod writes;

use super::{
    py_tables,
    python::{
        source::chain,
        walk::{ScopeId, Walk},
    },
    Family, UnprovenCause,
};
use crate::targets::dependency_identity::{module_root, python_dotted_path};
use census::{Binding, Census, Import, Kind};
use lookup::Alternative;
use prism::ast::ParsedFile;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

pub(super) struct Resolution {
    pub family: Family,
    pub identity: Result<String, UnprovenCause>,
    #[cfg(test)]
    pub route: Option<super::evidence::Route>,
}

pub(super) struct Resolver<'a, 'tree> {
    parsed: &'tree ParsedFile,
    walk: &'a Walk<'tree>,
    repo: &'a Path,
    roots: Vec<String>,
    census: Census<'tree>,
    writes: Vec<writes::Write>,
}

impl<'a, 'tree> Resolver<'a, 'tree> {
    pub(super) fn new(
        parsed: &'tree ParsedFile,
        walk: &'a Walk<'tree>,
        repo: &'a Path,
        file: &str,
    ) -> Self {
        let mut roots = vec![String::new()];
        if repo.join("src").is_dir() {
            roots.push("src".to_owned());
        }
        let mut path = PathBuf::from(file);
        while path.pop() && !path.as_os_str().is_empty() {
            let directory = path.to_string_lossy();
            if !repo.join(&path).join("__init__.py").is_file()
                && !roots.iter().any(|root| root == &directory)
            {
                roots.push(directory.into_owned());
            }
        }
        let mut resolver = Self {
            parsed,
            walk,
            repo,
            roots,
            census: Census::new(parsed, walk),
            writes: Vec::new(),
        };
        resolver.index_writes();
        resolver
    }

    pub(super) fn resolve_call(&self, callee: Node<'tree>, scope: ScopeId) -> Option<Resolution> {
        let path = chain(self.parsed, callee, self.walk.class(scope))?;
        let (root, rest) = path.split_first()?;
        let spelled = if root == "open" && rest.is_empty() {
            Some(Family::Filesystem)
        } else {
            py_tables::family(&path.join("."))
        };
        let alternatives = self.lookup(scope, root, callee);

        // A sole, explicitly remapped library name decides eligibility even
        // before its import is available. Multiple binders remain ambiguous.
        if let [alt] = alternatives.as_slice() {
            if let [binding] = alt.bindings.as_slice() {
                if let Kind::Import(import) = &binding.kind {
                    if py_tables::library(module_root(&import.path))
                        && py_tables::family(&import_path(import, rest)).is_none()
                    {
                        return None;
                    }
                }
            }
        }
        let family = alternatives
            .iter()
            .flat_map(|alt| &alt.bindings)
            .filter_map(|binding| match &binding.kind {
                Kind::Import(import) => py_tables::family(&import_path(import, rest)),
                _ => None,
            })
            .chain(spelled)
            .min()?;
        let refuse = |cause| {
            Some(Resolution {
                family,
                identity: Err(cause),
                #[cfg(test)]
                route: None,
            })
        };
        if self.census.wildcard {
            return refuse(UnprovenCause::StarImport);
        }

        let mut proven = None;
        let mut refusal = None;
        let is_join = alternatives.len() > 1;
        for alt in &alternatives {
            match self.prove_alternative(alt, root, rest, callee, scope) {
                Ok(path) if proven.as_ref().is_none_or(|prior| prior == &path) => {
                    proven = Some(path)
                }
                Ok(_) => return refuse(UnprovenCause::ShadowedRoot),
                Err(cause) => {
                    refusal =
                        Some(refusal.map_or(cause, |earlier: UnprovenCause| earlier.min(cause)))
                }
            }
        }
        if let Some(cause) = refusal {
            return refuse(if is_join {
                UnprovenCause::ShadowedRoot
            } else {
                cause
            });
        }
        let canonical = proven?;
        Some(Resolution {
            family: py_tables::family(&canonical)?,
            identity: Ok(canonical),
            #[cfg(test)]
            route: Some(super::evidence::Route::Root),
        })
    }

    fn prove_alternative(
        &self,
        alt: &Alternative<'_, 'tree>,
        root: &str,
        rest: &[String],
        at: Node<'tree>,
        scope: ScopeId,
    ) -> Result<String, UnprovenCause> {
        use UnprovenCause::*;
        if self.name_rebound(alt.owner, root, at, scope) || alt.implicit_cell {
            return Err(ShadowedRoot);
        }
        let Some(first) = alt.bindings.first() else {
            if root == "open" && rest.is_empty() {
                if self.name_rebound(0, "__builtins__", at, scope)
                    || !self.census.spaces[0]
                        .bindings
                        .get("__builtins__")
                        .is_none_or(Vec::is_empty)
                {
                    return Err(ShadowedRoot);
                }
                return if self.builtin_rebound("open", at, scope) {
                    Err(MemberRebound)
                } else {
                    Ok("builtins.open".to_owned())
                };
            }
            return Err(NotImported);
        };
        if !self.coalesces(alt) {
            return Err(ShadowedRoot);
        }
        let Kind::Import(import) = &first.kind else {
            return Err(ShadowedRoot);
        };
        let path = import_path(import, rest);
        if import.relative {
            return Err(RepositoryLocalModule);
        }
        if py_tables::family(&path).is_none() {
            return Err(ShadowedRoot);
        }
        if self.repository_local(&path) {
            return Err(RepositoryLocalModule);
        }
        let Some(binding) = alt
            .bindings
            .iter()
            .copied()
            .find(|binding| self.available(binding, at, scope) && !self.type_only(binding))
        else {
            return Err(BindingUnavailable);
        };
        if self.member_rebound(import, binding, at, scope, rest) {
            return Err(MemberRebound);
        }
        Ok(path)
    }

    fn available(&self, binding: &Binding<'tree>, at: Node<'tree>, scope: ScopeId) -> bool {
        let mut current = scope;
        loop {
            if binding.scope == current {
                return binding.position <= at.start_byte();
            }
            if current == 0 || self.walk.scopes[current].kind.is_function() {
                return true;
            }
            current = self.walk.scopes[current].parent;
        }
    }
}

fn import_path(import: &Import, rest: &[String]) -> String {
    python_dotted_path(&import.path, rest.iter().map(String::as_str))
}
