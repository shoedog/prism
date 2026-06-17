# Name Resolution — Scope-Graph Architecture Design Spec (Rust first, C++-general)

> **Status: SOUND-WITH-CONCERNS — codex round 7 = GO for Rust Phase 1** (no remaining core-shape blocker;
> "comprehensive enough to proceed to Rust Phase 1 slicing"). **Design-of-record, pending the owner's
> explicit go-for-slicing** (HOLD on plan/development is owner-gated). **Rev 7** folds the 5 round-7
> clarifications (all non-core): macro **wildcard** poison (§4.3b), C++ occurrence-visibility-by-policy
> sentence (§1), `visible(binding,q,trav)` traversal-context signature (§1), qualified-`::` Phase-1-
> safety-vs-Phase-3-precision wording (§9), stronger stable-ID keys (§8). **Rev 6** folded round-6 (which
> judged the design *"close enough to proceed with Rust Phase 1 after [these] fixes"*): (1) **module-boundary STOP** for bare Rust names — a nested module does NOT see
> a parent module's unqualified names (§4, the round-6 BLOCKER); (2) **`Target::Local` + `Item.callable`**
> so a local `let f` shadows a free `fn f` without minting a wrong call edge; (3) Phase-1 **qualified `::`
> graph-resolves or falls through** (not left on the legacy heuristic — §10); (4) a structured
> **`ResolveQuery`** + a **`visible()` policy hook** (ADL/occurrence visibility are policy, not engine —
> makes the C++ reservation adequate); + §6 terminology / per-kind extents. Rev 5 had folded the 3 Rust
> soundness fixes + **opened `Vis`/`EdgeKind`** (→ Go/Java/Python/TS-JS data-model-complete) + reserved
> C++. **Net: Rust is ~sound (1 confirming review from go); js/ts/java/go data-model-complete; C++
> reserved.** History: Rev 1 module-map (FLAWED) → Rev 2 scope graph (Néron–Tin–Visser–Wachsmuth) → Rev 3
> refine (model endorsed) → Rev 4 data/policy split + pressure test → Rev 5 open Vis/EdgeKind. Companion
> plan (the F3 win) **deferred** until SOUND.
>
> **Standard:** comprehensive schematic up front, no naive approximation (prior approximations here
> missed precision/recall and forced rewrites). The data model must *represent* the full system;
> *populating/consuming* it is phased.

---

## §1 Goal, the model, the seam

**Goal:** sound, complete-by-design answers to "what does this Rust path/`use`/call name refer to, in
which scope, in which file(s)" — driving (a) `nav module-deps`/`repo-map` precise import/re-export
edges and (b) block-scope-aware import-aware call narrowing (the F3 fan-out fix). Designed so C++
`using`/namespace resolution reuses the same core.

**The core is a language-neutral scope-graph DATA model + a pluggable per-language resolution POLICY**
(rev-4/B2,M7). The *data* (scopes/bindings/edges/conditions) is shared across all languages; the
*lookup rules* (edge order, candidate combination, well-formedness/overload-coherence, anchors,
namespace set) are a `ResolutionPolicy` plug-in — because C++ multi-phase lookup + ADL + overload sets
cannot be expressed as "just more data" over a fixed Rust-style walk.

```
ScopeId; ItemId; FileId
struct SourceLoc { file: FileId, byte: usize }                 // rev-4/B1: location is file-qualified
struct Span { lo: SourceLoc, hi: SourceLoc }                   // (same-file lo/hi in practice)
type NamespaceId = u16   // OPEN, policy-owned (Rust: Type/Value/Macro; TS: type/value; Java: type/method/…)
type VisKindId = u16     // rev-5/round5-B2: OPEN visibility kind, policy-interpreted (Rust pub/pub(in)/
                         // priv; Java public/protected/package/private; C++ public/protected/private/friend)
type EdgeKindId = u16    // rev-5/round5-B3: OPEN edge kind (Rust glob; Java on-demand-import; Python/TS/
                         // Java/C++ inheritance/base; C++ using-namespace/inline-ns; prelude)
// CfgCond is a FORMULA (rev-3/M10): And/Or/Not/Atom(key,val?) + conservative compatible()/exclusive()
enum CfgCond { True, Atom(key, Option<val>), Not(Box), And(Vec), Or(Vec) }
struct Vis { kind: VisKindId, restrict: Option<ScopeId>, payload: PolicyBlob }  // rev-5: OPEN, not a closed enum
struct Occurrence { unit: Option<UnitId>, order: u32 }   // rev-5/round5-B4 RESERVED: C++ include/expansion
                                                         // occurrence (a decl is visible only after its point)
struct ScopeExtent { file: FileId, range: Span, cond: Option<CfgCond>, occ: Option<Occurrence> }  // rev-5: occ replaces bare tu
struct Scope {
    id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>,    // lexical parent
    extents: Vec<ScopeExtent>,                                // 1+ (reopened ns / macro / merged TS ns)
    owner_item: Option<ItemId>,                               // an Item that owns this scope
    cond: Option<CfgCond>,
}
enum ScopeKind { Root, Module, Block, Type, Callable, ExternPrelude, TranslationUnit }  // +TU (C++)

struct Binding {                 // a definition OR an import/re-export alias
    scope: ScopeId, name: Ident, ns: NamespaceId,
    target: BindTarget, vis: Vis, cond: Option<CfgCond>,
    vis_extents: Vec<Span>,      // MULTI-region, file-qualified visibility (block scope + "visible after
                                 // def"; a macro may have disjoint visible regions)
}
enum BindTarget { Resolved(Target), Pending(RawPath, Anchor) }   // Anchor is OPAQUE/policy-owned (rev-5/round5-8)
enum Target {
    Scope(ScopeId),
    Item { id: ItemId, ns: NamespaceId, owns: Option<ScopeId>, callable: bool },  // owns: enum->variants, etc.
    Local(BindingRef),           // rev-6/round6-2: a local value binding (param/let/closure/pattern) —
                                 // SHADOWS (stops lookup) but is NOT an in-repo item; consumers mint a
                                 // call edge ONLY for `Item{callable:true}`, never `Local`/`Scope`/non-callable.
    External(ExternRef),
}

struct Edge {                    // rev-5/round5-B3: open kind + provenance/order
    from: ScopeId, kind: EdgeKindId, to: BindTarget, vis: Vis, cond: Option<CfgCond>,
    order: u32,                  // source/decl order (base-list position, import order) — policy uses it
    vis_range: Option<Span>,     // rev-5/round5-B4 RESERVED: edge usable only past this point (C++ include)
}
// EdgeKindId registry is policy-owned: Rust {Lexical, Glob}; Java {Lexical, OnDemandImport, Inherit};
// Python {Lexical, GlobImport, InheritMRO}; TS/JS {Lexical, GlobImport, Inherit}; C++ {Lexical,
// UsingDirective, Inherit, InlineNs, ...}. The shared engine treats kinds opaquely; the policy orders
// + interprets them (e.g. Python MRO uses Inherit edges' `order`). Named imports/re-exports stay Bindings.
```

**Resolution = data + a `ResolutionPolicy`, driven by a structured `ResolveQuery` (rev-6/round6-4).**
The single entry is `resolve(q: ResolveQuery)`; `resolve_path` is the multi-segment form (prefix
segments resolve in the policy's scope-bearing namespaces — Type/Module/Enum/Trait — to `Scope`/
`Item.owns` targets, longest-prefix; final segment in `q.ns`).

```
struct ResolveQuery {            // extensible — C++ ADL/two-phase add fields without changing the engine
    name: Ident, ns: NamespaceId, from: ScopeId, at: SourceLoc, cfg: CfgCtx,
    ctx: PolicyQueryCtx,         // RESERVED (round6-4): call syntax, arg-type candidates, template args,
                                 // associated ns/classes, ordinary-lookup result (C++ ADL) — opaque to engine
}
```
The **policy** supplies: per-rib **edge order**; **candidate-combination** (single `Resolved` vs overload
`ResolvedSet` vs `Ambiguous`); anchor mapping; candidate **injection** (C++ ADL, from `q.ctx`); and —
critically (round6-4 + round7-2/3) — a **`visible(binding, q, trav) -> bool` HOOK** where `trav` carries
the **traversal context** (current edge, current lookup scope, provenance — needed for Rust glob-re-export
visibility + C++ access). The engine does **not** hard-code a span/`SourceLoc` check (that would be
Rust-shaped). **Binding visibility may be occurrence-qualified by the policy** (round7-2): for C++ header
inclusion, `visible()` may consult `ResolveQuery.ctx` + the binding's `ScopeExtent.occ` (a decl is visible
in a TU only *after* its include point) — visibility is fully a policy concern, not only an edge/span test.
The shared engine walks the graph + calls these hooks. Result is **per-candidate**:

```
struct Candidate { target: Target, cond: CfgCond, provenance: Provenance }  // why/where it came from
enum ResStatus { Resolved, ResolvedSet, Ambiguous, Poisoned, Unresolved }
struct Resolution { candidates: Vec<Candidate>, status: ResStatus }
```
Per-candidate `cond` keeps cfg-exclusive duplicate targets distinct (never merged); `ResolvedSet` (legit
multiple: C++ overloads, multi-namespace) is distinct from `Ambiguous` (conflict). `Poisoned` = a
deferred/unexpanded glob could shadow → fall through. Consumers narrow/edge only on a `Resolved`
single-in-repo candidate; everything else falls through (§7).

**Seam invariant (rev-4):** the **data model** (`Scope`/`Binding`/`Edge`/`CfgCond`/`SourceLoc`) is
language-neutral and shared. Each language supplies **two plug-ins**: a **populator** (builds neutral
scopes/bindings/edges from its AST) and a **`ResolutionPolicy`** (edge order, glob/visibility rules,
candidate combination/coherence, anchor mapping, namespace registry, optional candidate injection). No
language-specific concept (`crate::`, editions, ADL, overload coherence, capitalization-visibility)
lives in the data model or the shared engine — only in a populator/policy. §4 specifies the **Rust**
populator+policy in full; §5 **pressure-tests** the data model against Go, Java, Python, TS/JS, and C++
(each: its populator+policy sketch + "does it force a *data-model* change, or only a policy?").

---

## §2 The complete problem space (requirements the model is checked against)

✦ = correctness-critical for common code (must be Phase-1-correct: modeled, or strict-fall-through —
never *wrong*). Each row maps to scope-graph elements in §6.

### §2.1 Rust crate graph
- ✦ Crate roots: `src/lib.rs`, `src/main.rs`, `src/bin/*.rs`, `src/bin/<n>/main.rs`; `tests/`,
  `benches/`, `examples/` (each its own crate/root scope). ✦ **Workspaces** (`[workspace] members`):
  each member is a crate. ✦ **Dep extern-names + renames** (`bar = { package = "foo" }`): the in-source
  name is `bar`; in-workspace deps → that member's Root scope, else `External`. Editions (2015/18/21/24)
  change anchor semantics (§2.3). `[lib]`/`[[bin]]`/… path overrides; `autobins`.
### §2.2 Rust module/scope tree
- ✦ `mod foo;` → `<declaring-module-dir>/foo.rs` **or** `/foo/mod.rs` (declaring *module's* dir, not
  the file's). ✦ inline `mod foo {}` (nested; own path + dir segment). ✦ **block scopes:** `use`/items
  inside any `{}` (fn body, inner block) bind only within that range. `#[path="…"]` mod. ✦ cfg-gated /
  **mutually-exclusive cfg duplicate** mods (`#[cfg(unix)] mod imp;` + `#[cfg(windows)] mod imp;`):
  conditioned, never merged. `foo.rs` **and** `foo/mod.rs` both present = a rustc **error/ambiguous**
  binding (not "prefer one"). Visibility on mods/items.
### §2.3 Rust paths & `use`
- ✦ Anchors: `crate::`, `self::`, `super::`×n (of the **lexical** scope), bare leading ident, `::name`
  — **edition-dependent**: 2015 `use` paths are crate-root-relative, non-`use` bare paths lexical; 2018+
  bare leading ident = name lookup w/ **extern-prelude** participation (not a dep-table check). ✦ `use`
  forms: simple/alias/nested-group/`{self, …}`/glob/`pub use`(re-export)/`pub(in …)`. ✦ **item-vs-module**
  (`use crate::engine` vs `…::start`) → longest-scope-prefix. ✦ **re-export chains** (`pub use` of a
  `pub use`) — edge-follow with cycle guard. ✦ **glob re-export** (`pub use a::*`). Macros (3rd
  namespace; `#[macro_export]`, `#[macro_use]`, `macro_rules!` textual order). `extern crate … as`.
  Prelude (implicit).
### §2.4 Rust item/namespace resolution
- ✦ **3 namespaces** (Type, Value, Macro): one ident may bind in several (unit/tuple struct = Type +
  Value; enum variants = Value). `use` can import multiple namespaces at once. Call-narrowing reads the
  **Value** namespace; module-deps is namespace-agnostic (edge to file).
### §2.5 C++ (the reuse target — §5)
- Reopened/anonymous/inline namespaces; class/enum scopes; `using namespace` (≈Glob) / `using` decl
  (≈Import); overload sets (multiple Value bindings per name); ADL; header inclusion (a namespace spans
  many files → a scope with many file-ranges).

---

## §3 The scope-graph core (language-neutral)

### §3.1 Scopes & lexical structure
A `Scope` is any binding region (`ScopeKind`); `parent` is the lexical enclosing scope; `(file, range)`
gives source extent so **block-local `use`** binds only for calls whose byte is within `range`
(BLOCKER-1 fix: scope+range, not `ModuleKey`). A file contributes many scopes (inline mods, blocks); a
logical scope may span many file-ranges (C++ reopened namespace) — so scopes ↔ files is many-to-many,
not a `file→module` function.

### §3.2 Bindings (namespace-qualified, conditioned)
`(scope, name, ns) → Target`, with `Vis` + `CfgCond` + `range`. **Multiple bindings per `(scope,name,
ns)`** are legal: cfg-alternatives (distinct `cond`) or genuine ambiguity (rustc error — recorded,
surfaced, never silently picked). `Target::Item` carries the defining `ItemId` + its namespace
(BLOCKER-2 fix: namespaces + item identity preserved, not `Option<Ident>`).

### §3.3 What carries a name vs what reaches other scopes (rev-3)
- **Named imports/re-exports are `Binding`s** (not edges): `use a::b [as c]` and `pub use a::b` add a
  `Binding{name=c|b, target=Pending(a::b path), vis}` to the importing scope; the `Pending` target is
  resolved at the fixpoint (rev-3/B1), following re-export chains with a cycle guard. `pub use` ⇒
  `vis=Public` (a re-export); private `use` ⇒ `vis=Private`.
- **Globs are `Edge`s** with `vis` (rev-3/M7): `use a::*` ⇒ private `Glob` edge; `pub use a::*` ⇒
  `Public` `Glob` edge (re-export). A glob brings *all* of the target scope's visible bindings of the
  requested `(name, ns)` — resolved on demand; if expansion is **deferred**, it **poisons** (§3.4).
- `Lexical` edges are the parent chain. All carry `CfgCond`.

### §3.4 Resolution engine (policy-driven; the order below is the Rust *policy* — rev-4)
The shared engine walks scopes inner→outer and consults the active `ResolutionPolicy` at each decision
(edge order, candidate combination, injection). **The per-rib order shown here is the Rust policy's**
(§4); another language's policy supplies its own order/combination over the same engine + data — e.g.
C++ injects ADL candidates and combines overloads into a `ResolvedSet` (§5). `resolve_name(name, ns,
from_scope, at: SourceLoc, cfg)` walks inner→outer; under the **Rust policy, at each scope, in order**:
1. **Local explicit `Binding`s** for `(name, ns)` with `vis_range ∋ at_byte`, accessible (`Vis`), and
   `cfg`-compatible — resolving any `Pending` target at the fixpoint (cycle-guarded). A hit here
   **shadows** everything outward (explicit/local beats glob beats outer — the Rust rib rule).
2. Else **this scope's `Glob` edges**: union their **accessible** `(name, ns)` bindings (the policy's
   `visible` hook decides accessibility — §1; for Rust local lookup that's accessible-not-just-public,
   §4). **If any in-scope glob is unexpanded/deferred, return `Poisoned`** — consumers fall through;
   never silently skip to a lower-priority outer target.
3. Else recurse to the **lexical parent** — but the **policy decides where to stop**: the Rust policy
   stops a *bare-name* walk at the enclosing `Module` (no parent-module inheritance — §4 round6-B1),
   so step 3 is policy-gated, not an unconditional outward walk.
`resolve_path(segments, ns, …)` (rev-3/M9): resolve `segments[..n-1]` in the **Type/Module** namespace
to a `Scope` (longest-module-prefix), then `segments[n-1]` in `ns` within that scope. **Anchors**
(`crate`/`self`/`super`/bare/`::`) are pre-mapped by the populator's edition-aware `Anchor` to the
starting scope + namespace rules (§4.4) before the walk.
**Result status:** exactly one ⇒ `Resolved`; a legitimate set (C++ overload set; the same name bound in
>1 namespace) ⇒ `ResolvedSet` (rev-3/B2, NOT ambiguity); ≥2 conflicting items under compatible cfg ⇒
`Ambiguous`; deferred-glob ⇒ `Poisoned`; nothing ⇒ `Unresolved`. `External` (dep/prelude) is a
`Resolved`/`ResolvedSet` to `External` (known, files `[]`). Only `Resolved`(/single-file `ResolvedSet`)
to an in-repo target narrows/edges; all else falls through (§7).

### §3.5 Conditions (cfg) — a formula, never merge exclusive worlds (rev-3/M10)
`CfgCond` is a **formula** (`And/Or/Not/Atom`), not opaque text, with conservative `compatible()` /
`exclusive()` (unknown atoms ⇒ treated as maybe-compatible). Resolution **accumulates the conjunction**
of scope+edge+binding conditions along a path. Mutually-exclusive duplicate mods (`cfg(unix)` vs
`cfg(windows)`) are distinct conditioned bindings (rev-3/M6-cfg) — never merged. If two candidates'
conditions are **not provably exclusive** and they differ, that's `Ambiguous`/fall-through (recall-safe),
**not** a silent merge. Full cfg *evaluation* (selecting a target) is deferred; the formula + the
compatible/exclusive lattice are present now (so deferral never yields a wrong merge).

### §3.6 The seam (rev-4: data + two plug-ins)
The above is the entire language-neutral **data model + engine**. Each language supplies (1) a
**`Populator`** (AST → neutral scopes/bindings/edges) and (2) a **`ResolutionPolicy`** (namespace
registry; per-rib **edge order**; glob/visibility rules; **candidate combination** — single `Resolved`
vs overload `ResolvedSet` vs `Ambiguous`; anchor mapping; optional candidate **injection** e.g. C++
ADL; well-formedness/coherence predicates). The shared engine walks the graph and calls the policy at
each decision point. Consumers depend only on `resolve_name`/`resolve_path` + `Resolution`. §4 = the
Rust populator+policy; §5 pressure-tests Go/Java/Python/TS-JS/C++. Nothing in §3 mentions any language.

---

## §4 Rust populator + policy (specified in full)
**Populator** (builds neutral data):
1. **Crate graph:** Cargo.toml(s) (workspace members, `[lib]`/`[[bin]]` paths, edition, `[dependencies]`
   + `package=` renames) with filename-convention fallback → a `Root` scope per crate; dep extern-names
   → `External` or edges to member Roots.
2. **Module/scope tree:** per crate, worklist from Root: `mod_item` (name, `#[path]`, body=inline,
   `#[cfg]`) → `Module` scopes via the **declaring module's directory** (foo.rs→foo/, foo/mod.rs→foo/,
   inline→its path dir); `foo.rs`+`foo/mod.rs` both → ambiguous binding. **Block scopes** for fn bodies /
   inner blocks that introduce `use`/items. Item defs that own a scope (struct/enum/trait) set
   `Scope.owner_item` + `Target::Item.owns` (rev-4/M6, for `Enum::Variant`/`Trait::Assoc` paths).
3. **Bindings & edges (rev-4/M9 terminology):** item defs → `Binding`s in `{Type, Value, Macro}` (unit/
   tuple struct → Type+Value; enum variant → Value). `use a::b [as c]` → a **`Binding{target=Pending}`**
   (named import); `pub use a::b` → same with `vis=Public` (**re-export**). `use a::*` → a **`Glob` Edge**
   (`vis=Private`); `pub use a::*` → a `Public` `Glob` edge. Visibility (`pub`/`pub(in p)`/private) + cfg
   attached to bindings/edges; `vis_extents` = the lexical block/after-def region(s).
3a. **Local value/pattern bindings — Phase 1 (rev-5/round5-B1, a soundness BLOCKER, not deferrable):**
   fn params, `let`/pattern bindings, closure args, `for`/`match`/`if let` patterns → **`Value`-namespace
   `Binding`s** in their block scope, `vis_extents` = the in-scope region. They **shadow free-fn items**
   in `Value` lookup, so `fn f(){} fn g(){ let f=||{}; f() }` does NOT mis-narrow `f()` to the free fn.
   (Target may be a non-callable/local binding; the point is it *shadows* + thus narrowing stops there or
   falls through — never resolves to the outer fn.)
3b. **Name-introducing macros — Phase 1 WILDCARD poison (rev-5/round5-7 + rev-7/round7-1):** an
   *unexpanded* item-position `macro_rules!`/proc/attribute macro can introduce *unknowable* names, so
   Phase 1 must **wildcard-poison** the affected namespace/range of that scope (NOT a known name-set —
   "affected names" isn't computable pre-expansion) — exactly like a deferred glob. So `m!(); f()` where
   `m!` may emit `fn f` ⇒ `f` is `Poisoned` → fall through, never an outer in-repo `fn f`. (`macro_rules!`
   textual scope, when populated: precise `vis_extents` — a later same-name macro shadows an earlier one,
   and an outer macro becomes visible again after an inner shadowing scope ends.) Full expansion = Phase 3.

**Rust `ResolutionPolicy`:** namespaces `{Type, Value, Macro}` (scope-bearing for `resolve_path`
prefixes: Type/Module + `Item.owns`); per-rib order **local explicit Binding (incl. value/pattern, 3a) →
this-scope `Glob` → lexical parent**, explicit/local shadows glob shadows outer.
**MODULE-BOUNDARY STOP (rev-6/round6-B1 — a soundness BLOCKER fix):** for a **bare (unqualified) name**,
the rib walk crosses only `Block`/`Callable` lexical parents **up to and including the enclosing
`Module`, then STOPS** — Rust does NOT inherit unqualified names from a *parent module* (`fn start(){}
mod m { fn g(){ start() } }` does not see the outer `start`; it needs `super::start`/an import). Crossing
into a parent `Module` for a bare name ⇒ `Unresolved` → fall through (never the parent/root item). Only
explicit anchors (`super::`/`crate::`/`self::`) + the extern-prelude reach module ancestors.
**Glob accessibility (rev-5/round5-6):** for *local* lookup a `Glob` brings names **accessible** at the use site (incl.
`pub(super)`/`pub(in)` visible there — NOT "public only"); a *`pub use *` re-export* exposes only names
public at the re-export site. A **deferred/unexpanded glob OR a still-`Pending` local import ⇒
`Poisoned`** for the affected name (an unresolved local `use` must **not** continue outward to an outer
same-name — rev-5/round5). Candidate combination = single `Resolved` (or `ResolvedSet` only across
distinct namespaces); two globs to different items ⇒ `Ambiguous`. **anchors (corrected, rev-4/M4):**
- `crate::` → crate Root; `self::`/`super::`×n are **module-relative** (walk up to the enclosing
  `Module`, then ancestors) — these appear only in *paths*.
- **2015:** a **`use` path** is crate-root-relative; `::x` is crate-root-based; a **bare non-`use`
  (expression) path is LEXICAL** — block-local `use`/items first, then outward modules (rev-4/M4 — the
  rev-3 "module-relative" was wrong).
- **2018+:** `::x` is **extern-prelude**-based; a bare leading ident is normal lexical lookup with
  **local bindings shadowing the extern-prelude** (only an unshadowed leading ident hits a crate).
- `extern crate … as` → alias binding in the crate Root. Edition from Cargo.toml (default 2015 if
  omitted, per Cargo); no manifest for a root ⇒ anchor conservatively (fall through over a wrong guess).

## §5 Pressure test: does the data model host every prism language? (owner-requested)
For each language: its populator+policy sketch, and the verdict — **does it force a *data-model* change,
or only a populator/policy plug-in?** (The data model = §1/§3; a per-language policy/populator is the
sanctioned extension point.) This validates the model as prism's universal name-resolution foundation
before slicing.

- **Go.** Populator: a package = a multi-extent `Module` (all files in the dir); file + block scopes.
  `import "p"` → `Binding`(alias→`External`/in-repo package); `import . "p"` → `Glob`; `import _` →
  no binding (side-effect). Policy: **visibility = capitalization** (exported iff Capitalized — a `Vis`
  predicate); namespaces ~1 (+ labels); no module nesting; package-scoped lookup. **Verdict: policy +
  populator only — no data-model change.** (prism already has Go package-dir import narrowing +
  embedding/interface dispatch — the scope graph *unifies* them.)
- **Java.** Populator: `package a.b;` → `Module`; class/interface → `Type` scopes (nested, inheritance
  via `Glob`-like edges to super/interface scopes); `import a.b.C` → `Binding`; `import a.b.*` →
  on-demand `Glob`; `import static …` → static `Binding`/`Glob`. Policy: **4 visibility levels**
  (public/protected/package-private/private) = `Vis` + a policy predicate; **single-type-import shadows
  on-demand-glob** (a policy ordering rule); multiple namespaces (type/method/field/package) →
  `NamespaceId`; classpath → `External`. **Verdict: policy + populator — no data-model change.**
- **Python.** Populator: module/package (files + `__init__.py`) → `Module`; function/class/comprehension
  → `Block`/`Type`; `import m [as n]`/`from m import x [as y]` → `Binding`(Pending); `from m import *` →
  `Glob`; class inheritance → `Glob`-like edges to base `Type` scopes. Policy: **LEGB** (lexical walk +
  a builtins `ExternPrelude`); **MRO = a policy candidate-ordering** over base edges (C3 linearization);
  `global`/`nonlocal` = policy rebind; ~1 namespace; dynamic (`getattr`/monkeypatch) → fall through.
  **Verdict: policy + populator — no data-model change. Closes `from_import_alias` (alias `Binding`) +
  `inherited_override` (inheritance edge + MRO policy).**
- **TS/JS.** Populator: ES-module/file `Module`; `let`/`const` `Block` vs `var`/function hoisting →
  `vis_extents`; `import {x}`/`import * as ns`/default/`require` → `Binding`/`Glob`; class inheritance →
  base-`Type` edges; **TS `namespace` + declaration merging** (interface+namespace+class share a name) →
  **multi-extent `Module` + multiple `Binding`s**. Policy: **type-vs-value namespaces** (a name bound in
  both) → `NamespaceId`; hoisting via `vis_extents`. **Verdict: policy + populator — no data-model
  change** (multi-extent + multi-binding already cover declaration merging).
  *(Round-5 fix:* the previously-closed `EdgeKind`/`Vis` made the "fits" claim false for Java/Python/TS
  inheritance + access — rev-5 OPENS both (`EdgeKindId` + `order`/provenance, `Vis{kind,payload}`), which
  is precisely what makes them fit; see each verdict above.)*
- **C++ (the stress case).** Populator: namespaces → `Module`, **reopened → multi-extent**, **anonymous
  → per-TU** (`TranslationUnit` scope), **inline → bidirectional transparent edges**; classes → `Type`
  + access; `using namespace` → open `Inherit`/`UsingDirective` edge; `using N::x` → `Binding`;
  **overloads → multiple `Value` `Binding`s**. Policy (**the deferred slot**): multi-phase lookup,
  **ADL = candidate injection** (needs call-site **query-context API** — rev-5/round5-5, RESERVED),
  overload coherence → `ResolvedSet`, **include-occurrence order** (`ScopeExtent.occ` + `Edge.vis_range`
  — rev-5/round5-B4, RESERVED: a decl is visible only *after* its include point). **Verdict: data model
  RESERVES the shapes** (`occ`/`TranslationUnit`/`Edge.vis_range` + the policy query-context extension) —
  **no re-architecture when C++ is built**; the lookup complexity is the (deferred) C++ policy.

**Summary verdict (rev-5).** With the **opened `Vis`/`EdgeKind` (+ `order`/provenance)**, the data model
hosts **Rust, Go, Java, Python, TS/JS as DATA-MODEL-COMPLETE** — they fit *now* with populator+policy
plug-ins and **no data-model change** (their policies are built in later phases; nothing about them is
"reserved"). **Only C++ keeps RESERVED slots** (`occ`/include-occurrence + the ADL query-context API) —
specified enough to guarantee no re-architecture, full lookup deferred. So js/ts/java/go are
*sufficiently architected* (owner's rev-5 ask), and per-language name-resolution work belongs **inside**
this effort (not standalone-parallel — it would be redone).

## §6 Form → scope-graph mapping (coverage check)
| Rust form | Scope-graph elements | Phase |
|---|---|---|
| block-local `use` | `Binding(Pending)` in a `Block` scope (named import — NOT an edge); `vis_extents`-gated | 1 ✦ |
| local value/pattern binding (param/`let`/closure/`for`/`match`) | `Value` `Binding` → `Target::Local`; shadows free fns | 1 ✦ |
| bare name does NOT cross a module boundary | rib walk stops at enclosing `Module` (round6-B1) | 1 ✦ |
| 3 namespaces; multi-ns `use` | `(name, ns)` binding keys; ns-tagged `Target::Item` | 1 ✦ |
| `crate`/`self`/`super`/bare/`::` (editions) | `Anchor` resolver → scope+edge starts | 1 ✦ |
| `mod foo;` correct dir; inline; nested | `Module` scopes w/ declaring-dir rule | 1 ✦ |
| workspace / multi-crate / dep renames | Root scope per crate; extern-name edges | 1 ✦ |
| `foo.rs`+`foo/mod.rs` both | ambiguous `Binding` (error state) | 1 ✦ |
| `pub use` re-export **chain** | `Public`-vis `Binding(Pending)`; fixpoint + cycle guard | 1 ✦ (concrete follow; else fall through) |
| `#[path]` mod | populator file override | 1 ✦ (detect → correct file or fall through) |
| cfg / exclusive-cfg dup mods | `CfgCond` formula on scopes/bindings; conditioned, never merged | 1 (represent) / 2 (evaluate) |
| glob `use a::*` (+ `pub use a::*`) | vis-carrying `Glob` edge; **deferred ⇒ poison/fall-through** | 1 edge+poison / 2 expand |
| visibility `pub`/`pub(in …)`/private | `Vis`; **enforced in resolve OR fall through** (rev-3/M6) | **1** (enforce-or-fall-through) |
| overload set (C++) / multi-namespace | multiple `Binding`s ⇒ `ResolvedSet` (≠ `Ambiguous`) | core now |
| multi-extent scope (reopened ns / macro) | `Scope.extents: Vec<ScopeExtent>` | core now |
| macros (`#[macro_export]`, `macro_rules!`) | Macro-ns `Binding`s; `vis_range` textual-order | 2–3 |
| prelude / `#[no_implicit_prelude]` | `ExternPrelude` scope | 2–3 |
| **NOT modeled (out):** proc-macro/`build.rs`-generated items, full macro **expansion**, `include!()` | — degrade to `Unresolved`→fall through | — |

## §7 Invariants (recall-safety is structural)
- Narrow/edge **only** on `Resolved` (or a single-in-repo-file `ResolvedSet`) for the right `(name, ns)`
  in the call's scope+byte; `ResolvedSet`(multi-file)/`Ambiguous`/`Poisoned`/`Unresolved`/`External` →
  **fall through, never wrong** (rev-3/M4–M5: no "resolve to the facade", no skipping a poisoning glob).
- Block-scope (`vis_range`) + namespace + **visibility (enforced)** + cfg are all *filters in resolve* —
  a path crossing an un-enforced visibility/edition rule **falls through**, never resolves wrong (M6).
- **Local-binding shadow (rev-5/round5-B1):** a local `Value` binding (param/`let`/closure/pattern) in
  range shadows a free-fn item — narrowing must see it (resolve there or fall through), never reach past
  it to an outer `fn`. **Poison-not-skip (rev-5):** an unexpanded name-introducing macro, a deferred
  glob, or a still-`Pending` local `use` ⇒ `Poisoned` for that name → fall through; never continue
  outward to an outer same-name.
- cfg formula, over-approximate, never silently drop/merge exclusive worlds. Determinism: stable
  `ScopeId`/`ItemId` derivation (spec'd in §8) + sorted structures.

## §8 Incremental & cache
**Whole-repo/workspace** scope graph (not diff-scoped); whole-program rebuild after any incremental
merge — a `mod`/`#[path]`/manifest change re-shapes unchanged files' resolution, and import-narrowed
results for unchanged callers must be **invalidated** (the Go-embedding recompute pattern). **Cache key
(rev-3/M11)** = all relevant **manifests** (Cargo.toml workspace+members) + path/`#[path]` overrides +
the **source-file set/existence** (a `mod foo;` resolves differently if `foo.rs` is added/removed) + cfg/
feature inputs — not just edited-source hashes (today `cpg_cache.rs` hashes source only). **Stable IDs
(rev-7/round7-5):** `ScopeId`/`ItemId` derived deterministically from `(crate, module-path, item-name,
ns, ordinal)` **plus source `(file, byte)`, a condition fingerprint, and TU/`occ`** where applicable —
needed to disambiguate the hard cases: cfg-duplicate modules, block locals, anonymous C++ namespaces,
macro-generated/reopened extents, and same-name items in one module. Bump `CACHE_VERSION` for the graph.

## §9 Phasing (architecture whole; build sliced — plan owns PR lines; HELD pending owner)
- **Phase 1 (the F3 win):** scope-graph core (incl. `Pending` bindings, multi-extent scopes,
  `ResolvedSet`, the per-rib + glob-poison resolver, `resolve_path`) + Rust populator for the ✦ set
  (crate graph w/ workspace+editions+dep-names; module+block scopes w/ correct dirs+inline+`#[path]`
  detection; 3 namespaces; **edition-correct anchors**; **concrete `pub use` following**; **visibility
  enforce-or-fall-through**; cfg-formula *representation*) + consumers (module-deps edges + block-scope-
  aware Value-ns narrowing) + whole-repo rebuild/cache. Everything not modeled → strict fall-through.
- **Phase 2:** glob member *expansion*; cfg *evaluation*; dep/external precision; richer visibility.
- **Phase 3:** macros (textual scope); prelude; **full qualified `::`-call *precision* via the graph**
  (NB rev-7/round7-4: Phase-1 *safety* for qualified `::` is already required — graph-resolve-or-disable-
  legacy per §10; Phase 3 adds full precision, not the safety guarantee); the **C++ populator**;
  Python/TS/JS populators (closing `inherited_override`/`from_import_alias`, block scoping, TS namespace).

## §10 Consumers
- **module-deps/repo-map:** named-import/re-export `Binding`s + `Glob` edges, via `resolve`, → file
  edges; `External` → external label; `Unresolved`/`Poisoned` → `UnresolvedModule` (rare).
- **Unqualified narrowing (`resolution.rs`):** for a bare call, `resolve(ResolveQuery{name, Value,
  call_scope, call_loc, …})`; a `Resolved` to a single in-repo **`Item{callable:true}`** → narrow;
  `Local`/non-callable/`ResolvedSet`(>1 file)/`Ambiguous`/`Poisoned`/`Unresolved`/`External` → fall
  through (a local-binding shadow stops narrowing without an edge).
- **Qualified `::` (rev-6/round6-3 — recall-safety):** Phase 1 must NOT leave qualified Rust paths on the
  legacy stem/owner heuristic (`src/resolution.rs:~532`) — that can emit wrong edges the graph would
  reject. Phase 1 either **graph-resolves** qualified paths via `resolve_path` **or falls through**
  (disable the legacy qualified heuristic for Rust where the graph is authoritative). "Untouched" is not
  recall-safe. Full qualified-path precision via the graph is a later phase, but the *fall-through*
  guarantee is Phase 1.

## §11 Round-7 verdict (the go gate): GO for Rust Phase 1
Codex round 7 (gpt-5.5 xhigh): **SOUND-WITH-CONCERNS, no remaining core-shape blocker — "comprehensive
enough to proceed to Rust Phase 1 slicing, provided the Phase-1 plan preserves the spec's fall-through/
poison rules exactly."** All Rust §2 rows + C++ §2.5 representable with **no core change**; round-6 fixes
confirmed correct; engine↔policy seam clean (no Rust-ism in the engine); Go/Java/Python/TS-JS still fit
(`Target::Local` + `visible()` help them). The 5 round-7 findings (folded into rev 7) are non-core:
macro **wildcard** poison (§4.3b), C++ occurrence-visibility-by-policy (§1), `visible()` traversal-ctx
signature (§1), qualified-`::` phasing wording (§9), stronger stable-ID keys (§8).

### Minimal Rust Phase-1 surface (codex §11.5 — the Phase-1 plan must include exactly this, no more)
core graph types · Rust crate/module/block **populator** · pending import/re-export **fixpoint** ·
Rust **anchor policy** (editions; module-boundary stop) · **local/value shadow bindings** (`Target::
Local`) · **visibility enforce-or-fall-through** · **cfg condition carry** · **glob/macro/pending
poison** (macro = wildcard) · **consumer replacement** for unqualified narrowing + **qualified-`::`
safe fall-through** (disable the legacy heuristic where the graph is authoritative). Everything else
(glob expansion, cfg evaluation, macro expansion, full qualified precision, prelude, C++/Py/TS/JS
populators) is later phases — **recall-safe by fall-through**.

### Carried into the Phase-1 plan (the "preserve exactly" rules)
Resolved-or-fall-through (never wrong); per-rib local→glob→parent with **module-boundary stop** for bare
names; **wildcard** macro poison + deferred-glob poison + pending-import poison (never reach an outer
same-name); edge/narrow only on `Resolved` single-in-repo `Item{callable}`; whole-repo rebuild + the
widened cache key. **Status: design-of-record; spec→plan pending the owner's go-for-slicing.**
