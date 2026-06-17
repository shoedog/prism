# Rust Module Resolution — Comprehensive Architecture Design Spec

> **Status:** design spec (architecture/schematic). Companion implementation plan (slicing):
> `docs/superpowers/plans/2026-06-16-prism-rust-use-imports.md`. Supersedes that plan's rev-2/3
> "common-conventions-only" architecture per owner direction: the *architecture* must represent the
> **full** Rust module system up front (so phasing the implementation never forces a re-architecture);
> *slicing* which phases ship when is the plan's job.
>
> **Why comprehensive-up-front:** prism has been bitten by promising approximations (heuristics; the
> rev-2 naive module map — codex found 2 BLOCKERs + 4 MAJORs in it) that missed the precision/recall
> bar and required rewrites (original effort + rewrite = double waste). This spec designs the data
> model + seam to represent *all* of Rust's module resolution; the implementation populates/consumes
> progressively, but never needs to change shape.

---

## §1 Goal, consumers, and the language-agnostic seam

**Goal:** a sound, complete-by-design model of "what does this Rust path/`use` refer to, and in which
file(s)" — driving (a) `nav module-deps`/`repo-map` precise import edges and (b) import-aware call
narrowing (the F3 fan-out fix), for the #1/dogfood language.

**The seam (shared with C++ `using`/namespaces).** Resolution splits into a **language-specific
populator/anchor-resolver** and a **language-agnostic resolution core**:

```
// language-agnostic. Keys are ABSOLUTE, crate-qualified module paths (anchor resolution —
// crate::/self::/super::/edition rules — is the populator's job). A module maps to a SET of
// files (Rust: 1 + inline submodules elsewhere; C++: a namespace spans many headers).
struct ModuleGraph {
    modules: Map<ModuleKey, ModuleNode>,   // ModuleKey = (CrateId, Vec<Ident>)
    crates:  Map<CrateId, CrateNode>,
}
struct ModuleNode {
    files: Vec<FileId>,                    // defining file(s) (Vec for C++/inline generality)
    defines: Map<Ident, ItemKind>,        // fns/types/traits/consts/submods defined here
    reexports: Map<Ident, AbsPath>,       // `pub use` targets (re-export edges)
    parent: Option<ModuleKey>,
}
trait ModuleResolver {
    // resolve an already-anchor-normalized absolute path to its module + (optional) item + files,
    // following re-export edges; glob => all public items of the terminal module.
    fn resolve_absolute(&self, path: &AbsPath) -> Resolution;
}
struct Resolution { module: Option<ModuleKey>, item: Option<Ident>, files: Vec<FileId>,
                    external: bool, via_reexport: Vec<ModuleKey> }
```

**Invariant (the codex-BLOCKER fix, generalized):** anything Rust-specific — `crate::`/`self::`/`super::`
anchors, lexical-module scope, the `foo.rs`-vs-`foo/mod.rs` directory rule, `#[path]`, editions — lives
in the **Rust populator**, never in `ModuleGraph`/`ModuleResolver`. The core sees only absolute
crate-qualified paths and multi-valued module→files. C++ reuses the core by populating it from
namespace declarations and resolving `using`/`using namespace`/ADL anchors in a **C++ populator**.

---

## §2 The complete Rust module-resolution problem space

Enumerated so the architecture is checked against *all* of it (✦ = correctness-critical for common
code; the rest is the long tail the architecture must still represent).

### §2.1 Crate graph
- ✦ Crate roots: `src/lib.rs` (lib), `src/main.rs` (bin). Multiple bins: `src/bin/*.rs`,
  `src/bin/<name>/main.rs`. Also `tests/*.rs`, `benches/*.rs`, `examples/*.rs` — each is its own crate
  with its own `crate::` root.
- ✦ **Workspaces:** `[workspace] members = [...]`; each member is a separate crate. Cross-member
  `use other_member::…` resolves to that member (in-repo).
- Cargo.toml-driven overrides: `[lib] path`, `[[bin]] name/path`, `[[test]]/[[bench]]/[[example]]`,
  `autobins`/`autotests` toggles.
- ✦ **Dependencies + renames:** `[dependencies] foo = ...`, `bar = { package = "foo" }` (the extern
  name `bar` ≠ package `foo`), `[dev-dependencies]`, `[build-dependencies]`. In-workspace deps resolve
  to a member; out-of-workspace deps are **external** (known-but-not-in-repo, NOT a phantom).
- **Editions** (`edition = "2015|2018|2021|2024"`): affect path anchoring (2015 paths are crate-root-
  relative by default + `extern crate`; 2018+ uniform paths + bare-first-segment = extern crate),
  macro import rules, and `dyn`/`async` keywords (orthogonal).
- `extern crate foo;` (2015, and still legal): introduces `foo` into scope; `extern crate foo as bar;`.

### §2.2 Module tree
- ✦ `mod foo;` → `<child_dir>/foo.rs` **or** `<child_dir>/foo/mod.rs`, where `<child_dir>` is the
  **declaring module's directory** (BLOCKER-2 fix): crate root → crate-src dir; a module in `foo.rs`
  → `foo/`; a module in `foo/mod.rs` → `foo/`; an inline module `m` → `<containing module dir>/m/`.
- ✦ Inline `mod foo { … }`: submodule in the **same file**; arbitrarily nested; **creates a path
  segment AND a directory segment** for its own `mod` children.
- ✦ `#[path = "rel/or/abs.rs"] mod foo;` and `#[path] mod foo { }` (path attr on inline changes the
  child base dir). Overrides convention.
- **cfg-gated** `#[cfg(...)] mod foo;` / `mod foo { #![cfg(...)] }`: for static analysis the policy is
  **include all** (over-approximate; record the cfg as a condition) so a `NotReached` is never a false
  proof — never silently drop a cfg-gated module.
- Visibility (`pub`, `pub(crate)`, `pub(super)`, `pub(in path)`) on `mod` and items — needed for
  re-export/glob correctness and for honest "is this reachable from there".

### §2.3 Paths & `use`
- ✦ Anchors: `crate::` (this crate's root), `self::` (lexical module), `super::`(×n) (ancestor of the
  **lexical** module — BLOCKER-1: lexical, not file), bare `name::` (2018: extern crate `name`, else
  in-scope item; 2015: crate-root or `self`), `::name::` (2018 extern / 2015 crate-root).
- ✦ `use` forms: simple `use a::b`; alias `use a::b as c`; nested group `use a::{b, c::d}`; **`self` in
  group** `use a::{self, b}` (imports the module `a` itself + `b`); glob `use a::*`; `pub use` /
  `pub(in …) use` (**re-export** — adds a binding to the importing module's namespace).
- ✦ **Item-vs-module ambiguity** (MAJOR-4): `use crate::engine` (module) vs `use crate::engine::start`
  (item in module). Resolved by **longest-module-prefix**: descend module segments as far as they name
  modules; the remainder is the item path.
- ✦ **Re-export chains:** `pub use a::b;` then elsewhere `use crate::c::b` where `c` re-exports `b` →
  follow re-export edges to the original definition (with a cycle guard).
- Macro imports: `use foo::my_macro;` (2018), `#[macro_use] extern crate foo;` (2015), `macro_rules!`
  textual scoping + `#[macro_export]` (crate-root visible) — a distinct namespace (macro vs type vs
  value); model as item-kind on bindings.
- Prelude: std/core prelude names resolvable without `use` (external; mark as resolved-external).

### §2.4 Item resolution (beyond file)
- ✦ The terminal `use`/path resolves to an **item** (fn/struct/trait/const/type/module/macro) in a
  module — its `defining_module` + name (+ file). Glob → all public items of the terminal module.
- Namespaces: Rust has 3 (types, values, macros); the same ident can bind in two (e.g. a unit struct
  is both a type and a value). The binding table is keyed by `(Ident, Namespace)` where it matters;
  for call-narrowing the **value** namespace is what counts.

---

## §3 Architecture — layers that represent §2

### §3.1 Crate-graph layer (`CrateNode`)
`CrateNode { id, name, kind, root_file, edition, deps: Map<extern_name, DepTarget> }` where
`DepTarget = InRepoCrate(CrateId) | External(package)`. **Built from** Cargo.toml(s) when present
(workspace members, `[lib]`/`[[bin]]`/… paths, `[dependencies]` + `package=` renames, `edition`),
with **filename-convention fallback** when Cargo.toml is absent/partial (so a loose `src/lib.rs`
without manifest still works). Multiple crate roots ⇒ multiple `CrateId`s; **`ModuleKey` is
`(CrateId, path)`** so `crate::engine` never collides across crates (MAJOR-3 fix).

### §3.2 Module-graph layer (`ModuleNode`)
Built per crate by a worklist from the root: extract `mod` decls per file (`mod_item`: `name`,
optional `body` = inline, optional `#[path]`), resolve each to its file via the **declaring module's
directory** (§2.2), creating `ModuleNode`s keyed by `(CrateId, path)`. Inline modules create nodes
backed by the same file but a **distinct path + child directory**. `#[path]` overrides the child base.
cfg-gated mods are included (flagged). The inverse "file → module" is **not** a function (a file has
many modules via inline) — the architecture instead carries the **lexical module on each `use`/decl
at extraction time** (BLOCKER-1 fix), so `super::`/`self::` resolve against the lexical module, never a
file-level guess.

### §3.3 Binding / use / re-export layer
Per `ModuleNode`: `defines` (items declared lexically in that module), `uses` (each carrying its raw
imported path + alias/glob/visibility + **its lexical `ModuleKey`**), and `reexports` (derived from
`pub use`). Extraction walks the AST tracking the lexical module stack (inline-mod-aware) so every
`use`/item is attributed to the correct `ModuleNode`.

### §3.4 Resolution algorithm
`resolve(raw_path, lexical_module)`:
1. **Anchor-normalize** (Rust populator) → absolute `(CrateId, segs)`: `crate`→(crate,root);
   `self`→lexical; `super`×n→ancestor(lexical,n); bare/`::` per edition (extern-crate via
   `CrateNode.deps`, else in-crate); `extern crate` aliases applied.
2. **Descend** `ModuleGraph` by `segs`, **longest module prefix** wins; at each `pub use` boundary
   follow the **re-export edge** (cycle-guarded); a glob terminal expands to the module's public items.
3. **Yield** `Resolution { module, item, files, external, via_reexport }`. External (dep / std
   prelude) → `external: true, files: []` (known, not phantom). Unresolved (incomplete tree / unknown)
   → empty — consumers **fall through** (recall-safe), never narrow on a guess.

### §3.5 Build, incremental, cache
- Built once per repo at `CallGraph` build time, stored on `CallGraph` (or `CpgContext`).
- **Incremental: whole-program rebuild after merge** (MAJOR-5) — a `mod`/`#[path]`/Cargo.toml change
  in one file alters mappings for *unchanged* files, so the graph is recomputed over all files
  post-merge (the established Go-embedding-promotion pattern: replace-not-merge). Cargo.toml is a build
  input (re-read on change).
- `CACHE_VERSION` bump iff the graph is serialized on `CallGraph` (it should be, to keep warm-cache
  resolution); record the schema in the cache-version history.

### §3.6 Consumers (recall-safe)
- **module-deps/repo-map:** each `use` → `resolve` → resolved file edge(s); external → resolved-external
  label (not `UnresolvedModule`); truly-unresolved → `UnresolvedModule` (rare, long-tail). Replaces the
  Rust call-derived-only contract.
- **Unqualified-call narrowing:** in `resolution.rs`'s `qualifier==None`/no-`::` branch, **after**
  local-def preference: if the bare callee name is a `use`-imported (or glob-imported) value-namespace
  binding, `resolve` it and narrow `functions[name]` to the resolved file(s)/item. **Recall-safe:**
  unresolved/external/ambiguous (>1 file, glob with multiple) → fall through unchanged; never drop.
  Qualified `::` calls (R1/R2/R7) untouched.

---

## §4 Form → handling map (architecture coverage check)

| Rust form | Layer / handling | Phase |
|---|---|---|
| `use a::b;` / `as` / nested / `{self, …}` | §3.3 extraction + §3.4 longest-prefix | 1 |
| `use crate::m;` (module) vs `::m::item` (item) | §3.4 longest-module-prefix | 1 |
| `crate::`/`self::`/`super::` (lexical) | §3.4.1 anchor-normalize w/ lexical module | 1 |
| `mod foo;` → `foo.rs`/`foo/mod.rs` (correct base) | §3.2 declaring-module dir | 1 |
| inline `mod foo {}` (nested) | §3.2 inline node, distinct path+dir | 1 |
| multi-crate / workspace members | §3.1 `(CrateId, path)` keys | 1 |
| Cargo.toml `[lib]`/`[[bin]]`/members/edition | §3.1 manifest parse (conv. fallback) | 1–2 |
| `pub use` re-export **chains** | §3.3 reexports + §3.4.2 edge-follow | 2 |
| glob `use a::*` expansion | §3.4.2 terminal glob → public items | 2 |
| `#[path = "…"]` mod | §3.2 path-attr override | 2 |
| cfg/feature-gated mods | §3.2 include-all + cfg flag | 2 |
| dep renames (`package = `) / external | §3.1 `DepTarget::External` | 2 |
| `extern crate` (2015) / `#[macro_use]` | §3.1 + §3.3 macro namespace | 3 |
| macros (`my_macro!`, `#[macro_export]`) | §3.3 macro-namespace bindings | 3 |
| visibility-aware resolution (`pub(in …)`) | §3.3 visibility on bindings | 3 |
| std/core prelude | §3.4 external-resolved | 3 |
| **NOT modeled** (out of scope) | proc-macro-*generated* mods/items; `build.rs`-generated code; full macro **expansion**; `include!()` | — |

The architecture **represents** every in-scope row with the same `ModuleGraph`/`Resolution` shapes;
later phases populate more (re-export edges, glob expansion, macro namespace, manifest depth) without
changing the core. The "NOT modeled" rows are genuinely out (they require macro/proc-macro expansion or
running build scripts) and degrade to **unresolved → fall through**, never wrong.

---

## §5 Phasing (architecture is whole; implementation is sliced — plan owns PR boundaries)

- **Phase 1 (first PR, the F3 win):** crate graph (convention + Cargo.toml-lite: members, edition,
  `[lib]`/`[[bin]]` paths, dep names) · module graph (correct dirs, inline, lexical scope, `(CrateId,
  path)`) · `use`/`mod` extraction (full path + lexical module) · resolution (anchors + longest-prefix;
  re-export/glob/`#[path]` resolve to the re-exporting/declaring module for now — a real edge, not the
  transitive original) · consumers (module-deps edges + unqualified narrowing) · whole-program
  incremental rebuild + cache bump. **Covers the bulk of real Rust correctly.**
- **Phase 2:** re-export *chain* following · glob member expansion · `#[path]` · cfg-flagging · dep
  renames/external precision · deeper Cargo.toml.
- **Phase 3:** macros (namespace + `#[macro_export]` + `#[macro_use]`/`extern crate`) · visibility-aware
  resolution · prelude · qualified `::`-path call resolution via the graph (an R1/R2/R7 precision
  upgrade).

Each phase is recall-safe by construction: a form a phase hasn't populated resolves to empty/external →
consumers fall through. No phase invalidates the architecture.

## §6 C++ pairing (the seam reused, not re-architected)

C++ `using`/namespaces populate the **same** `ModuleGraph`: a `namespace foo { }` (possibly reopened
across many headers) → a `ModuleNode` with `files: Vec` (many); `using namespace foo;` ≈ a glob;
`using foo::bar;` ≈ an item import; anonymous namespaces + ADL are the C++ populator's anchor rules.
The resolution core (`resolve_absolute`, longest-prefix, re-export/`using`-edge follow, multi-file
modules) is **unchanged** — only a C++ populator + anchor-resolver is added. This is why the value
"amortizes across two languages" and why the seam must carry no Rust-isms (§1 invariant).

## §7 Risks / invariants
- **Recall-safety is structural:** narrowing/edges apply only on a **resolved** in-repo target; every
  unresolved/external/ambiguous path falls through unchanged. The model never *guesses* a file.
- **Lexical scope is load-bearing:** `super::`/`self::`/visibility correctness depends on per-`use`
  lexical module attribution (not file) — the BLOCKER-1 fix is foundational, in Phase 1.
- **Directory base is load-bearing:** `mod` child resolution uses the declaring module's canonical dir
  (BLOCKER-2) — foundational, Phase 1.
- **Whole-program rebuild** on any `mod`/manifest change (MAJOR-5) — no per-file merge of the graph.
- **Over-approximate cfg** (include-all) so reasoning honesty holds; never silently drop a gated module.
- **Determinism:** BTreeMap/sorted everywhere (prism convention) so cache + goldens are stable.

## §8 Open questions for the codex re-review
1. Is the `(CrateId, path)` + multi-file `ModuleNode` model sufficient to represent **every** §2 row,
   or is there a Rust form that doesn't fit (forcing a core change rather than a populator addition)?
2. Is the §1 seam truly C++-reusable, or does any Rust assumption still leak into the core?
3. Phase-1 boundary: is anything in Phase 2/3 actually **correctness-critical for common code** (must
   move to Phase 1), or is the recall-safe fall-through sufficient for the deferred rows?
4. Anchor normalization across editions (2015 vs 2018 bare-path semantics) — is the §3.4.1 rule correct
   and complete?
