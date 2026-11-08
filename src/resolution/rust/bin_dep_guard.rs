//! #749: the block-shadow refusal for what a package's binary roots newly reach.
//!
//! A binary root keyed with its package's in-repo `[dependencies]` resolves `alias::f()`
//! through a module-level `use a::m as alias`. A qualified path starts at the enclosing
//! module, chases that import and hands the import's own scope to the extern fallback, so a
//! block or callable scope that rebinds the path's first segment in the type namespace is
//! never seen (#768; rustc binds the block's). A result that lands in a crate the caller's
//! root reaches through a binary-root entry is therefore refused when such a block binding
//! exists. `#[path]` placement needs no guard here: it is rustc's (#765).
//!
//! Library roots and own-library edges are untouched.
use crate::call_graph::CallSite;
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::{crate_root_of, NS_TYPE};
use crate::name_resolution::types::{FileId, ScopeId, ScopeKind};
use crate::resolution::{DropReason, ResolutionOutcome};

/// `out`, or a dropped `UnknownName` when a resolved target lies in a crate reached through
/// a binary-root dependency entry of `from`'s crate root and a block or callable scope
/// between the site and its module binds the path's first segment in the type namespace.
pub(crate) fn guard<'a>(
    graph: &ScopeGraph,
    site: &CallSite,
    from: ScopeId,
    out: ResolutionOutcome<'a>,
) -> ResolutionOutcome<'a> {
    if graph.bin_dep_entries.is_empty() || out.resolved.is_empty() {
        return out;
    }
    let Some((first, _)) = site.callee_name.split_once("::") else {
        return out;
    };
    let Some(root) = crate_root_of(graph, from) else {
        return out;
    };
    let deps: Vec<ScopeId> = graph
        .bin_dep_entries
        .range((root, ScopeId(0))..=(root, ScopeId(u32::MAX)))
        .map(|&(_, target)| target)
        .collect();
    if deps.is_empty() || !block_binds_type(graph, from, first) {
        return out;
    }
    let refused = out.resolved.iter().any(|r| {
        graph
            .file_paths
            .get(&r.target.file)
            .and_then(|&id| file_crate_root(graph, id))
            .is_some_and(|target_root| deps.contains(&target_root))
    });
    if refused {
        ResolutionOutcome::dropped(DropReason::UnknownName)
    } else {
        out
    }
}

/// A block or callable scope between `from` and its enclosing module binds `name` in the type
/// namespace.
fn block_binds_type(graph: &ScopeGraph, from: ScopeId, name: &str) -> bool {
    let mut cur = Some(from);
    while let Some(id) = cur {
        let Some(scope) = graph.scope(id) else {
            return false;
        };
        match scope.kind {
            ScopeKind::Module | ScopeKind::Root => return false,
            ScopeKind::Block | ScopeKind::Callable
                if graph.binding_indices(id, name, NS_TYPE).next().is_some() =>
            {
                return true
            }
            _ => {}
        }
        cur = scope.parent;
    }
    false
}

/// The crate root of the crate whose module tree holds `file` (any scope with an extent in
/// the file names it: a file belongs to one module).
fn file_crate_root(graph: &ScopeGraph, file: FileId) -> Option<ScopeId> {
    graph
        .scopes
        .values()
        .find(|s| s.extents.iter().any(|e| e.file == file))
        .and_then(|s| crate_root_of(graph, s.id))
}
