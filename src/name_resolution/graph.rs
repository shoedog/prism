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
    /// Whether every parsed manifest agreed on one edition (spec §2 BLOCKER-2).
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
    pub scopes: std::collections::BTreeMap<ScopeId, Scope>,
    pub bindings: Vec<Binding>,
    /// Rust local-binding facts keyed by `(file, def_byte)`.
    #[serde(default)]
    pub local_facts: BTreeMap<(FileId, usize), LocalFact>,
    pub edges: Vec<Edge>,
    pub macro_wildcards: Vec<MacroWildcard>,
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
