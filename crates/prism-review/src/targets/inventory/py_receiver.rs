//! Scope-local proof of imported callables and listed receiver methods.

mod annotation;
mod census;
mod class_slots;
mod constructor;
mod context;
mod field;
mod helper;
mod lookup;
mod named;
mod slots;
mod writes;

use super::{
    py_tables,
    python::{
        source::{chain, Source},
        type_syntax::has_ambiguous_type_syntax,
        walk::{ScopeId, Walk},
    },
    Family, Grade, UnprovenCause,
};
use crate::targets::dependency_identity::{module_root, python_dotted_path};
use census::{Binding, Census, Import, Kind};
use context::Protocol;
use lookup::Alternative;
use prism::ast::ParsedFile;
use slots::MemberRead;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

pub(super) struct Resolution {
    pub family: Family,
    pub identity: Result<String, UnprovenCause>,
    pub grade: Grade,
    #[cfg(any(test, feature = "inventory-evidence"))]
    pub route: Option<super::evidence::Route>,
    #[cfg(any(test, feature = "inventory-evidence"))]
    pub receiver: Option<super::evidence::ReceiverProof>,
    #[cfg(any(test, feature = "inventory-evidence"))]
    pub declaration: Option<super::evidence::ClassSource>,
    #[cfg(any(test, feature = "inventory-evidence"))]
    pub field: Option<Box<super::evidence::FieldProof>>,
}

impl Resolution {
    fn refused(family: Family, grade: Grade, cause: UnprovenCause) -> Self {
        Self {
            family,
            grade,
            identity: Err(cause),
            #[cfg(any(test, feature = "inventory-evidence"))]
            route: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            receiver: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            declaration: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            field: None,
        }
    }

    fn prefer(earlier: Option<Self>, later: impl FnOnce() -> Option<Self>) -> Option<Self> {
        if earlier.as_ref().is_some_and(|proof| proof.identity.is_ok()) {
            return earlier;
        }
        match (earlier, later()) {
            (Some(earlier), Some(later))
                if matches!(
                    (&earlier.identity, &later.identity),
                    (Err(earlier), Err(later)) if earlier <= later
                ) =>
            {
                Some(earlier)
            }
            (earlier, later) => later.or(earlier),
        }
    }
}

#[derive(Clone, Copy)]
enum PathUse<'a> {
    Root,
    Constructor {
        method: Option<&'a str>,
        protocol: Option<Protocol>,
    },
    Declaration {
        method: Option<&'a str>,
        static_only: bool,
    },
    Typing {
        path: &'static str,
        static_only: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PathMatch {
    Root(Family),
    Class(Option<Family>),
    Typing,
}

impl PathUse<'_> {
    fn matches(self, path: &str) -> Option<PathMatch> {
        let class = |method: Option<&str>| {
            if !py_tables::constructor(path) {
                return None;
            }
            let family = match method {
                Some(method) => Some(py_tables::method_family(path, method)?),
                None => None,
            };
            Some(PathMatch::Class(family))
        };
        match self {
            Self::Root => py_tables::family(path).map(PathMatch::Root),
            Self::Constructor { method, protocol } => {
                if protocol.is_some_and(|protocol| !protocol.accepts(path)) {
                    return None;
                }
                class(method)
            }
            Self::Declaration { method, .. } => class(method),
            Self::Typing { path: expected, .. } => (path == expected).then_some(PathMatch::Typing),
        }
    }

    fn static_only(self) -> bool {
        matches!(
            self,
            Self::Declaration {
                static_only: true,
                ..
            } | Self::Typing {
                static_only: true,
                ..
            }
        )
    }
}

pub(super) struct Resolver<'a, 'tree> {
    parsed: &'tree ParsedFile,
    walk: &'a Walk<'tree>,
    repo: &'a Path,
    roots: Vec<String>,
    census: Census<'tree>,
    writes: Vec<writes::Write>,
    field_rows: FieldRows,
    #[cfg(any(test, feature = "inventory-evidence"))]
    source: &'a Source<'a>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldRows {
    Disabled,
    Enabled,
    Refused,
}

impl<'a, 'tree> Resolver<'a, 'tree> {
    pub(super) fn new(
        parsed: &'tree ParsedFile,
        walk: &'a Walk<'tree>,
        repo: &'a Path,
        file: &str,
        field_rows: bool,
        source: &'a Source<'a>,
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
        let field_rows = if !field_rows {
            FieldRows::Disabled
        } else if has_ambiguous_type_syntax(source.parsed()) {
            FieldRows::Refused
        } else {
            FieldRows::Enabled
        };
        let mut resolver = Self {
            parsed,
            walk,
            repo,
            roots,
            census: Census::new(parsed, walk, field_rows != FieldRows::Disabled),
            writes: Vec::new(),
            field_rows,
            #[cfg(any(test, feature = "inventory-evidence"))]
            source,
        };
        resolver.index_writes();
        resolver
    }

    pub(super) fn resolve_call(&self, callee: Node<'tree>, scope: ScopeId) -> Option<Resolution> {
        let root = self
            .resolve_path(callee, scope, PathUse::Root)
            .and_then(|(kind, identity)| {
                let PathMatch::Root(family) = kind else {
                    return None;
                };
                Some(Resolution {
                    family,
                    grade: Grade::Exact,
                    #[cfg(any(test, feature = "inventory-evidence"))]
                    route: identity.is_ok().then_some(super::evidence::Route::Root),
                    #[cfg(any(test, feature = "inventory-evidence"))]
                    receiver: None,
                    #[cfg(any(test, feature = "inventory-evidence"))]
                    declaration: None,
                    #[cfg(any(test, feature = "inventory-evidence"))]
                    field: None,
                    identity,
                })
            });
        let root = Resolution::prefer(root, || self.inline_receiver(callee, scope));
        let named = Resolution::prefer(root, || self.named_receiver(callee, scope));
        let field = Resolution::prefer(named, || self.field_receiver(callee, scope));
        Resolution::prefer(field, || self.declared_receiver(callee, scope))
    }

    fn resolve_path(
        &self,
        callee: Node<'tree>,
        scope: ScopeId,
        path_use: PathUse<'_>,
    ) -> Option<(PathMatch, Result<String, UnprovenCause>)> {
        let path = chain(self.parsed, callee, self.walk.class(scope))?;
        self.resolve_parts(&path, callee, scope, path_use)
    }

    fn resolve_parts(
        &self,
        path: &[String],
        callee: Node<'tree>,
        scope: ScopeId,
        path_use: PathUse<'_>,
    ) -> Option<(PathMatch, Result<String, UnprovenCause>)> {
        let (root, rest) = path.split_first()?;
        let spelled = if matches!(path_use, PathUse::Root) && root == "open" && rest.is_empty() {
            Some(PathMatch::Root(Family::Filesystem))
        } else {
            path_use.matches(&path.join("."))
        };
        let alternatives = self.lookup(scope, root, callee);

        // A sole, explicitly remapped library name decides eligibility even
        // before its import is available. Multiple binders remain ambiguous.
        if let [alt] = alternatives.as_slice() {
            if let [binding] = alt.bindings.as_slice() {
                if let Kind::Import(import) = &binding.kind {
                    if py_tables::library(module_root(&import.path))
                        && path_use.matches(&import_path(import, rest)).is_none()
                    {
                        return None;
                    }
                }
            }
        }
        let tie = alternatives
            .iter()
            .flat_map(|alt| &alt.bindings)
            .filter_map(|binding| match &binding.kind {
                Kind::Import(import) => path_use.matches(&import_path(import, rest)),
                _ => None,
            })
            .chain(spelled)
            .min()?;
        let refuse = |cause| Some((tie, Err(cause)));
        if self.census.wildcard {
            return refuse(UnprovenCause::StarImport);
        }

        let mut proven = None;
        let mut refusal = None;
        let is_join = alternatives.len() > 1;
        for alt in &alternatives {
            match self.prove_alternative(alt, root, rest, callee, scope, path_use) {
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
        Some((path_use.matches(&canonical)?, Ok(canonical)))
    }

    fn prove_alternative(
        &self,
        alt: &Alternative<'_, 'tree>,
        root: &str,
        rest: &[String],
        at: Node<'tree>,
        scope: ScopeId,
        path_use: PathUse<'_>,
    ) -> Result<String, UnprovenCause> {
        use UnprovenCause::*;
        if self.name_rebound(alt.owner, root, at, scope) || alt.implicit_cell {
            return Err(ShadowedRoot);
        }
        let Some(first) = alt.bindings.first() else {
            if matches!(path_use, PathUse::Root) && root == "open" && rest.is_empty() {
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
        if path_use.matches(&path).is_none() {
            return Err(ShadowedRoot);
        }
        if self.repository_local(&path) {
            return Err(RepositoryLocalModule);
        }
        let Some(binding) = alt.bindings.iter().copied().find(|binding| {
            path_use.static_only()
                || (self.available(binding, at, scope) && !self.type_only(binding))
        }) else {
            return Err(BindingUnavailable);
        };
        let member_read = match path_use {
            PathUse::Root => MemberRead::Root,
            _ => MemberRead::Receiver,
        };
        if self.member_rebound(import, binding, at, scope, rest, member_read) {
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
