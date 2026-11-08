//! `pub(in <path>)` restrict resolution (#710), run once after population.
//!
//! The populator cannot resolve a restrict while it walks: an out-of-line module
//! is walked before its parent's `mod` binding exists (`walk/items.rs`), so
//! `crate::a` is not yet nameable while `a/b.rs` is read. Every `pub(in ..)`
//! binding therefore records `(binding index, path)` and [`resolve_restricts`]
//! places the path in the finished graph.
//!
//! The rule only ever picks a module on the item's **own ancestor chain**
//! whose names match the path (rustc requires `pub(in path)` to name an
//! ancestor), and refuses (`restrict = None`, the unchanged "unknown
//! visibility" outcome) whenever it cannot place the path: a first segment other
//! than `crate`/`self`/`super`, a path longer than the chain, a segment that
//! does not name the next chain module through a module binding, a re-export
//! (a `use` binding is `Pending`, never a module target).

use std::collections::{BTreeMap, BTreeSet};

use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::NS_TYPE;
use crate::name_resolution::types::{BindTarget, RawPath, ScopeId, ScopeKind, Target};

/// Set `vis.restrict` of every pending `pub(in ..)` binding. A path that cannot
/// be placed stays `None`.
pub(super) fn resolve_restricts(graph: &mut ScopeGraph, pending: &[(usize, RawPath)]) {
    if pending.is_empty() {
        return;
    }
    let modules = module_bindings(graph);
    let resolved: Vec<Option<ScopeId>> = pending
        .iter()
        .map(|(idx, path)| place(graph, &modules, graph.bindings[*idx].scope, path))
        .collect();
    for ((idx, _), restrict) in pending.iter().zip(resolved) {
        graph.bindings[*idx].vis.restrict = restrict;
    }
}

/// `(declaring scope, name)` → the module scopes it names in the type
/// namespace (a resolved module binding only; a `use` alias is `Pending`).
type ModuleBindings = BTreeMap<(ScopeId, String), BTreeSet<ScopeId>>;

fn module_bindings(graph: &ScopeGraph) -> ModuleBindings {
    let mut out = ModuleBindings::new();
    for b in &graph.bindings {
        if b.ns == NS_TYPE {
            if let BindTarget::Resolved(Target::Scope(s)) = &b.target {
                out.entry((b.scope, b.name.clone())).or_default().insert(*s);
            }
        }
    }
    out
}

/// The enclosing `Module`/`Root` of `scope` (itself if it is one).
fn enclosing_module(graph: &ScopeGraph, scope: ScopeId) -> Option<ScopeId> {
    let mut cur = Some(scope);
    while let Some(id) = cur {
        let s = graph.scope(id)?;
        if matches!(s.kind, ScopeKind::Module | ScopeKind::Root) {
            return Some(id);
        }
        cur = s.parent;
    }
    None
}

/// Module chain of `scope`, innermost first: `[D, parent module, .., crate Root]`.
/// `None` when the chain does not end at a crate `Root`.
fn module_chain(graph: &ScopeGraph, scope: ScopeId) -> Option<Vec<ScopeId>> {
    let mut chain = vec![enclosing_module(graph, scope)?];
    loop {
        let top = *chain.last()?;
        match graph.parent_of(top) {
            Some(parent) => chain.push(enclosing_module(graph, parent)?),
            None => break,
        }
    }
    let root = graph.scope(*chain.last()?)?;
    matches!(root.kind, ScopeKind::Root).then_some(chain)
}

fn place(
    graph: &ScopeGraph,
    modules: &ModuleBindings,
    scope: ScopeId,
    path: &RawPath,
) -> Option<ScopeId> {
    let chain = module_chain(graph, scope)?;
    let segs = &path.0;
    // The anchor, as an index into `chain` (0 = the item's own module D).
    let (anchor, rest) = match segs.first()?.as_str() {
        "crate" => (chain.len() - 1, &segs[1..]),
        "self" => (0, &segs[1..]),
        "super" => {
            let hops = segs.iter().take_while(|s| s.as_str() == "super").count();
            // `super` of the crate root is undefined: never hop above it.
            if hops >= chain.len() {
                return None;
            }
            (hops, &segs[hops..])
        }
        // 2018+ requires an anchor; 2015 crate-relative restricts are refused.
        _ => return None,
    };
    // The remaining segments name the modules below the anchor on the way down
    // to D, in order; the path may stop short of D but never go past it.
    if rest.len() > anchor {
        return None;
    }
    let mut at = anchor;
    for seg in rest {
        let next = at - 1;
        let named = modules.get(&(chain[at], seg.clone()))?;
        if !named.contains(&chain[next]) {
            return None;
        }
        at = next;
    }
    Some(chain[at])
}
