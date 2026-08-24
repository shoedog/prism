//! Scope-graph container — Task 1 (PR-1), split out of `types.rs` (Task 2 review).
//!
//! This is the *shape* of the resolution engine's input: the whole-repo
//! [`ScopeGraph`] (scopes + bindings + edges + macro wildcards) and its
//! builder/accessor surface. The pure leaf/data-model declarations
//! (`Scope`/`Binding`/`Edge`/`Target`/`CfgCond`/…) live in
//! [`crate::name_resolution::types`]; this module only holds the container and
//! the [`MacroWildcard`] marker it carries.
//!
//! **No engine logic.** Engine traversal lives in
//! [`crate::name_resolution::engine`].
//!
//! ## Determinism
//! `scopes` is a `BTreeMap` (sorted by `ScopeId`). `bindings`/`edges`/
//! `macro_wildcards` are `Vec`s whose **insertion order is meaningful** (see the
//! `ScopeGraph` doc).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::name_resolution::binding_lookup::LocalFact;
use crate::name_resolution::types::{Binding, Edge, FileId, NamespaceId, Scope, ScopeId, Span};

fn default_complete() -> bool {
    true
}

fn default_edition() -> u16 {
    2015
}

fn default_edition_uniform() -> bool {
    true
}

// ── MacroWildcard — unexpanded name-introducing macro (§4.3b) ────────────────

/// An *unexpanded* name-introducing macro invocation (item-position
/// `macro_rules!`/proc/attribute macro) that may emit **unknowable** names.
///
/// Phase 1 cannot compute the introduced name-set pre-expansion, so the
/// populator records a **wildcard** marker over `(scope, ns, range)`: any bare
/// lookup of `ns` whose byte falls in `range` is **poisoned** (exactly like a
/// deferred glob) — the engine must fall through, never reach an outer
/// same-name (§4.3b / §7 poison-not-skip). Full macro expansion is Phase 3.
///
/// The wildcard is **glob-tier**: an explicit local binding for the name in the
/// same scope shadows it (it poisons only when no explicit binding claims the
/// name), exactly as an explicit binding shadows a deferred glob.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MacroWildcard {
    /// The scope the macro was invoked in.
    pub scope: ScopeId,
    /// The namespace the wildcard poisons.
    pub ns: NamespaceId,
    /// The byte range the macro's potential introductions cover.
    pub range: Span,
}

// ── ScopeGraph — the engine's input shape ─────────────────────────────────────

/// The whole-repo scope graph: the shared input to the resolution engine.
///
/// Built by a language populator (Task 3 for Rust) and consumed by
/// `engine::resolve`/`resolve_path`.  Kept in the neutral data model because it
/// is the *shape* of the engine input, not engine logic.
///
/// ## Determinism
/// `scopes` is a `BTreeMap` (sorted by `ScopeId`). `bindings`/`edges`/
/// `macro_wildcards` are `Vec`s whose **insertion order is meaningful**:
/// - a `Binding`'s index within its scope is its `BindingRef::ordinal` source;
/// - an `Edge`'s `order` field carries decl order independently.
///
/// Populators must insert in a stable order; the engine never relies on `Vec`
/// position for correctness beyond honoring the policy's combination rules.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeGraph {
    /// Whether this graph came from a complete whole-workspace build.
    ///
    /// Incomplete/subset builds are non-authoritative for every consumer site.
    #[serde(default = "default_complete")]
    pub complete: bool,
    /// Rust crate edition used by edition-dependent path anchors.
    ///
    /// Phase 2: per-crate editions for mixed-edition workspaces.
    #[serde(default = "default_edition")]
    pub edition: u16,
    /// Whether every parsed manifest is on the same path-anchoring class
    /// (all 2015 or all 2018+); see repo_loader `anchoring_class_uniform`.
    /// Consumed by the `ScopeResolution` disproof predicate: a non-uniform
    /// workspace is non-authoritative for disproof (keep-all), because a
    /// wrong-edition anchor could mis-resolve and drop a real edge (P1).
    #[serde(default = "default_edition_uniform")]
    pub edition_uniform: bool,
    /// Repo-relative file path to deterministic `FileId` mapping.
    ///
    /// PR-2's populator already uses this sorted-key mapping; consumers need the
    /// path side to map call sites/import edges back onto graph file IDs.
    #[serde(default)]
    pub file_paths: std::collections::BTreeMap<String, crate::name_resolution::types::FileId>,
    /// Rust: per consuming-crate library `Root` → (in-source dependency name →
    /// the depended-on in-repo library `Root`). Built at `Builder::finish()` from
    /// each member's `[dependencies]` PATH and WORKSPACE deps that resolve to an
    /// in-repo crate (external/registry/git deps excluded). The leading-segment
    /// crate-root fallback resolves a 2018+ bare-crate leading segment ONLY through
    /// this per-crate map, so a crate can name another in-repo crate iff it actually
    /// depends on it (Rust's extern prelude is per-crate). Keys (dep names) are
    /// hyphen→underscore normalized. Other languages leave this empty.
    #[serde(default)]
    pub crate_deps_by_root:
        std::collections::BTreeMap<ScopeId, std::collections::BTreeMap<String, ScopeId>>,
    pub scopes: std::collections::BTreeMap<ScopeId, Scope>,
    pub bindings: Vec<Binding>,
    /// Rust local-binding facts keyed by `(file, def_byte)`.
    #[serde(default)]
    pub local_facts: BTreeMap<(FileId, usize), LocalFact>,
    pub edges: Vec<Edge>,
    pub macro_wildcards: Vec<MacroWildcard>,
    /// Deferred-glob expansion telemetry for resolution walked over **this**
    /// graph (spec §3.5). Scoped here rather than in a process-global so a
    /// measurement (`navigation::queries::call_stats`) that resets and snapshots
    /// it cannot pick up resolution another thread performed against a different
    /// graph. Not serialized and not part of the graph's identity — see
    /// [`GraphGlobStats`](crate::name_resolution::glob_stats::GraphGlobStats).
    #[serde(skip)]
    pub glob_stats: crate::name_resolution::glob_stats::GraphGlobStats,
}

impl ScopeGraph {
    /// An empty graph.
    pub fn new() -> Self {
        ScopeGraph {
            complete: true,
            edition: default_edition(),
            edition_uniform: default_edition_uniform(),
            ..Self::default()
        }
    }

    /// Insert a scope (keyed by its `id`).
    pub fn add_scope(&mut self, s: Scope) {
        self.scopes.insert(s.id, s);
    }

    /// Append a binding (its index becomes its scope-relative ordinal source).
    pub fn add_binding(&mut self, b: Binding) {
        self.bindings.push(b);
    }

    /// Append an edge.
    pub fn add_edge(&mut self, e: Edge) {
        self.edges.push(e);
    }

    /// Append an unexpanded-macro wildcard marker.
    pub fn add_macro_wildcard(&mut self, m: MacroWildcard) {
        self.macro_wildcards.push(m);
    }

    /// The scope record for `id`, if present.
    pub fn scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(&id)
    }

    /// The lexical parent of `id`, if any.
    pub fn parent_of(&self, id: ScopeId) -> Option<ScopeId> {
        self.scopes.get(&id).and_then(|s| s.parent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::name_resolution::types::ScopeId;

    #[test]
    fn crate_deps_by_root_round_trips_through_serde() {
        // A graph carrying a per-consuming-crate dep map must serialize and
        // deserialize the new field intact (in-memory bincode round-trip).
        let mut g = ScopeGraph::new();
        let mut deps = std::collections::BTreeMap::new();
        deps.insert("b_crate".to_string(), ScopeId(7));
        g.crate_deps_by_root.insert(ScopeId(0), deps);
        let bytes = bincode::serialize(&g).expect("serialize");
        let back: ScopeGraph = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(
            back.crate_deps_by_root
                .get(&ScopeId(0))
                .and_then(|m| m.get("b_crate")),
            Some(&ScopeId(7)),
            "the per-crate dep map must survive a serde round-trip"
        );
    }

    #[test]
    fn crate_deps_by_root_defaults_empty_on_missing_field() {
        // A new graph has an empty map (serde(default) keeps an old field-less
        // named-format blob robust; cross-VERSION compat is the CACHE_VERSION bump,
        // since the cache is bincode and deserializes before the version check).
        let g = ScopeGraph::new();
        assert!(g.crate_deps_by_root.is_empty());
    }
}
