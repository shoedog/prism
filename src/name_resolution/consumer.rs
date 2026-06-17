//! Inert consumer-facing helpers for the Rust scope graph (Task 4a).
//!
//! Nothing in production calls these yet. PR-3's call narrowing and module-deps
//! consumers will use this layer so both share the same authority and
//! recall-safety predicates.

use crate::call_graph::{CallKind, CallSite};
use crate::name_resolution::engine::{resolve, resolve_path};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::{RustPolicy, NS_MACRO, NS_TYPE, NS_VALUE};
use crate::name_resolution::rust_populator::enclosing_scope;
use crate::name_resolution::types::{
    BindTarget, Binding, Candidate, CfgCtx, Edge, FileId, PolicyQueryCtx, RawPath, ResStatus,
    Resolution, ResolveQuery, ScopeId, SourceLoc, Target,
};

/// A Rust import-like graph artifact that can contribute a module-deps edge.
#[derive(Debug, Clone, Copy)]
pub enum GraphImport<'a> {
    /// A named `use` / `pub use` binding.
    Named(&'a Binding),
    /// A `use a::*` / `pub use a::*` glob edge. The edge to `a` is real even
    /// while member expansion remains deferred/poisoned for call resolution.
    Glob(&'a Edge),
}

/// The module-deps view of resolving one Rust import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedImport {
    /// An in-repo defining file path.
    File(String),
    /// A known external/std/dep import.
    External,
    /// Not safely resolvable to a single in-repo file or external target.
    Unresolved,
}

/// Is `graph` authoritative for `site`'s containing region?
///
/// Authority is keyed on the call site's own modeled scope. A poisoned target
/// reached by lookup is still an authoritative site; the resolution result then
/// falls through without minting a wrong edge.
pub fn authoritative_for(graph: &ScopeGraph, site: &CallSite) -> bool {
    if !graph.complete {
        return false;
    }
    let Some(file) = file_id_for_path(graph, &site.caller.file) else {
        return false;
    };
    enclosing_scope(graph, file, site.start_byte).is_some()
}

/// Resolve a call site to a graph call edge target.
///
/// Returns `Some` only for an authoritative site whose kind-routed lookup
/// resolves to exactly one in-repo callable item. Every other outcome returns
/// `None` so the caller can apply the PR-3 fall-through contract.
///
/// Macro-resolution seam: `MacroInvocation` sites resolve only in the Macro
/// namespace, never the Value namespace, so a future macro call-site increment
/// cannot cross to a same-name `fn` when macro resolution falls through.
pub fn graph_callable_edge(graph: &ScopeGraph, site: &CallSite) -> Option<Target> {
    if !authoritative_for(graph, site) {
        return None;
    }
    let ns = match site.kind {
        CallKind::Call => NS_VALUE,
        CallKind::MacroInvocation => NS_MACRO,
    };
    let file = file_id_for_path(graph, &site.caller.file)?;
    let from = enclosing_scope(graph, file, site.start_byte)?;
    let q = ResolveQuery {
        name: site.callee_name.clone(),
        ns,
        from,
        at: SourceLoc {
            file,
            byte: site.start_byte,
        },
        cfg: CfgCtx::default(),
        ctx: PolicyQueryCtx::default(),
    };
    let policy = RustPolicy::new(graph, 2018);
    let res = resolve(graph, &q, &policy);
    match (res.status, res.candidates.as_slice()) {
        (ResStatus::Resolved, [Candidate { target, .. }]) => match target {
            Target::Item { callable: true, .. } => Some(target.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve a Rust import/re-export/glob into the module-deps edge predicate.
///
/// This is intentionally not callable-gated: type-only imports and re-exports
/// are real module dependencies.
pub fn graph_module_dep_edge(graph: &ScopeGraph, import: GraphImport<'_>) -> ResolvedImport {
    if !graph.complete {
        return ResolvedImport::Unresolved;
    }
    match import {
        GraphImport::Named(binding) => resolve_named_import(graph, binding),
        GraphImport::Glob(edge) => resolve_glob_import(graph, edge),
    }
}

fn resolve_named_import(graph: &ScopeGraph, binding: &Binding) -> ResolvedImport {
    if graph.scope(binding.scope).is_none() {
        return ResolvedImport::Unresolved;
    }
    match &binding.target {
        BindTarget::Resolved(target) => import_from_target(graph, target),
        BindTarget::Pending(path, anchor) => {
            if path.0.is_empty() {
                return ResolvedImport::Unresolved;
            }
            let Some(at) = scope_loc(graph, binding.scope) else {
                return ResolvedImport::Unresolved;
            };
            let policy = RustPolicy::new(graph, 2018);
            let res = resolve_path(
                graph,
                path,
                binding.ns,
                anchor,
                binding.scope,
                NS_TYPE,
                &at,
                &policy,
            );
            import_from_resolution(graph, &res)
        }
    }
}

fn resolve_glob_import(graph: &ScopeGraph, edge: &Edge) -> ResolvedImport {
    if graph.scope(edge.from).is_none() {
        return ResolvedImport::Unresolved;
    }
    match &edge.to {
        BindTarget::Resolved(target) => import_from_target(graph, target),
        BindTarget::Pending(path, anchor) => resolve_glob_path(graph, path, anchor, edge.from),
    }
}

fn resolve_glob_path(
    graph: &ScopeGraph,
    path: &RawPath,
    anchor: &crate::name_resolution::types::Anchor,
    from: ScopeId,
) -> ResolvedImport {
    if path.0.is_empty() {
        return ResolvedImport::Unresolved;
    }
    let Some(at) = scope_loc(graph, from) else {
        return ResolvedImport::Unresolved;
    };
    let policy = RustPolicy::new(graph, 2018);
    let res = resolve_path(graph, path, NS_TYPE, anchor, from, NS_TYPE, &at, &policy);
    import_from_resolution(graph, &res)
}

fn import_from_resolution(graph: &ScopeGraph, res: &Resolution) -> ResolvedImport {
    match res.status {
        ResStatus::Resolved => match res.candidates.as_slice() {
            [Candidate { target, .. }] => import_from_target(graph, target),
            _ => ResolvedImport::Unresolved,
        },
        ResStatus::ResolvedSet => import_from_same_file_set(graph, &res.candidates),
        ResStatus::Ambiguous | ResStatus::Poisoned | ResStatus::Unresolved => {
            ResolvedImport::Unresolved
        }
    }
}

fn import_from_same_file_set(graph: &ScopeGraph, candidates: &[Candidate]) -> ResolvedImport {
    let mut files = std::collections::BTreeSet::new();
    for c in candidates {
        match target_file(graph, &c.target) {
            Some(path) => {
                files.insert(path);
            }
            None => return ResolvedImport::Unresolved,
        }
    }
    if files.len() == 1 {
        ResolvedImport::File(files.into_iter().next().expect("one file"))
    } else {
        ResolvedImport::Unresolved
    }
}

fn import_from_target(graph: &ScopeGraph, target: &Target) -> ResolvedImport {
    match target {
        Target::External(_) => ResolvedImport::External,
        Target::Scope(_) | Target::Item { .. } => target_file(graph, target)
            .map(ResolvedImport::File)
            .unwrap_or(ResolvedImport::Unresolved),
        Target::Local(_) => ResolvedImport::Unresolved,
    }
}

fn target_file(graph: &ScopeGraph, target: &Target) -> Option<String> {
    match target {
        Target::Scope(scope) => file_for_scope(graph, *scope),
        Target::Item { .. } => defining_binding_file(graph, target),
        Target::Local(_) | Target::External(_) => None,
    }
}

fn defining_binding_file(graph: &ScopeGraph, target: &Target) -> Option<String> {
    graph
        .bindings
        .iter()
        .filter(|b| matches!(&b.target, BindTarget::Resolved(t) if t == target))
        .filter_map(|b| file_for_scope(graph, b.scope))
        .min()
}

fn file_for_scope(graph: &ScopeGraph, scope: ScopeId) -> Option<String> {
    let file = graph.scope(scope)?.extents.first()?.file;
    path_for_file(graph, file)
}

fn path_for_file(graph: &ScopeGraph, file: FileId) -> Option<String> {
    graph
        .file_paths
        .iter()
        .find_map(|(path, id)| (*id == file).then(|| path.clone()))
}

fn file_id_for_path(graph: &ScopeGraph, path: &str) -> Option<FileId> {
    graph.file_paths.get(path).copied()
}

fn scope_loc(graph: &ScopeGraph, scope: ScopeId) -> Option<SourceLoc> {
    graph
        .scope(scope)?
        .extents
        .first()
        .map(|e| e.range.lo.clone())
}
