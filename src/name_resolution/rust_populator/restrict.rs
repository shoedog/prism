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
//! (a `use` binding is `Pending`, never a module target), and any module on the chain
//! that a `#[path]` attribute mounted at a file other than rustc's.
//!
//! The populator resolves a non-inline `#[path]` relative to the declaring file's
//! `<stem>/` directory, rustc relative to the declaring file's directory (#765), so
//! the modeled tree can bind a decoy file whose synthetic chain still names the
//! restrict's modules. Such a mount is recorded as misplaced
//! ([`expected_mount_file`]) and no restrict is placed across it.

use std::collections::{BTreeMap, BTreeSet};

use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::NS_TYPE;
use crate::name_resolution::types::{BindTarget, RawPath, ScopeId, ScopeKind, Target};

/// Set `vis.restrict` of every pending `pub(in ..)` binding. A path that cannot
/// be placed stays `None`.
pub(super) fn resolve_restricts(
    graph: &mut ScopeGraph,
    pending: &[(usize, RawPath)],
    misplaced: &BTreeSet<ScopeId>,
) {
    if pending.is_empty() {
        return;
    }
    let modules = module_bindings(graph);
    let resolved: Vec<Option<ScopeId>> = pending
        .iter()
        .map(|(idx, path)| place(graph, &modules, misplaced, graph.bindings[*idx].scope, path))
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
    misplaced: &BTreeSet<ScopeId>,
    scope: ScopeId,
    path: &RawPath,
) -> Option<ScopeId> {
    let chain = module_chain(graph, scope)?;
    // A module on the chain mounted at the wrong file: the modeled ancestry is not
    // rustc's, so the path cannot be checked against it.
    if chain.iter().any(|m| misplaced.contains(m)) {
        return None;
    }
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

/// The file rustc loads for `#[path = rel]` on a non-inline `mod` declared in
/// `decl_file`, whose populator child directory is `ctx_dir`.
///
/// The path is relative to the declaring file's directory. In a non-mod-rs file
/// (not `mod.rs`, not a crate root, not itself `#[path]`-mounted) the populator's
/// child directory is `<stem>/`, so that case steps back to the file's directory;
/// everywhere else (root files, `mod.rs`, mounted files, inside inline `mod`
/// blocks, where both rules add the same components) the two directories agree.
pub(super) fn expected_mount_file(decl_file: &str, ctx_dir: &str, rel: &str) -> String {
    let own_child_dir = decl_file
        .strip_suffix(".rs")
        .map(|stem| format!("{stem}/"))
        .unwrap_or_default();
    let is_mod_rs = decl_file == "mod.rs" || decl_file.ends_with("/mod.rs");
    let dir = if !is_mod_rs && ctx_dir == own_child_dir {
        match decl_file.rfind('/') {
            Some(i) => &decl_file[..=i],
            None => "",
        }
    } else {
        ctx_dir
    };
    format!("{dir}{rel}")
}
