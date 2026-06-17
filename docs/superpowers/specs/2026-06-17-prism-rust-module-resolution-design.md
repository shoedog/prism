# Name Resolution — Scope-Graph Architecture Design Spec (Rust first, C++-general)

> **Status:** design spec, **iterating to SOUND under codex re-review — HOLD before plan/development**
> (owner). **Rev 4** (owner: adopt the pluggable resolver + pressure-test all languages): splits the
> core into a **shared language-neutral data model** + a **per-language `ResolutionPolicy`** plug-in
> (round-4 B2/M7 — C++ multi-phase/ADL/overloads can't be a fixed Rust walk over "just data"). Folds the
> other 7 round-4 findings: `SourceLoc{file,byte}` + **multi-region `vis_extents`** (B1), **per-candidate
> results** `Vec<Candidate{target,cond,provenance}>` (M3), 2015 bare-`use` *lexical* fix (M4), glob/
> re-export local-vs-re-export visibility (M5), **item-owns-scope** for `Enum::Variant`/`Trait::Assoc`
> (M6), open `NamespaceId` (M8), terminology cleanup (M9). Adds **§5 — a per-language pressure test**
> (Go/Java/Python/TS-JS/C++): the data model hosts all five with populator+policy plug-ins only; **C++
> alone drove one data-model reservation** (`tu`/`TranslationUnit`). History: Rev 1 module-map (FLAWED);
> Rev 2 adopted the scope graph (Néron–Tin–Visser–Wachsmuth); Rev 3 refined it (round-3 endorsed the
> model). Companion plan (the F3 win) **deferred** until SOUND.
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
type NamespaceId = u16   // rev-4/M8: OPEN, populator-owned (Rust: Type/Value/Macro; C++/TS add their own)
// CfgCond is a FORMULA (rev-3/M10): And/Or/Not/Atom(key,val?) + conservative compatible()/exclusive()
enum CfgCond { True, Atom(key, Option<val>), Not(Box), And(Vec), Or(Vec) }
enum Vis { Public, Restricted(ScopeId), Private }              // pub(in p)->Restricted; +policy predicate
                                                              // for Java's 4 levels / C++ access

struct ScopeExtent { file: FileId, range: Span, cond: Option<CfgCond>, tu: Option<TuId> }  // rev-4: +tu (C++)
struct Scope {
    id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>,    // lexical parent
    extents: Vec<ScopeExtent>,                                // 1+ (reopened ns / macro / merged TS ns)
    owner_item: Option<ItemId>,                               // rev-4/M6: the Item that owns this scope
    cond: Option<CfgCond>,
}
enum ScopeKind { Root, Module, Block, Type, Callable, ExternPrelude, TranslationUnit }  // +TU (C++)

struct Binding {                 // a definition OR an import/re-export alias
    scope: ScopeId, name: Ident, ns: NamespaceId,
    target: BindTarget, vis: Vis, cond: Option<CfgCond>,
    vis_extents: Vec<Span>,      // rev-4/B1: MULTI-region, file-qualified visibility (block scope +
                                 // "visible after def"; a macro may have disjoint visible regions)
}
enum BindTarget { Resolved(Target), Pending(RawPath, Anchor) }   // Pending alias path, resolved at fixpoint
enum Target { Scope(ScopeId), Item { id: ItemId, ns: NamespaceId, owns: Option<ScopeId> }, External(ExternRef) }
                                 // rev-4/M6: an Item may OWN a Scope (enum->variants, struct/trait->assoc)

struct Edge { from: ScopeId, kind: EdgeKind, to: BindTarget, vis: Vis, cond: Option<CfgCond> }
enum EdgeKind { Lexical, Glob }  // named imports/re-exports are Bindings; re-export = Public-vis Binding/Glob
```

**Resolution = data + a `ResolutionPolicy`.** Entry points `resolve_name(name, ns, from, at: SourceLoc,
cfg)` and `resolve_path(segments, ns, from, at, cfg)` (prefix segments resolve in the policy's
"scope-bearing" namespaces — Type/Module/Enum/Trait — to `Scope`/`Item.owns` targets, longest-prefix;
final segment in `ns`). The **policy** supplies: the per-rib **edge order**, glob/visibility rules, the
**candidate-combination** (single `Resolved` vs an overload `ResolvedSet` vs `Ambiguous` conflict),
anchor mapping, and any candidate **injection** (C++ ADL). The shared engine walks the graph; the policy
decides ordering/combination/well-formedness. Result is **per-candidate** (rev-4/M3):

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

**Rust `ResolutionPolicy`:** namespaces `{Type, Value, Macro}` (scope-bearing for `resolve_path`
prefixes: Type/Module + `Item.owns`); per-rib order **local explicit Binding → this-scope `Glob` →
lexical parent**, explicit shadows glob shadows outer, deferred glob ⇒ `Poisoned`; candidate combination
= a single `Resolved` (or `ResolvedSet` only across distinct namespaces); two globs to different items ⇒
`Ambiguous`; **anchors (corrected, rev-4/M4):**
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
- **C++ (the stress case).** Populator: namespaces → `Module`, **reopened → multi-extent**, **anonymous
  → per-TU** (`ScopeExtent.tu` + `TranslationUnit` scope — *added in rev-4 for this*), **inline →
  bidirectional transparent edges**; classes → `Type` + access; `using namespace` → `Glob`; `using
  N::x` → `Binding`; **overloads → multiple `Value` `Binding`s**. Policy (**the deferred slot**):
  multi-phase lookup, **ADL = candidate injection** (argument-type namespaces, call-site dependent),
  **overload-set coherence predicate** → `ResolvedSet`, TU/include-order awareness. **Verdict: the
  rev-4 data model RESERVES for it** (`tu` + `TranslationUnit` + multi-extent + per-candidate +
  item-owns-scope) — **no further data-model change expected**; the lookup *complexity* is isolated in
  the C++ `ResolutionPolicy`, designed when C++ is actually built.

**Summary verdict.** The rev-4 data model + the policy seam host **Rust, Go, Java, Python, TS/JS with
populator+policy plug-ins only — no data-model change**. **C++ is the one that drove a data-model
reservation** (the `tu`/`TranslationUnit` dimension, now in §1); with that reserved, even C++ needs only
a (complex, deferred) policy. This is the comprehensiveness evidence the owner asked for: the foundation
is universal, and per-language *name-resolution* work therefore belongs **inside** this effort (must not
run standalone in parallel — it would be redone).

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
- **module-deps/repo-map:** named-import/re-export `Binding`s + `Glob` edges, via `resolve`, → file
  edges; `External` → external label; `Unresolved`/`Poisoned` → `UnresolvedModule` (rare).
- **Unqualified narrowing (`resolution.rs`):** for a bare call, `resolve_name(name, Value, call_scope,
  call_loc)`; a `Resolved` single in-repo item/file → narrow; `ResolvedSet`(>1 file)/`Ambiguous`/
  `Poisoned`/`Unresolved`/`External` → fall through. Qualified `::` untouched (Phase 3 upgrade).

## §11 Open questions for the next re-review (round 5)
Rev-4 split the core into a **shared data model + a per-language `ResolutionPolicy`** and added
`SourceLoc`, multi-region `vis_extents`, per-candidate `Resolution`, item-owns-scope, open `NamespaceId`,
the `tu`/`TranslationUnit` reservation, and the §5 5-language pressure test. Pressure-test:
1. **Data/policy boundary:** is the `ResolutionPolicy` interface (namespace registry, per-rib edge
   order, candidate combination/coherence, anchor mapping, candidate injection) **sufficient and
   complete** to express each of the 5 languages' lookup — i.e. does the *shared engine/data* now need
   **no** change for any of their policies (esp. C++ multi-phase + ADL + overload coherence)? Or does
   some policy still force an engine/data change?
2. **Pressure-test correctness (§5):** is each language's populator+policy mapping right, and is the
   "no data-model change except C++ `tu`" verdict correct — or does Go/Java/Python/TS-JS *also* force a
   data-model change you can name?
3. **Rust policy correctness (§4):** are the corrected anchors (2015 bare-`use` crate-root vs bare-expr
   **lexical**; 2018 extern-prelude-after-local; module-relative `self`/`super`) now right vs rustc?
4. **Core-fix soundness:** `SourceLoc` + multi-region `vis_extents` (macro disjoint regions), per-
   candidate `cond` (cfg-exclusive duplicates), item-owns-scope (`Enum::Variant`/`Trait::Assoc`),
   `NamespaceId` — each correct + sufficient?
5. **Recall-safety:** any common path that still resolves **wrong** rather than falling through?
6. **Go/no-go:** is the model finally comprehensive enough to proceed to slicing (Rust Phase 1), or is
   another revision needed? If go, what's the minimal Phase-1 policy surface?
