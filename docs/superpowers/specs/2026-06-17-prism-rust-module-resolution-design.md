# Name Resolution — Scope-Graph Architecture Design Spec (Rust first, C++-general)

> **Status:** design spec, **iterating to SOUND under codex re-review — HOLD before plan/development**
> (owner). Rev 2 adopts a language-neutral **scope graph** as the core, replacing the rev-1
> `module_path → file` map. Two codex reviews (rev-1 plan, rev-1 spec) found the map shape cannot
> represent block-scoped `use`, Rust's 3 namespaces, cfg-conditioned duplicate modules, glob/re-export
> edges, or edition-correct anchoring — and isn't C++-reusable. The scope-graph formalism (Néron–Tin–
> Visser–Wachsmuth, *A Theory of Name Resolution*) represents all of it and generalizes across
> languages. Companion plan (the F3 win) is **deferred** until this spec is SOUND.
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
ScopeId; ItemId; FileId; CfgCond (opaque condition expr, language-neutral)
enum Namespace { Type, Value, Macro, /* C++ adds: Label? Tag? — extensible */ }
enum Vis { Public, Restricted(ScopeId), Private }          // pub(in path) -> Restricted(scope)

struct Scope {                 // a region that can hold bindings + out-edges
    id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>,  // lexical parent edge
    file: FileId, range: ByteRange,        // source range — block-scoping is range containment
    cond: Option<CfgCond>,                 // cfg-gated scope
}
enum ScopeKind { Root, Module, Block, Type, Callable, ExternPrelude }  // lang-neutral; populator maps

struct Binding {               // a name introduced INTO a scope (definition OR import alias)
    scope: ScopeId, name: Ident, ns: Namespace,
    target: Target, vis: Vis, cond: Option<CfgCond>, range: ByteRange,
}                              // multiple Bindings may share (scope,name,ns): cfg-alts / genuine ambiguity
enum Target { Scope(ScopeId), Item(ItemId, Namespace), External(ExternRef) }

struct Edge {                  // how a scope reaches OTHER scopes' bindings
    from: ScopeId, kind: EdgeKind, to: EdgeTo, cond: Option<CfgCond>,
}
enum EdgeKind { Lexical, Import /*use a::b (aliased)*/, Glob /*use a::* */, ReExport /*pub use*/ }
enum EdgeTo { Resolved(ScopeId), Path(RawPath, /*anchor*/ Anchor) }  // Path resolved lazily/fixpoint
```

**Resolution** (`resolve(name, ns, from_scope, at_byte)`): scope-graph name lookup — collect visible
`Binding`s for `(name, ns)` reachable from `from_scope` via a **well-founded edge order** (local →
lexical-parent chain → import/glob/re-export edges), filtered by `Vis`, `at_byte` (block-scope range
containment), and `CfgCond`; **detect ambiguity** (>1 distinct target under compatible conditions);
guard import/re-export cycles; reach a fixpoint over `EdgeTo::Path` edges. Yields
`Resolution { targets: Vec<(Target, CfgCond)>, status: Resolved|Ambiguous|Unresolved }`.

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

### §3.3 Edges (the non-lexical reachability)
`Import`/`Glob`/`ReExport` edges (and `Lexical` for the parent chain) carry an `EdgeTo` that is either a
`Resolved(ScopeId)` or an unresolved `Path` resolved at fixpoint. `Glob` brings *all* visible bindings
of the target scope (a glob *edge*, not a map row — MAJOR-4 fix); `ReExport` makes a brought-in binding
visible from `from` (re-export *edge*, chains followed with a cycle guard — MAJOR-5 fix). Edges carry
`CfgCond`.

### §3.4 Resolution algorithm
`resolve(name, ns, from_scope, at_byte, cfg_ctx)`:
1. **Local** bindings of `from_scope` matching `(name, ns)`, `range ∋ at_byte`, visible, cfg-compatible.
2. Else follow edges in well-founded order — **lexical parent chain**, then **import/re-export** edges
   (resolving their `Path` targets recursively at fixpoint), then **glob** edges (lower priority, as in
   Rust: an explicit `use` shadows a glob). Visibility + cfg filter at each hop; cycles guarded.
3. **Ambiguity:** if ≥2 distinct `Target`s survive under *compatible* conditions → `Ambiguous`
   (consumers fall through — never pick). Distinct cfg-exclusive targets → returned as conditioned
   alternatives, not ambiguity.
4. **Anchors** (`crate`/`self`/`super`/bare/`::`) are pre-resolved by the **populator's `Anchor`**
   into a `from_scope` + starting edge set, per edition (the core just walks edges).
Result: `Resolution { targets: [(Target, CfgCond)], status }`. **External** (dep/prelude) →
`Resolved` to `External` (known, files `[]`). **Unresolved** → empty (fall through).

### §3.5 Conditions (cfg) — never merge exclusive worlds
Scopes/bindings/edges carry an optional `CfgCond`. Default policy: **over-approximate** (include all
cfg variants; attach the condition) so reasoning honesty holds and a `NotReached`/narrow is never a
false proof; mutually-exclusive duplicate mods are **distinct conditioned bindings** (MAJOR-6 fix), so
resolution returns conditioned alternatives, not a merged file set. Full cfg *evaluation* (picking a
target platform) is deferred; representation is present now.

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
4. **Anchor resolver (edition-aware):** `crate`→crate Root; `self`→lexical scope; `super`×n→ancestor;
   2015 `use`-path→crate-root-relative; 2018+ bare leading ident→extern-prelude-or-lexical name lookup;
   `::name`→extern; `extern crate … as` aliases. (Edition from Cargo.toml; default 2015 if absent —
   verify.)

## §5 C++ populator (the reuse proof — design only; built in a later phase)
Namespaces → `Module`-kind scopes (reopened namespace = one logical scope with many `(file, range)`
contributions; anonymous → TU-`Block`-scoped; inline namespace → transparent re-export-like edge);
classes/enums → `Type` scopes; `using namespace N;` → `Glob` edge; `using N::x;` → `Import` edge;
overload sets → multiple `Value` `Binding`s per name (resolution returns the set); ADL → an anchor/
augmentation in the C++ resolver. The core (§3) is unchanged — only a populator + anchor resolver are
added. (This section is the *generality check*, not a Phase-1 deliverable.)

## §6 Form → scope-graph mapping (coverage check)
| Rust form | Scope-graph elements | Phase |
|---|---|---|
| block-local `use` | `Import` edge on a `Block` scope; `range`-gated resolve | 1 ✦ |
| 3 namespaces; multi-ns `use` | `(name, ns)` binding keys; ns-tagged `Target::Item` | 1 ✦ |
| `crate`/`self`/`super`/bare/`::` (editions) | `Anchor` resolver → scope+edge starts | 1 ✦ |
| `mod foo;` correct dir; inline; nested | `Module` scopes w/ declaring-dir rule | 1 ✦ |
| workspace / multi-crate / dep renames | Root scope per crate; extern-name edges | 1 ✦ |
| `foo.rs`+`foo/mod.rs` both | ambiguous `Binding` (error state) | 1 ✦ |
| `pub use` re-export **chain** | `ReExport` edges, fixpoint + cycle guard | 1 ✦ (else strict fall-through) |
| `#[path]` mod | populator file override | 1 ✦ (detect → correct file or fall through) |
| cfg / exclusive-cfg dup mods | `CfgCond` on scopes/bindings; conditioned alternatives | 1 (represent) / 2 (evaluate) |
| glob `use a::*` (+ glob re-export) | `Glob`/`ReExport` edge; member expansion | 1 edge / 2 expand |
| visibility `pub(in …)` filtering | `Vis::Restricted`; filter in resolve | 1 represent / 2 enforce |
| macros (`#[macro_export]`, `macro_rules!`) | Macro-ns bindings; textual-order | 2–3 |
| prelude / `#[no_implicit_prelude]` | `ExternPrelude` scope edge | 2–3 |
| **NOT modeled (out):** proc-macro/`build.rs`-generated items, full macro **expansion**, `include!()` | — degrade to Unresolved→fall through | — |

## §7 Invariants (recall-safety is structural)
- Narrow/edge **only** on `status==Resolved` to in-repo file(s) for the right `(name, Value-ns)` in the
  call's scope+byte; `Ambiguous`/`Unresolved`/`External` → **fall through, never wrong** (MAJOR-5 fix —
  no "resolve to the facade for now").
- Block-scope (range) + namespace + visibility + cfg are all *filters in `resolve`*, not afterthoughts.
- cfg over-approximate (never silently drop/merge). Determinism: sorted/ID-stable structures.

## §8 Incremental & cache
Whole-program rebuild of the scope graph after any incremental merge (a `mod`/`#[path]`/manifest change
re-shapes unchanged files' resolution) — the established Go-embedding recompute pattern; import-narrowed
results for unchanged callers must be invalidated. **Cargo.toml enters the cache key** (today the cache
hashes source files only — `cpg_cache.rs`); bump `CACHE_VERSION` for the serialized graph.

## §9 Phasing (architecture whole; build sliced — plan owns PR lines; HELD pending owner)
- **Phase 1 (the F3 win):** scope-graph core + Rust populator for the ✦ set (crate graph incl.
  workspace+editions+dep-names; module+block scopes w/ correct dirs+inline+`#[path]`-detect; 3
  namespaces; anchors; concrete `pub use` following; ambiguity/cfg *representation*) + consumers
  (module-deps edges + block-scope-aware Value-ns narrowing) + whole-program rebuild/cache. Everything
  not modeled → strict fall-through (never wrong).
- **Phase 2:** glob member expansion; visibility enforcement; cfg evaluation; dep/external precision.
- **Phase 3:** macros; prelude; qualified `::`-call resolution via the graph; the **C++ populator**.

## §10 Consumers
- **module-deps/repo-map:** `Import`/`ReExport`/`Glob` edges + resolved targets → file edges; `External`
  → external label; `Unresolved` → `UnresolvedModule` (rare).
- **Unqualified narrowing (`resolution.rs`):** for a bare call, `resolve(name, Value, call_scope,
  call_byte)`; `Resolved` to one in-repo item/file → narrow; else fall through. Qualified `::` untouched
  (Phase 3 upgrade).

## §11 Open questions for the next re-review
1. Is `Scope/Binding/Edge/Target` + the §3.4 algorithm sufficient to represent **every** §2 row
   (incl. C++ §2.5) with *no core change* — or does any form still force a core shape change?
2. Is the well-founded edge order (lexical → import/re-export → glob, with visibility/cfg/range
   filters + ambiguity) **correct** vs rustc name resolution (esp. glob-vs-explicit shadowing, 2015
   vs 2018 anchoring, re-export cycles)?
3. Does the `Anchor`-in-populator split keep the core truly language-neutral for C++ (§5), or does an
   anchor/ADL/overload concept still need to live in the core?
4. Phase-1 boundary: is any ✦ row still under-modeled such that a *common* path resolves **wrong**
   (not merely unresolved)? Specifically re-export following, `#[path]`, and edition anchoring.
5. Cfg representation: is `Option<CfgCond>` on scopes/bindings/edges enough, or is a richer
   condition lattice needed to avoid merging exclusive worlds while staying recall-safe?
