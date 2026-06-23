//! Scope-graph core data model — Task 1 (PR-1).
//!
//! Transcribed faithfully from the design-of-record spec §1
//! (`docs/superpowers/specs/2026-06-17-prism-rust-module-resolution-design.md`).
//!
//! **No engine / no behavior.** This module defines *only* the leaf data-model
//! types and the `ResolutionPolicy` trait signature. The graph *container*
//! (`ScopeGraph` + `MacroWildcard`) lives in [`crate::name_resolution::graph`];
//! engine logic in [`crate::name_resolution::engine`]; population in PR-2.
//!
//! ## Determinism
//! All multi-item collections use `BTreeMap`/`BTreeSet`; all key types derive
//! `Ord`/`PartialOrd`. `Vec` fields whose order is meaningful keep insertion
//! order (extent/candidate lists) — callers must insert in a stable order.
//!
//! ## Serialization stability
//! All public types derive `Serialize` + `Deserialize`. Field names are the
//! canonical names from the spec; do not rename without a spec revision.

use serde::{Deserialize, Serialize};

use crate::name_resolution::graph::ScopeGraph;

// ── Opaque IDs ──────────────────────────────────────────────────────────────

/// Identifies a scope in the scope graph.
///
/// Stable-ID derivation (from content) is Task 3; for now these are
/// assigned by the populator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ScopeId(pub u32);

/// Identifies a defining item in the repo.
///
/// Stable-ID derivation is Task 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u32);

/// Identifies a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FileId(pub u32);

/// Identifies a compilation unit (e.g., a C++ TU). Reserved for Phase 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnitId(pub u32);

// ── Open type aliases — policy-owned ────────────────────────────────────────

/// Namespace discriminant.  Policy-owned.
/// Rust: Type=0 / Value=1 / Macro=2 (conventional; policy owns the registry).
pub type NamespaceId = u16;

/// Visibility kind discriminant.  Policy-owned (policy owns the registry).
/// Rust: pub=0 / pub(crate)=1 / pub(super)=2 / pub(in)=3 / priv=4 (conventional).
pub type VisKindId = u16;

/// Edge kind discriminant.  Policy-owned (lexical ascent is structural, not an edge).
/// Rust: Glob=1 (conventional; policy owns the registry).
pub type EdgeKindId = u16;

// ── Source locations ─────────────────────────────────────────────────────────

/// A byte-offset location inside a specific file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceLoc {
    pub file: FileId,
    pub byte: usize,
}

/// A half-open byte range `[lo, hi)` within (in practice) the same file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Span {
    pub lo: SourceLoc,
    pub hi: SourceLoc,
}

// ── CfgCond — configuration-condition formula ────────────────────────────────

/// A cfg-condition formula (rev-3/M10).
///
/// `compatible(other)` and `exclusive(other)` are the two query methods;
/// the engine uses them to decide whether two conditioned candidates can
/// coexist or are mutually exclusive.
///
/// ## Conservative-on-unknown rule
/// When the formula is complex or involves atoms the engine cannot evaluate,
/// both methods fall back to the conservative answer:
/// - `compatible` → `true` (assume they may coexist → no silent merge)
/// - `exclusive`  → `false` (assume they might coexist → no silent drop)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CfgCond {
    /// Unconditionally active.
    True,
    /// `cfg(key)` or `cfg(key = "val")`.
    Atom(String, Option<String>),
    /// `not(inner)`.
    Not(Box<CfgCond>),
    /// `all(…)`.
    And(Vec<CfgCond>),
    /// `any(…)`.
    Or(Vec<CfgCond>),
}

impl CfgCond {
    /// Returns `true` when `self` and `other` *might* be simultaneously
    /// active — i.e., they are **not provably exclusive**.
    ///
    /// Conservative: unknown combinations return `true`.
    pub fn compatible(&self, other: &CfgCond) -> bool {
        !self.exclusive(other)
    }

    /// Returns `true` when `self` and `other` **cannot** be simultaneously
    /// active.
    ///
    /// Conservative: only the cases we can prove exclusive return `true`;
    /// everything else returns `false` (safe — may miss some exclusive pairs).
    pub fn exclusive(&self, other: &CfgCond) -> bool {
        match (self, other) {
            // not(x) is exclusive with x
            (CfgCond::Not(inner), _) if inner.as_ref() == other => true,
            (_, CfgCond::Not(inner)) if inner.as_ref() == self => true,

            // Two atoms with the same key but different *Some* values are exclusive
            // (e.g. target_os="linux" vs target_os="windows").
            // Atom(key, None) vs Atom(key, Some(_)): unknown — conservative false.
            // Phase 2 (before cfg EVALUATION): exclude additive keys (feature, target_feature) — they compose, so feature="a" vs feature="b" are NOT exclusive. Safe in Phase 1 (multi-candidate falls through either way).
            (CfgCond::Atom(k1, Some(v1)), CfgCond::Atom(k2, Some(v2))) => k1 == k2 && v1 != v2,

            _ => false,
        }
    }
}

// ── Occurrence (RESERVED — C++ Phase 3) ─────────────────────────────────────

/// A declaration occurrence, scoped to a compilation unit.
///
/// **RESERVED** (rev-5/round5-B4): C++ include-occurrence visibility. A
/// declaration is visible in a TU only after its include point. Population
/// and enforcement are deferred to Phase 3 (C++ populator).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Occurrence {
    pub unit: Option<UnitId>,
    /// Ordinal within the unit (include/expansion order).
    pub order: u32,
}

// ── ScopeExtent ──────────────────────────────────────────────────────────────

/// The source extent of one "opening" of a scope.
///
/// A scope may have multiple extents (C++ reopened namespaces, macro-merged
/// TS declaration namespaces).
///
/// `occ` is **RESERVED** for C++ Phase 3.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ScopeExtent {
    pub file: FileId,
    pub range: Span,
    pub cond: Option<CfgCond>,
    /// RESERVED — C++ occurrence-visibility (Phase 3).
    pub occ: Option<Occurrence>,
}

// ── ScopeKind ────────────────────────────────────────────────────────────────

/// The kind of a scope node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ScopeKind {
    /// Crate root.
    Root,
    /// A `mod foo` or package-level scope.
    Module,
    /// A block `{}` inside a function or other item.
    Block,
    /// A `struct`/`enum`/`trait`/`class` type scope.
    Type,
    /// A function or method body scope.
    Callable,
    /// The implicit extern-prelude scope (std/core/alloc + extern crates).
    ExternPrelude,
    /// A C++ translation unit (RESERVED — Phase 3).
    TranslationUnit,
}

// ── Scope ────────────────────────────────────────────────────────────────────

/// A binding region in the scope graph.
///
/// `parent` is the **lexical** enclosing scope. `extents` carries 1 or more
/// source ranges (a normal scope has exactly 1; reopened/multi-file scopes
/// accumulate more). `owner_item` links a scope to the item that owns it
/// (e.g. the `struct` that owns its field scope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub id: ScopeId,
    pub kind: ScopeKind,
    /// Lexical enclosing scope (`None` only for the crate Root).
    pub parent: Option<ScopeId>,
    /// One extent per "opening" of this scope.
    pub extents: Vec<ScopeExtent>,
    /// The item that owns this scope (if any).
    pub owner_item: Option<ItemId>,
    pub cond: Option<CfgCond>,
}

// ── Vis — open visibility ────────────────────────────────────────────────────

/// An open, policy-interpreted visibility annotation.
///
/// `kind` is a policy-assigned `VisKindId` (Rust: pub/pub(crate)/priv/…).
/// `restrict` is the scope to which visibility is restricted (for `pub(in p)`).
/// `payload` is policy-opaque extra data (reserved).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Vis {
    pub kind: VisKindId,
    pub restrict: Option<ScopeId>,
    /// Policy-opaque payload.  Use `Default::default()` unless the policy needs it.
    pub payload: PolicyBlob,
}

/// Opaque per-policy payload stored inside `Vis` and `Anchor`.
///
/// Reserved — policies may stash serialisable per-binding data here.
/// The engine treats this as an opaque blob.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PolicyBlob {
    // Phase N: policies may add typed fields here behind a feature or a
    // discriminated union.  For now the blob is empty (the unit struct
    // compiles and serializes as `{}` in JSON).
}

// ── Anchor ───────────────────────────────────────────────────────────────────

/// A path anchor — *how* a path's leading segment is rooted before the walk.
///
/// **Policy-owned / engine-opaque.** The engine never matches on `AnchorKind`;
/// only a `ResolutionPolicy::anchor` implementation interprets it (exactly as
/// `NamespaceId`/`VisKindId`/`EdgeKindId` are policy-interpreted scalars that
/// nonetheless live here in the neutral data model). The variants below are the
/// forms the Rust policy needs (Task 2); other languages may ignore them or add
/// their own conventions via `Anchor::default()` + `payload`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AnchorKind {
    /// A bare leading ident in an **expression** position (edition-lexical).
    Bare,
    /// `self::` — module-relative to the enclosing `Module`.
    SelfMod,
    /// `super::` ×n — n hops up the module ancestry.
    Super(u32),
    /// `crate::` — the enclosing crate Root.
    CrateRoot,
    /// A bare leading ident inside a **`use` path** (edition-sensitive rooting:
    /// 2015 crate-root-relative; 2018+ lexical/extern-prelude).
    UsePath,
    /// A leading `::` — 2015 crate-root-based; 2018+ extern-prelude-based.
    LeadingColon,
}

/// Opaque per-policy anchor mapping for path resolution prefixes.
///
/// The populator maps `crate::` / `self::` / `super::` / `::` / bare idents to a
/// starting scope + rule via an `Anchor`.  Policy-owned; the engine passes it
/// verbatim to `ResolutionPolicy::anchor` and never inspects the fields.
///
/// `prelude` carries the extern-prelude `ScopeId` when the populator knows it
/// (2018 `::x`); `None` ⇒ the policy anchors conservatively (fall through over a
/// wrong guess — §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Anchor {
    pub kind: AnchorKind,
    /// Extern-prelude scope, when known (for 2018 `::`/bare-crate anchoring).
    pub prelude: Option<ScopeId>,
}

impl Default for Anchor {
    /// A bare expression-position anchor with no known prelude — the most
    /// conservative default (lexical walk; never a wrong cross-module guess).
    fn default() -> Self {
        Anchor {
            kind: AnchorKind::Bare,
            prelude: None,
        }
    }
}

impl Anchor {
    /// `crate::` — root of the enclosing crate.
    pub fn crate_root() -> Self {
        Anchor {
            kind: AnchorKind::CrateRoot,
            prelude: None,
        }
    }
    /// `self::` — the enclosing module.
    pub fn self_mod() -> Self {
        Anchor {
            kind: AnchorKind::SelfMod,
            prelude: None,
        }
    }
    /// `super::` ×n.
    pub fn super_n(n: u32) -> Self {
        Anchor {
            kind: AnchorKind::Super(n),
            prelude: None,
        }
    }
    /// A bare expression-position leading ident.
    pub fn bare() -> Self {
        Anchor::default()
    }
    /// A 2015 `use`-path leading ident (crate-root-relative).
    pub fn use_path_2015() -> Self {
        Anchor {
            kind: AnchorKind::UsePath,
            prelude: None,
        }
    }
    /// A 2018 leading `::` with a known extern-prelude scope.
    pub fn leading_colon_2018(prelude: u32) -> Self {
        Anchor {
            kind: AnchorKind::LeadingColon,
            prelude: Some(ScopeId(prelude)),
        }
    }
}

// ── BindTarget / Target ──────────────────────────────────────────────────────

/// A single identifier (name segment), e.g. `"HashMap"` or `"fmt"`.
pub type Ident = String;

/// A raw (unresolved) path, as a list of name segments.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RawPath(pub Vec<Ident>);

/// An opaque reference to a local binding (for `Target::Local`).
///
/// Phase N: when the engine builds edges it records the source `Binding`
/// identity here.  For now a pair of (scope, ordinal) suffices.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BindingRef {
    pub scope: ScopeId,
    /// Insertion-order ordinal of the binding within the scope (0-based).
    pub ordinal: u32,
}

/// An opaque reference to an external (out-of-repo) definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExternRef {
    /// The canonical external name (e.g. `std::collections::HashMap`).
    pub name: String,
}

/// What a `Binding`'s name resolves to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Target {
    /// The name introduces or re-exports a *scope* (a module, type body, …).
    Scope(ScopeId),
    /// The name refers to a defining item in the repo.
    ///
    /// - `owns`: the scope this item's body introduces (for `Enum::Variant`,
    ///   `Trait::Assoc`, etc.).
    /// - `callable`: `true` only for `fn`/`method`/closure items — the engine
    ///   mints a call edge **only** for `Item{callable:true}`.
    Item {
        id: ItemId,
        ns: NamespaceId,
        owns: Option<ScopeId>,
        callable: bool,
    },
    /// A local value binding (param / `let` / closure arg / pattern).
    ///
    /// (rev-6/round6-2): shadows free-fn items in `Value` lookup but is NOT
    /// itself a call target.  `callable` must never be assumed for `Local`.
    Local(BindingRef),
    /// An out-of-repo definition (stdlib, external crate, …).
    External(ExternRef),
}

/// The resolution state of a `Binding`'s target.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BindTarget {
    /// Target already known (either from an item definition or after fixpoint).
    Resolved(Target),
    /// Target not yet resolved — a named import (`use a::b`) waiting for fixpoint.
    Pending(RawPath, Anchor),
}

// ── Binding ──────────────────────────────────────────────────────────────────

/// A definition **or** an import/re-export alias inside a scope.
///
/// `(scope, name, ns)` is the lookup key; multiple bindings with the same
/// key are legal (cfg-alternatives, overloads, genuinely ambiguous).
///
/// `vis_extents` carries the multi-region, file-qualified visibility spans
/// (block scope + "visible after def" semantics; a macro may yield disjoint
/// visible regions).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub scope: ScopeId,
    pub name: Ident,
    pub ns: NamespaceId,
    pub target: BindTarget,
    pub vis: Vis,
    pub cond: Option<CfgCond>,
    /// Multi-region file-qualified visibility spans.
    pub vis_extents: Vec<Span>,
}

// ── Edge ─────────────────────────────────────────────────────────────────────

/// A scope-to-scope (or scope-to-target) edge.
///
/// Named imports/re-exports are `Binding`s, not edges (rev-3/M7).
/// Globs (`use a::*`) are `Edge`s with a glob `EdgeKindId`.
///
/// `vis_range` is **RESERVED** for C++ include-occurrence visibility (Phase 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: ScopeId,
    pub kind: EdgeKindId,
    pub to: BindTarget,
    pub vis: Vis,
    pub cond: Option<CfgCond>,
    /// Source/declaration order (base-list position, import order).
    pub order: u32,
    /// RESERVED — C++ include-occurrence visibility (Phase 3).
    pub vis_range: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityDecision {
    Visible,
    Hidden,
    Unknown,
}

// ── Resolution results ───────────────────────────────────────────────────────

/// Why / where a candidate was found (opaque to the engine; policy-set).
///
/// Phase N: the Rust policy fills this in with a structured provenance record.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Provenance {
    // Phase N: structured provenance (scope chain, edge kinds taken, …).
}

/// One resolution candidate returned by the engine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Candidate {
    pub target: Target,
    /// The accumulated cfg condition along the path to this candidate.
    pub cond: CfgCond,
    pub provenance: Provenance,
}

/// The outcome status of a resolution query.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResStatus {
    /// Exactly one candidate — the engine may mint an edge.
    Resolved,
    /// A legitimate set (C++ overloads; same name in >1 namespace).
    ResolvedSet,
    /// ≥2 conflicting candidates under compatible cfg — consumer falls through.
    Ambiguous,
    /// A deferred/unexpanded glob or still-`Pending` import shadowed this name.
    Poisoned,
    /// No candidate found.
    Unresolved,
}

/// The result of resolving a `ResolveQuery`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub candidates: Vec<Candidate>,
    pub status: ResStatus,
}

// ── ResolveQuery ─────────────────────────────────────────────────────────────

/// Traversal context passed to `ResolutionPolicy::visible()`.
///
/// The engine populates this at each lookup point; policy implementations
/// inspect it. It carries the **lookup scope** the binding was found in and
/// whether it was reached **via a glob edge** (vs a direct rib binding) — what
/// the Rust glob-re-export / accessible-not-just-public rule needs (§4), and
/// what C++ access control will consult later.
///
/// The engine deliberately does **not** hard-code a span/`SourceLoc` visibility
/// check (that would be Rust-shaped — §1); all visibility decisions go through
/// `visible()`, which may consult these fields + the binding's own data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalCtx {
    /// The scope currently being searched (the rib the binding lives in / the
    /// glob target scope when `via_glob`).
    pub lookup_scope: Option<ScopeId>,
    /// `true` when the binding was reached through a `Glob` edge rather than as
    /// a direct binding at the current rib.
    pub via_glob: bool,
    /// The edge kind taken to reach the binding (when `via_glob`).
    pub edge_kind: Option<EdgeKindId>,
}

/// Opaque policy-query context.
///
/// C++ ADL / two-phase lookup extensions add fields here without changing the
/// engine.  **RESERVED** (round6-4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyQueryCtx {
    // Phase N: call syntax, arg-type candidates, template args, ADL classes, etc.
}

/// Opaque cfg evaluation context supplied by the caller.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfgCtx {
    // Phase N: active feature flags, target OS/arch, … needed to evaluate CfgCond.
}

/// A structured query passed to `resolve` / `resolve_path`.
///
/// Extensible — C++ ADL / two-phase lookup may add fields to `ctx` without
/// touching the engine signature.
///
/// `ctx` is **RESERVED** (round6-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveQuery {
    pub name: Ident,
    pub ns: NamespaceId,
    pub from: ScopeId,
    pub at: SourceLoc,
    pub cfg: CfgCtx,
    /// RESERVED — opaque policy-query context (ADL, call syntax, …).
    pub ctx: PolicyQueryCtx,
}

// ── ResolutionPolicy trait ────────────────────────────────────────────────────

/// A pluggable, per-language resolution policy.
///
/// The engine is language-neutral; it walks the scope graph and calls these
/// hooks at each decision point.  The Rust policy (`RustPolicy`) is
/// implemented in Task 2.  C++ / Go / Java / Python / TS-JS policies are
/// later phases.
///
/// All method bodies below are **trait signatures only**; stub `todo!()` bodies
/// are provided where a default makes no sense.  No engine logic belongs here.
pub trait ResolutionPolicy {
    /// The set of namespace IDs this policy recognises as "scope-bearing"
    /// for `resolve_path` prefix resolution (e.g. Type+Module in Rust).
    fn namespaces(&self) -> Vec<NamespaceId>;

    /// The ordered list of `EdgeKindId`s the engine should follow at each rib,
    /// in priority order (earlier = higher priority, shadows later).
    fn edge_order(&self) -> Vec<EdgeKindId>;

    /// Combine a list of raw candidates (from a single rib) into a `Resolution`.
    ///
    /// The policy decides:
    /// - single item → `Resolved`
    /// - legitimate multi-item (C++ overload set, multi-namespace) → `ResolvedSet`
    /// - conflicting items under compatible cfg → `Ambiguous`
    fn combine(&self, candidates: Vec<Candidate>) -> Resolution;

    /// Decides whether `binding` is accessible from query `q` during a traversal
    /// described by `trav`.
    ///
    /// The `trav` context supplies the current edge kind, lookup scope chain,
    /// and provenance — needed for Rust glob re-export visibility and C++ access
    /// control (round6-4 + round7-2/3).
    ///
    /// The engine does **not** hard-code a span check; all visibility decisions
    /// go through this hook.
    fn visible(&self, binding: &Binding, q: &ResolveQuery, trav: &TraversalCtx) -> bool;

    /// Decides whether a glob edge itself is accessible from query `q`.
    ///
    /// Default policies keep historical behavior: all glob edges are visible.
    fn glob_edge_visible(
        &self,
        _edge: &Edge,
        _q: &ResolveQuery,
        _trav: &TraversalCtx,
    ) -> VisibilityDecision {
        VisibilityDecision::Visible
    }

    /// Classify a member binding's visibility from the query vantage as a tri-state.
    /// Mirrors `glob_edge_visible` at the member level: `Hidden` = proved not-visible
    /// (a glob soundly does not re-export it → safe to continue past); `Unknown` =
    /// undecidable (must fail closed → poison); `Visible` = contributes.
    ///
    /// Default is intentionally conservative: a policy that only knows boolean
    /// `visible == false` returns `Unknown`, so glob expansion keeps poisoning rather
    /// than silently skipping a member it cannot prove hidden.
    fn member_visible(&self, binding: &Binding, q: &ResolveQuery, trav: &TraversalCtx) -> VisibilityDecision {
        if self.visible(binding, q, trav) {
            VisibilityDecision::Visible
        } else {
            VisibilityDecision::Unknown
        }
    }

    /// Map a path anchor (`crate::`, `self::`, `super::`, bare ident, `::`) to
    /// the starting `ScopeId` + initial `NamespaceId` for the walk.
    ///
    /// Edition-aware for Rust (2015 vs 2018+).
    fn anchor(&self, anchor: &Anchor, from: ScopeId) -> Option<(ScopeId, NamespaceId)>;

    /// During a **bare-name** inner→outer walk, after looking in a scope of the
    /// given `kind`, should the engine ascend to its lexical parent?
    ///
    /// This is the policy's lexical-ascent gate. The **Rust module-boundary
    /// stop** lives here: a bare name crosses `Block`/`Callable`/`Type` parents
    /// but **stops at the enclosing `Module`/`Root`** (Rust does not inherit
    /// unqualified names from a parent module — §4 round6-B1).
    ///
    /// Default: `true` (an unconditional lexical walk) — for policies (e.g.
    /// Python LEGB) that do walk module ancestors. Rust overrides this.
    fn ascend_to_parent(&self, _kind: &ScopeKind) -> bool {
        true
    }

    /// Inject additional candidates (C++ ADL, prelude, …) for a query.
    ///
    /// Default: no injection (returns empty).  Override for languages that need it.
    fn inject(&self, _q: &ResolveQuery) -> Vec<Candidate> {
        vec![]
    }

    /// A path's leading segment may name a depended-on in-repo crate (Rust 2018+
    /// extern-prelude root). Returns that crate's library `Root` iff the anchor is
    /// an extern-prelude root kind AND the CONSUMING crate (containing `from`)
    /// declares an in-repo dependency under `name`. `from` is needed because the
    /// extern prelude is per-crate. Default: not applicable (returns `None`).
    fn extern_crate_root(
        &self,
        _graph: &ScopeGraph,
        _name: &str,
        _anchor: &Anchor,
        _from: ScopeId,
    ) -> Option<ScopeId> {
        None
    }
}
