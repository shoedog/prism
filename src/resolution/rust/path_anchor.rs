//! #724: the anchored spelling of a Rust macro-argument call, for the
//! authoritative route only, and the refusals for what #724 newly admits.
//!
//! The macro-argument extractor records `super::super::f(..)` inside
//! `assert!(..)` as `callee_name = "f"` (the chain from its first identifier,
//! because `crate`/`self`/`super` are not `identifier` nodes) and keeps the
//! written prefix aside in `CallSite::path_anchor`. Every consumer other than
//! the authoritative route (the legacy `::` branch, the callers index, the
//! dumps) reads `callee_name` as recorded, so only this accessor composes the
//! path the way rustc reads it.
use crate::call_graph::CallSite;
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::NS_TYPE;
use crate::name_resolution::types::{ScopeId, ScopeKind};
use crate::resolution::{DropReason, ResolutionOutcome};

/// The site as the authoritative route should read it: the written keyword
/// prefix followed by the recorded name (`"super::super"` + `"f"` is
/// `super::super::f`). `None` when the site has no anchor.
pub(crate) fn anchored_site(site: &CallSite) -> Option<CallSite> {
    let anchor = site.path_anchor.as_deref()?;
    let mut anchored = site.clone();
    anchored.callee_name = format!("{anchor}::{}", site.callee_name);
    anchored.path_anchor = None;
    Some(anchored)
}

/// Refuse what #724 newly admits when the graph cannot vouch for it. Applies to a site
/// that was anchored (`was_anchored`) or whose path starts with a `{self as m}` alias
/// name; any other site is returned untouched (ordinary qualified paths are #765's and
/// #768's pre-existing behaviour, not changed here). Two refusals, each removed by the
/// fix of its inherited cause:
/// - a resolved target in a file under a `#[path]` mount the populator misplaced (#765):
///   it may be a decoy of the file rustc loads;
/// - a qualified path whose first segment is a module-level `{self as m}` alias while a
///   block or callable scope between the site and its module also binds `m` in the type
///   namespace: qualified paths start at the enclosing module and never see that
///   block-scope binding (#768), so the module-level alias would win where rustc binds
///   the block's.
pub(crate) fn guard<'a>(
    graph: &ScopeGraph,
    site: &CallSite,
    was_anchored: bool,
    from: ScopeId,
    out: ResolutionOutcome<'a>,
) -> ResolutionOutcome<'a> {
    let name = site.callee_name.as_str();
    let first = name.split_once("::").map(|(first, _)| first);
    let alias_routed = first.is_some_and(|f| graph.self_import_aliases.iter().any(|(_, n)| n == f));
    if !(was_anchored || alias_routed) || out.resolved.is_empty() {
        return out;
    }
    if alias_routed && first.is_some_and(|f| block_shadows_module_alias(graph, from, f)) {
        return ResolutionOutcome::dropped(DropReason::UnknownName);
    }
    let in_misplaced = |file: &str| {
        graph
            .file_paths
            .get(file)
            .is_some_and(|id| graph.misplaced_mount_files.contains(id))
    };
    if out.resolved.iter().any(|r| in_misplaced(&r.target.file)) {
        return ResolutionOutcome::dropped(DropReason::UnknownName);
    }
    out
}

/// A block/callable scope between `from` and its enclosing module binds `name` in the
/// type namespace while a module on that module's chain holds `{self as name}`.
fn block_shadows_module_alias(graph: &ScopeGraph, from: ScopeId, name: &str) -> bool {
    let mut local = Vec::new();
    let mut cur = Some(from);
    let mut module = None;
    while let Some(id) = cur {
        let Some(scope) = graph.scope(id) else { break };
        if matches!(scope.kind, ScopeKind::Module | ScopeKind::Root) {
            module = Some(id);
            break;
        }
        if matches!(scope.kind, ScopeKind::Block | ScopeKind::Callable) {
            local.push(id);
        }
        cur = scope.parent;
    }
    let mut chain = module;
    let mut alias_in_chain = false;
    while let Some(id) = chain {
        let is_module = graph
            .scope(id)
            .is_some_and(|s| matches!(s.kind, ScopeKind::Module | ScopeKind::Root));
        if is_module && graph.self_import_aliases.contains(&(id, name.to_string())) {
            alias_in_chain = true;
            break;
        }
        chain = graph.parent_of(id);
    }
    alias_in_chain
        && graph
            .bindings
            .iter()
            .any(|b| b.ns == NS_TYPE && b.name == name && local.contains(&b.scope))
}
