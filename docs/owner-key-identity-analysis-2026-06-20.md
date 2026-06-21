# Owner-Key Identity Analysis — bare type name vs. qualified identity (2026-06-20)

Motivation: the bare-name owner key (`methods: BTreeMap<(bare_type, method), …>`,
`call_graph.rs:158`) has repeatedly produced a precision-FP class — two distinct
same-named types both owning the same method conflate under one key and, because
`primary_owners` collapses to a single name, the TraitCha demote never fires, so
BOTH resolve at **Exact (1.0)** (`resolution.rs` `owner_lookup_in_modules:637`).
This documents (1) the realized magnitude, measured with a new `call-stats`
counter, and (2) an independent codex (gpt-5.5 xhigh) design analysis of whether
to replace bare-name keying.

## 1. Realized magnitude — `multi_target_exact_*` counter

`prism nav call-stats` now emits `multi_target_exact_sites` (call sites resolving
to >1 Exact callee), `multi_target_exact_fanout` (fanout→site histogram), and
`multi_target_exact_by_kind`. NameOnly/TraitCha fanout is excluded — that bucket
is already demoted, not full-confidence. Measured on the 3 Rust anchors
(`nav --no-cache call-stats`):

| corpus | total call sites | multi-target-Exact sites | dominant kind | fanout tail | est. wrong Exact edges (Σ fanout−1) |
|--------|------------------|--------------------------|---------------|-------------|--------------------------------------|
| **ruff** | 170,863 | **2,769** (1.6%) | qualified_owner 1,820 (66%); typed_param 337, self_receiver 273, local_def 220, constructor_local 109 | **118-way ×81**, 38×14, 36×24, 14×128, 12×48, …, 2×1842 | **≈17,000** |
| ripgrep | 12,122 | 23 | qualified_owner 18, self_receiver 5 | all 2-way | 23 |
| prism (self) | 43,312 | 11 | typed_param 7, import_qualified 2, local_def 1, qualified_owner 1 | 8×2, 1×3, 2×4 | ≈16 |

**Verdict on magnitude: real but heavily corpus-concentrated.** ruff (a ~30-crate
workspace with per-crate `Settings`/`Args`/`TestDb`/`CliTest` duplicates) carries
essentially the entire bucket — ~17k full-confidence false Exact edges, dominated
by `qualified_owner` (the `T::m()` same-bare-name collision) with a brutal tail
(a `Settings`-class method fanning **118-way** at 81 sites). ripgrep and prism
(few-crate / single-crate) are negligible. This mirrors the Phase-3 lesson: the
buy is monorepo-shaped. The decision hinges on whether target review workloads are
large multi-crate workspaces.

Each multi-target-Exact site is ≥(fanout−1) false Exact edges by construction: a
Rust `T::m()` qualified call has exactly one static target. These are *worse* than
the trait-CHA bucket because they are emitted at full confidence (1.0), so they
survive Exact-only navigation/slicing rings (NameOnly is filtered there).

## 2. Codex design analysis (gpt-5.5, xhigh, read-only)

> Verbatim report below. Headline: **no fully-qualified-only owner-key scheme
> dominates bare-name + scope-narrowing on BOTH precision and recall** — any scheme
> that *requires* resolving the owner path trades away the recall floor that is
> load-bearing in a build-system-agnostic tree-sitter tool. Recommendation: keep
> bare-name as the recall floor, add a Rust scope-identity **narrowing** pass for
> type-qualified owner calls, **fail-open** to the bare ladder, and **demote (not
> drop)** unresolved same-primary-owner collisions.

### Option space (codex)

| Option | Key | Precision | Recall | Cost |
|---|---|---|---|---|
| A. Current bare name | `(bare_type, method)` | Poor for same-bare owners (up to 29+ Exact). | Highest — works with no import/path/type resolution. | Zero. |
| **B. Bare + Rust scope extension** | bare primary; `qualifier→ScopeId`, narrow pool to `methods_by_scope[(scope,method)]`; fail open. | High when scope resolves; wrong same-bare siblings dropped from the pool. | = bare (fallback/demote, not drop). | **Low-medium; identity index exists.** |
| C. File-qualified | `(file, bare_type, method)` | Fixes cross-file dups; misses same-file/module dups. | Loses edges when file resolution fails (reexports, inline mods, C++ headers). | Medium; cache shape changes. |
| D. Module/crate-path | `(crate::mod::Type, method)` | Better than file. | Tied to path-resolution coverage; good Rust, weak elsewhere. | High. |
| E. Scope/Target identity primary | `(ScopeId/Target, method)` | Best where available. | Bad if *required*: non-Rust + unresolved Rust lose bare edges. | Medium Rust / high globally; ScopeId not yet stable public identity. |
| F. Policy-driven dual-identity lattice | `Resolved(scope/target/canon)` + `UnresolvedBare` fallback | = E where resolved, = A where not. | Best practical shape *if* fail-open mandatory. | Medium; generalizes B. |

### Recommendation (codex)

Adopt **Option B now, as the first Rust case of Option F**. Keep bare-name
`methods` as the recall floor; add a Rust scope-identity narrowing pass for
type-qualified owner calls.

- **Hook:** `resolve_call_site_full` `Some(q)` branch, before R3 imported-module
  and R3b bare owner lookup, after self/receiver handling.
- **Syntax-aware:** `CallSite.qualifier` currently can't distinguish `T::m` (type
  qualifier) from `x.m` (value receiver) — both use the generic qualifier field.
  Add a syntax flag during extraction; do **not** guess from capitalization, and do
  **not** apply narrowing to materialized value-receiver calls.
- **Policy (recall-safety paramount):**
  1. Scope absent/incomplete, qualifier ambiguous, or no `methods_by_scope` match → **fail open** to the existing ladder.
  2. Scope resolves, narrowed set non-empty → emit only the narrowed set (out-of-scope same-bare siblings are provably wrong → drop from this site).
  3. Narrowed singleton → keep **Exact**.
  4. Narrowed multiple → **demote NameOnly** (preserve recall, don't claim 1.0).
  5. No scope proof and bare bucket has multiple same-primary-owner candidates → **demote the pool**, don't drop. This directly removes the full-confidence FP class while keeping evidence visible.
- **Do not replace the bare index.** Bare-name recall is not recoverable globally
  today, especially outside Rust.
- **Cross-language:** owner identity should be **per-language policy**
  (`try_owner_identity(lang, site) → Resolved | Ambiguous | Unresolved | Unsupported`),
  Rust-first; Go=package/type, Java=package/class, JS/TS=module/export, Python/Lua
  retain bare fallback. Uniformly replacing `methods` would make unsupported
  languages worse immediately.
- **Cost:** not greenfield for Rust — `methods_by_scope` / `identity_complete` are
  already populated from resolved impl-type syntax (`call_graph.rs:1640/1691/1718`)
  and consumed by the receiver-typed resolver (`resolution.rs:863`). Reading them on
  the qualified path likely avoids a `CACHE_VERSION` bump (currently v15).
- **Pre-implementation gate (cheapest realized-win measurement):** add a shadow
  counter — "multi-target-Exact sites narrowed to singleton by the scope graph" —
  to size how many of the 2,769 ruff sites the scope graph can actually
  disambiguate (vs. fail-open) **before** changing edge behavior.

### Answer to the maintainer's question

Bare name **does** provide a benefit that a qualified-only scheme cannot make up:
the **recall floor** — it matches candidates even when prism cannot resolve the
receiver's import/canonical path, which is the common case in a tree-sitter,
build-system-agnostic tool with only partial name resolution. No qualified-only
option lifts both precision and recall. The hybrid (bare floor + scope-graph
narrowing, fail-open, demote-not-drop) is the right answer: precision when the
scope graph resolves, bare-name recall when it doesn't.
