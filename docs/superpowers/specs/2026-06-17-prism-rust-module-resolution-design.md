# Name Resolution — Scope-Graph Architecture Design Spec (Rust first, C++-general)

> **Status:** design spec, **iterating to SOUND under codex re-review — HOLD before plan/development**
> (owner). **Rev 3** refines the scope-graph core after the round-3 review (which **endorsed the model**
> — "the direction is right" — and gave 11 surgical findings, all folded): unresolved `BindTarget::
> Pending` aliases/re-exports, multi-extent `Scope.extents`, a `ResolvedSet` result (≠ `Ambiguous`) for
> overload/multi-namespace, **per-rib resolution with glob-poison**, an explicit `resolve_path`, edge
> `vis`, `CfgCond` as a formula, module-relative `self`/`super` + edition-correct `::`/bare anchors,
> visibility **enforce-or-fall-through in Phase 1**, and a whole-repo cache-key + stable-ID scheme. Also
> folds the owner's Q2: the core generalizes to **Python / TS-JS / Go**, not just C++ (§5). Rev 1 was a
> `module_path→file` map (FLAWED — can't represent block scope, 3 namespaces, cfg-dup mods, glob/
> re-export, editions); Rev 2 adopted the scope graph (Néron–Tin–Visser–Wachsmuth, *A Theory of Name
> Resolution*). Companion plan (the F3 win) **deferred** until SOUND.
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

**The core is a language-neutral scope graph.** Three node/edge families + a resolution algorithm:

```
ScopeId; ItemId; FileId; ByteRange
// CfgCond is a FORMULA, not opaque text (rev-3/M10): And/Or/Not/Atom(key, val?), with
// conservative `compatible(a,b)` / `exclusive(a,b)` (Unknown atoms => conservative: maybe-compatible).
enum CfgCond { True, Atom(key, Option<val>), Not(Box), And(Vec), Or(Vec) }
enum Namespace { Type, Value, Macro }     // Rust's 3; populator-extensible (C++ Tag/Label)
enum Vis { Public, Restricted(ScopeId), Private }          // pub(in path) -> Restricted(scope)

// A logical scope may span MANY source extents (rev-3/B3): reopened C++ namespaces, header
// inclusion, Rust macro textual scope across a module's files.
struct ScopeExtent { file: FileId, range: ByteRange, cond: Option<CfgCond> }
struct Scope {
    id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>,   // lexical parent
    extents: Vec<ScopeExtent>,                               // 1+ (rev-3/B3)
    cond: Option<CfgCond>,
}
enum ScopeKind { Root, Module, Block, Type, Callable, ExternPrelude }  // lang-neutral; populator maps

// A name introduced INTO a scope: a definition OR an import/re-export alias. An alias may be
// UNRESOLVED until the fixpoint (rev-3/B1) -> BindTarget::Pending.
struct Binding {
    scope: ScopeId, name: Ident, ns: Namespace,
    target: BindTarget, vis: Vis, cond: Option<CfgCond>,
    vis_range: ByteRange,   // VISIBILITY range (rev-3/M8): block-scope + "visible after definition"
                            // (later macro_rules! truncates an earlier macro binding's vis_range)
}
enum BindTarget { Resolved(Target), Pending(RawPath, Anchor) }   // Pending = alias path, resolved at fixpoint
enum Target { Scope(ScopeId), Item(ItemId, Namespace), External(ExternRef) }

// Globs are EDGES (named imports/re-exports are Bindings above). Edges carry visibility
// (rev-3/M7): a private `use a::*` vs a `pub use a::*` re-export differ only by `vis`.
struct Edge { from: ScopeId, kind: EdgeKind, to: BindTarget, vis: Vis, cond: Option<CfgCond> }
enum EdgeKind { Lexical, Glob /* use a::* */ }   // ReExport-ness = a Public-vis Binding/Glob
```

**Resolution.** Two entry points (rev-3/M9): `resolve_name(name, ns, from_scope, at_byte, cfg_ctx)`
and `resolve_path(segments, ns, from_scope, at_byte, cfg_ctx)` (prefix segments resolve in the
**Type/Module** namespace to `Scope` targets; the final segment in the requested `ns` — longest-prefix
falls out). Lookup is **per-rib / per-scope, innermost-outward** (rev-3/M4): at each scope, (1) local
explicit `Binding`s for `(name, ns)` whose `vis_range ∋ at_byte`, visible, cfg-compatible; else (2)
**this scope's** glob `Edge`s — and a glob whose expansion is **deferred/unexpanded poisons** the
lookup (→ `Poisoned` → consumers fall through; never skip to a lower-priority outer match); else (3)
the lexical parent. Pending alias targets resolve at a **fixpoint** (cycle-guarded). Result:

```
enum ResStatus {
    Resolved(Target),            // exactly one
    ResolvedSet(Vec<Target>),    // legit multiple: C++ overload set; same name in >1 namespace (rev-3/B2)
    Ambiguous(Vec<Target>),      // genuine conflict (2 globs, different items) -> consumers fall through
    Poisoned,                    // a deferred glob could shadow -> fall through (recall-safe)
    Unresolved,                  // nothing (external/unknown) -> fall through
}
struct Resolution { status: ResStatus, cond: Option<CfgCond> }
```
`ResolvedSet` is distinct from `Ambiguous` so C++ overloads (and Rust multi-namespace) are not mistaken
for conflicts; call-narrowing treats a `ResolvedSet` spanning >1 file as fall-through (recall-safe).

**Seam invariant:** the scope graph + `resolve` are **language-neutral**. Everything Rust-specific —
`crate::`/`self::`/`super::`, edition anchor rules, `mod`-file conventions, the 3 Rust namespaces'
exact semantics — lives in the **Rust populator** (§4), which builds neutral scopes/bindings/edges and
supplies an `Anchor` resolver. C++ (§5) adds a populator (namespaces, classes, `using`, overload sets,
ADL) over the **same** core. No `CrateId`/`crate::`/edition leaks into the core (the prior seam's
defect).

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

### §3.4 Resolution algorithm (per-rib, innermost-outward — rev-3/M4)
`resolve_name(name, ns, from_scope, at_byte, cfg)` walks scopes inner→outer; **at each scope, in order**:
1. **Local explicit `Binding`s** for `(name, ns)` with `vis_range ∋ at_byte`, accessible (`Vis`), and
   `cfg`-compatible — resolving any `Pending` target at the fixpoint (cycle-guarded). A hit here
   **shadows** everything outward (explicit/local beats glob beats outer — the Rust rib rule).
2. Else **this scope's `Glob` edges**: union their public `(name, ns)` bindings. **If any in-scope glob
   is unexpanded/deferred, return `Poisoned`** (a glob *could* introduce `name` and shadow an outer
   match) — consumers fall through; never silently skip to a lower-priority outer target.
3. Else recurse to the **lexical parent**.
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

### §3.6 The seam
The above is the entire language-neutral core. A `Populator` builds it; an `Anchor` resolver (populator-
provided) maps language path-anchors to scope+edge starts. Consumers depend only on `resolve` +
`Resolution`. C++ is a second populator (§5); nothing in §3 mentions Rust.

---

## §4 Rust populator (builds the scope graph)
1. **Crate graph:** Cargo.toml(s) (workspace members, `[lib]`/`[[bin]]` paths, edition, `[dependencies]`
   + `package=` renames) with filename-convention fallback → a `Root` scope per crate; dep extern-names
   → edges to member Roots or `External`.
2. **Module/scope tree:** per crate, worklist from Root: `mod_item` (name, `#[path]`, body=inline,
   `#[cfg]`) → `Module` scopes via the **declaring module's directory** (foo.rs→foo/, foo/mod.rs→foo/,
   inline→its path dir); `foo.rs`+`foo/mod.rs` both present → ambiguous binding. **Block scopes** for
   fn bodies / blocks that introduce `use`/items.
3. **Bindings & edges:** item defs → `Binding`s in the 3 namespaces (a unit struct → Type+Value, etc.);
   `use` → `Import`/`Glob` edges (+ alias `Binding`s); `pub use` → `ReExport` edges; visibility/cfg
   attached.
4. **Anchor resolver (edition-aware — rev-3/M5).** `crate::` → the crate's Root scope. `self::`/
   `super::`×n are **module-relative**: walk from the lexical scope **up to its enclosing `Module`
   scope** (skipping `Block`/`Callable` ribs), then `self`=that module, `super`×n = its n-th ancestor
   module. Leading `::` and bare idents are **edition-split**: **2015** — a `use` path is crate-root-
   relative; `::x` is crate-root-based; a *non-`use*` bare path is module-relative. **2018+** — `::x`
   is **extern-prelude**-based; a bare leading ident is normal name lookup with **local bindings taking
   priority over the extern-prelude** (a local item/`use` named `foo` wins over a crate `foo`; only an
   unshadowed leading ident hits the extern prelude). `extern crate … as` adds an alias binding in the
   crate Root. Edition from Cargo.toml (workspace/package); **default per Cargo's rules (2015 if the
   manifest omits `edition`)** — and if no manifest is found for a root, treat anchoring conservatively
   (prefer fall-through over a wrong edition guess).

## §5 Other-language populators (the reuse proof — design only; later phases)
The same core (§3) is reused by adding a populator + an edition/anchor resolver per language; **no core
change**. This is both the generality check and the strategic payoff (one name-resolution engine for
all of prism's languages — it would *subsume* per-language import extraction and close several open
language gaps).

- **C++.** Namespaces → `Module` scopes with **multi-extent** (reopened across headers = one scope,
  many `ScopeExtent`s; anonymous → TU-`Block` extent; inline namespace → a transparent `Public` `Glob`
  edge to the parent). Classes/enums → `Type` scopes. `using namespace N;` → `Glob` edge; `using N::x;`
  → a `Binding` (Pending). **Overload sets → multiple `Value` `Binding`s per name; lookup returns
  `ResolvedSet`** (rev-3/B2 — not `Ambiguous`). **ADL** → a C++-resolver augmentation that adds the
  argument types' namespace scopes to the candidate set before overload resolution (an anchor/edge
  addition, not a core change). Header inclusion → extents.
- **Python.** Package/module scopes (files + `__init__.py`); `Callable`/`Type`/comprehension `Block`
  scopes; **LEGB** = the lexical-parent walk + an `ExternPrelude`-like builtins scope. `from m import y`
  → a `Binding(Pending m::y)`; `import x [as z]` → a module `Binding`; `from m import *` → a `Glob`
  edge; **class inheritance → an `Import`/`Glob`-like edge to base-class `Type` scopes** (the MRO). This
  closes prism's two open Python capability gaps — `from_import_alias` (alias binding) and
  `inherited_override` (inheritance edge). Mostly one namespace (Value); decorators/`global`/`nonlocal`
  are populator details.
- **TS/JS.** ES-module scopes; **`let`/`const` block scopes vs `var` function scope** (range-gated
  bindings — the same `vis_range` mechanism); `import {x}`/`import * as ns`/default → `Binding`/`Glob`
  edges (baseline import extraction already exists; this upgrades it); class inheritance → base-`Type`
  edges; **TS `namespace` + declaration merging** → multi-extent `Module` scopes (reopened-namespace-
  like); TS type-vs-value namespaces → the `Namespace` enum.
- **Go/Java** likewise (package scopes; Go already partly modeled; Java packages + inheritance edges).

**Coordination consequence:** because the scope graph subsumes per-language *name resolution*, the
open per-language name-resolution backlog items (the Python `from_import_alias`/`inherited_override`
gaps, TS namespace merging, JS/Rust block scoping) belong *inside* this effort and must **not** be
implemented standalone in parallel (they would be redone).

## §6 Form → scope-graph mapping (coverage check)
| Rust form | Scope-graph elements | Phase |
|---|---|---|
| block-local `use` | `Import` edge on a `Block` scope; `range`-gated resolve | 1 ✦ |
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
- cfg formula, over-approximate, never silently drop/merge exclusive worlds. Determinism: stable
  `ScopeId`/`ItemId` derivation (spec'd in §8) + sorted structures.

## §8 Incremental & cache
**Whole-repo/workspace** scope graph (not diff-scoped); whole-program rebuild after any incremental
merge — a `mod`/`#[path]`/manifest change re-shapes unchanged files' resolution, and import-narrowed
results for unchanged callers must be **invalidated** (the Go-embedding recompute pattern). **Cache key
(rev-3/M11)** = all relevant **manifests** (Cargo.toml workspace+members) + path/`#[path]` overrides +
the **source-file set/existence** (a `mod foo;` resolves differently if `foo.rs` is added/removed) + cfg/
feature inputs — not just edited-source hashes (today `cpg_cache.rs` hashes source only). **Stable IDs:**
`ScopeId`/`ItemId` derived deterministically from `(crate, module-path, item-name, ns, ordinal)` (not
insertion order) so the cache + goldens are stable across rebuilds. Bump `CACHE_VERSION` for the graph.

## §9 Phasing (architecture whole; build sliced — plan owns PR lines; HELD pending owner)
- **Phase 1 (the F3 win):** scope-graph core (incl. `Pending` bindings, multi-extent scopes,
  `ResolvedSet`, the per-rib + glob-poison resolver, `resolve_path`) + Rust populator for the ✦ set
  (crate graph w/ workspace+editions+dep-names; module+block scopes w/ correct dirs+inline+`#[path]`
  detection; 3 namespaces; **edition-correct anchors**; **concrete `pub use` following**; **visibility
  enforce-or-fall-through**; cfg-formula *representation*) + consumers (module-deps edges + block-scope-
  aware Value-ns narrowing) + whole-repo rebuild/cache. Everything not modeled → strict fall-through.
- **Phase 2:** glob member *expansion*; cfg *evaluation*; dep/external precision; richer visibility.
- **Phase 3:** macros (textual scope); prelude; qualified `::`-call resolution via the graph; the **C++
  populator**; Python/TS/JS populators (closing `inherited_override`/`from_import_alias`, block scoping,
  TS namespace merging).

## §10 Consumers
- **module-deps/repo-map:** `Import`/`ReExport`/`Glob` edges + resolved targets → file edges; `External`
  → external label; `Unresolved` → `UnresolvedModule` (rare).
- **Unqualified narrowing (`resolution.rs`):** for a bare call, `resolve(name, Value, call_scope,
  call_byte)`; `Resolved` to one in-repo item/file → narrow; else fall through. Qualified `::` untouched
  (Phase 3 upgrade).

## §11 Open questions for the next re-review (round 4)
The rev-3 core added: `BindTarget::Pending` (unresolved alias/re-export), `Scope.extents` (multi-extent),
`ResStatus::{ResolvedSet, Poisoned}`, per-rib + glob-poison resolution, `resolve_path`, edge `vis`,
`CfgCond` as a formula, module-relative `self`/`super` + edition `::`/bare anchors, visibility
enforce-or-fall-through in Phase 1, and the cache-key/stable-ID scheme. Pressure-test:
1. **Representability, again:** does the rev-3 core represent **every** §2 + §5 form (incl. C++ overload
   sets + ADL, reopened/anonymous namespaces, macro textual scope) with *no further core change*? Any
   remaining form that forces a struct/algorithm change?
2. **Resolver correctness, again:** is the **per-rib (local → this-scope glob → outward) + glob-poison +
   `ResolvedSet`-vs-`Ambiguous` + Pending-fixpoint + module-relative `self`/`super` + edition `::`/bare**
   algorithm now correct vs rustc (glob-vs-explicit shadowing, 2015/2018, re-export cycles, multi-glob
   conflict) AND adequate for C++ overload/ADL lookup?
3. **Macro textual scope:** is `vis_range` + multi-extent enough to represent "later `macro_rules!`
   shadows earlier" + cross-file `#[macro_use]`/module-order visibility (deferred to populate, but the
   slots must exist)?
4. **Recall-safety completeness:** with visibility *enforced* (or fall-through) in Phase 1, is there any
   common path that still resolves **wrong** rather than falling through? (re-export following depth,
   `#[path]`, edition anchors, glob poison, cfg non-exclusivity.)
5. **Cache/IDs:** is the §8 cache key (manifests + file-set/existence + cfg inputs) + the stable
   `ScopeId`/`ItemId` derivation complete and incremental-safe?
6. **Is the model now comprehensive enough to proceed to slicing**, or is another revision needed?
   (Explicit go/no-go for moving from spec → plan.)
