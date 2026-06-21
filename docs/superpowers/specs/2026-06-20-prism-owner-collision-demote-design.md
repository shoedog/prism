# Demote-not-drop for same-name owner-key collisions — design

**Status:** design-of-record (approved 2026-06-20), PLAN-READY
**Branch (design + diagnostic):** `precision-multitarget-counter`
**Area:** `src/resolution.rs` (`owner_lookup_in_modules`)
**Companion analysis:** [`docs/owner-key-identity-analysis-2026-06-20.md`](../../owner-key-identity-analysis-2026-06-20.md)

## 1. Problem

prism's method index keys owners by **bare type name**
(`methods: BTreeMap<(owner_name, method), Vec<FunctionId>>`, `call_graph.rs:158`).
When two *distinct* types share a bare name and both define the same method (e.g.
`crates/ruff/tests/cli/main.rs::CliTest::with_file` and
`crates/ty/tests/cli/main.rs::CliTest::with_file`), both register under the single
key `("CliTest","with_file")`. `owner_lookup_in_modules` (`resolution.rs:637-669`)
demotes a multi-candidate pool **only** when the candidates have more than one
*distinct primary owner* (the trait-CHA case). Same-bare-name collisions collapse
`primary_owners` to a single name, so the demote never fires and the pool is
emitted at **Exact (1.0)** — a full-confidence false positive: a qualified `T::m()`
call has exactly one static target, so every extra candidate is wrong.

These are worse than the trait-CHA demotes (which are already NameOnly): being
Exact, they survive Exact-only navigation/slicing rings (NameOnly is filtered
there). Measured with the `multi_target_exact_*` `call-stats` counters
(`src/navigation/queries.rs`):

| corpus | multi-target-Exact sites | est. wrong Exact edges (Σ fanout−1) |
|--------|-------------------------:|-------------------------------------:|
| ruff | 2,769 (1,820 `type_path`; fanout up to 118-way ×81) | ≈17,000 |
| ripgrep | 23 | 23 |
| prism (self) | 11 | ≈16 |

The buy is monorepo-concentrated (ruff: a ~30-crate workspace with per-crate
`Settings`/`Args`/`TestDb`/`CliTest` duplicates). The full owner-key identity
analysis (companion doc) and a codex (gpt-5.5 xhigh) design review concluded that
the recall floor of bare-name keying is load-bearing — no qualified-only scheme
lifts both precision and recall — and that the correct mitigation that helps
everywhere (including corpora with no scope graph) is **demote, don't drop** the
unresolvable same-primary-owner pool.

The §3 pre-gate shadow showed the scope-graph *narrowing* lever buys ≈0 (the
narrowing already exists upstream; the residual FPs are gated by whole-repo
scope-graph completeness, which large repos like ruff fail). Relaxing that
completeness gate is a separate, larger effort and is **out of scope** here (§12).

## 2. Goal / non-goals

**Goal:** eliminate the full-confidence (Exact, 1.0) over-attribution from
same-bare-name owner-key collisions, with zero recall loss, in the one shared
resolution chokepoint, across all caller paths and all languages.

**Non-goals:**
- *Correcting* the attribution to the single right target (needs scope/import
  resolution that is unavailable where this fires — see §12).
- Replacing or re-keying the bare-name `methods` index (its recall floor is
  load-bearing; companion analysis).
- Relaxing the scope-graph completeness gate (follow-on, §12).
- Touching the trait-CHA path (multi distinct owners) — already NameOnly.

## 3. Decision

**Blanket, confidence-only demote.** Any owner-keyed pool with `len > 1` that is
not already trait-CHA-demoted is emitted at **NameOnly** instead of Exact, keeping
**every candidate edge** (recall) and the **`QualifiedOwner` kind** (so caller
relabels are untouched). No file-count gate, no language gate, no new
`ResolutionKind`.

Rationale (full reasoning in the conversation record; summary):
- **File count is not a sound discriminator.** Both real collisions and the only
  legitimate non-collision case (one type with a `#[cfg]`-duplicated method) can be
  same-file or cross-file. A file-gate's failure mode — keeping **Exact on a
  same-file collision** (two same-named types in one file: inline `mod`s, codegen,
  test fixtures) — directly reopens the FP this fix closes.
- **The cfg-variant "loss" is not a real precision loss.** prism does not evaluate
  `cfg`, so it genuinely cannot pick the platform variant; emitting both at Exact
  1.0 is itself an over-claim. NameOnly ("ambiguous; here are the candidates") is
  the more honest confidence.
- **Recall is identical across options** (all keep the edges as NameOnly), so the
  only axis that differs is which edges keep the 1.0 claim; blanket is strictly the
  most conservative about that claim.
- **Symmetry + simplicity:** the multi-distinct-owner case already demotes
  regardless of file layout; blanket makes "a pool we cannot disambiguate →
  NameOnly" uniform, as a single added branch in a hot path with no new allocation.
- **No context is discarded:** if the scope graph could disambiguate by context,
  the upstream scope path (`resolution.rs:679-690`) already resolved or dropped the
  site before `owner_lookup_in_modules` is reached.

## 4. The change

`owner_lookup_in_modules` (`src/resolution.rs`), final return (currently
`resolution.rs:665-669`):

```rust
// BEFORE
Some(if pool.len() > 1 && primary_owners.len() > 1 {
    demoted(pool, ResolutionKind::TraitCha)
} else {
    exact(pool, ResolutionKind::QualifiedOwner)
})

// AFTER
Some(if pool.len() > 1 && primary_owners.len() > 1 {
    // Multiple DISTINCT primary owners — trait-CHA (dyn Trait). Unchanged.
    demoted(pool, ResolutionKind::TraitCha)
} else if pool.len() > 1 {
    // Same-bare-name owner-key collision (or a rare cfg-duplicated method of one
    // type): >1 candidate, one primary owner name, no scope proof reached here.
    // Demote — keep every edge (recall) but not at full confidence. Kind stays
    // QualifiedOwner so caller relabels (R3b/Self::/R6) fire unchanged; only the
    // confidence rides through as NameOnly.
    demoted(pool, ResolutionKind::QualifiedOwner)
} else {
    // Single candidate — Exact, unchanged.
    exact(pool, ResolutionKind::QualifiedOwner)
})
```

No other production code changes. `ResolutionKind` is unchanged → **no
`CACHE_VERSION` bump** (currently v15).

## 5. Confidence-only demote — semantics and why it is safe

`demoted(pool, kind)` sets each `ResolvedCallee.confidence = NameOnly` and
`.kind = kind`. `exact(...)` sets `Exact`. Returning
`demoted(pool, QualifiedOwner)` therefore yields `(QualifiedOwner, NameOnly)`.

Every caller of `owner_lookup` / `owner_lookup_in_modules` that relabels does so by
mutating **`callee.kind` only**, never `confidence`:
- `owner_lookup` (`resolution.rs:613`) relabels promoted aliases → `EmbeddedPromotion` (kind only).
- R3b (`resolution.rs:850`): `QualifiedOwner` → `QualifierOwner`.
- `Self::` (`resolution.rs:715`): `QualifiedOwner` → `SelfReceiver`.
- R6 P6-lite (`resolution.rs:932`): `QualifiedOwner` → `TypedParam`/`ConstructorLocal`.
- Receiver-typed `InRepo`-miss fallback (`resolution.rs:887`) and `Bare`
  (`resolution.rs:916`) return the `owner_lookup` result directly (no relabel).

Because the relabels key on `kind == QualifiedOwner` and touch only kind, the
NameOnly confidence is preserved through every path. Result: a recovered-receiver
collision surfaces as e.g. `(TypedParam, NameOnly)` — correct. This is why a single
branch at the chokepoint covers all four measured shapes (`type_path`,
`qualifier_field`, receiver fallback, `Self::`) without editing any caller.

## 6. Recall safety

The pool is returned in full; no candidate is dropped. Each edge is emitted at
`ResolutionConfidence::NameOnly` (navigation score 0.6, `queries.rs:211`) and still
appears in `callers`/`callees`/`ego` output. The only behavioral loss is that these
edges no longer appear in **Exact-only** traversals (`src/cpg/query.rs`) — which is
the intended removal of the full-confidence FP. NameOnly is counted as a resolved
edge by the Tier-A oracle comparison, so recall metrics are unaffected (verified by
the §11 gate).

## 7. Telemetry / observability

No new `ResolutionKind`. The demoted collisions move out of `kind_exact` into
`kind_nameonly`, distributed under whatever kind each caller path relabels them to
— `qualified_owner` for the bare `::`-split type_path and the receiver-fallback
paths (no relabel), `qualifier_owner` for R3b, `typed_param`/`constructor_local`
for R6, `self_receiver` for `Self::`. So `kind_nameonly[qualified_owner]` is only
the *unrelabeled* subset, not the whole population.

The **shape-stable headline signal** is therefore the `multi_target_exact_*`
counters: a site with ≥2 candidates that are now all NameOnly no longer counts as
multi-target-Exact, so `multi_target_exact_sites` / `_by_kind` / `_fanout` drop
toward zero on the anchors regardless of relabel. That pre-gate counter is the
fix's acceptance signal.

Optional (deferred, not in this change): extend the nav `Collision` warning
(`queries.rs:238`, today only on dropped `MultiOwnerCollision`) to also annotate a
demoted same-name pool. Not required for the fix; revisit if reviewers want the
in-output signal.

## 8. Interactions — checked

- **R4.5 (Go same-package free fns, `ResolutionKind::SamePackage`):** lives in the
  unqualified `None =>` branch and never enters `owner_lookup_in_modules`. No
  re-demote of its promotions.
- **Go methods:** route through `owner_lookup`, so same-named types across packages
  get the same fix — a cross-language win, not Rust-only.
- **Go embedding / interface dual-keying** (`promoted_aliases`, `interface_impls`):
  separate maps from `methods`; the `EmbeddedPromotion` relabel is kind-only, so an
  embedding-promoted method that is also a collision still demotes safely.
- **Scope graph:** if it could disambiguate, the upstream authoritative-scope path
  already handled the site; by the time `owner_lookup_in_modules` sees a multi pool,
  there is nothing left to discriminate.
- **Single-candidate lookups and the trait-CHA path:** untouched.

## 9. Components / file structure

Single-unit change; no new files.
- `src/resolution.rs` — the one added branch (§4). Sole production change.
- `tests/integration/resolution_test.rs` — new resolution-level tests (§10) +
  fix any existing assertion that used a multi-same-owner pool.
- `tests/cli/call_stats_test.rs` — invert the existing collision counter test to
  assert the demoted (NameOnly) outcome.
- `tests/navigation/callees_test.rs` — adjust only if its `qualified_owner`
  assertion uses a multi-same-owner pool.

## 10. Testing (TDD)

Resolution-level (`tests/integration/resolution_test.rs`), each red-first:
1. **Collision demotes:** two files each `struct Foo; impl Foo { fn make() -> Foo { Foo } }`,
   caller `Foo::make()`. Assert resolve returns **2 candidates, all
   `ResolutionConfidence::NameOnly`, kind `QualifiedOwner`**. (Red: Exact today.)
2. **Single owner stays Exact:** one `Foo::make` def + caller `Foo::make()` →
   **1 candidate, Exact, `QualifiedOwner`**. (Guards against over-demote.)
3. **Receiver-typed collision rides through:** a recovered receiver `x: Foo`
   calling `x.make()` with two `Foo::make` defs → **NameOnly, kind `TypedParam`**
   (confirms the confidence survives the R6 kind-relabel).
4. **Trait-CHA unchanged:** existing multi-distinct-owner test still **NameOnly,
   `TraitCha`** (no behavior change on that arm).

CLI counter (`tests/cli/call_stats_test.rs`):
5. Update `call_stats_reports_multi_target_exact_same_name_owner_collision`: the
   `Foo::make` collision fixture now yields `multi_target_exact_sites == 0` and the
   edges as `kind_nameonly[qualified_owner]` — the counter test becomes a
   fix-regression test.

Run (macOS note: the `cli` test binary runs fine via the compiled binary; bare
`cargo test --test cli` may stall at `_dyld_start` — compile with `--no-run` then
run the binary with a module-qualified filter):
```bash
cargo test --lib resolution
cargo test --test integration resolution_test::
cargo test --test cli --no-run && <cli-bin> call_stats_test:: 
cargo fmt --check
```

## 11. Tier-A acceptance gate

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # before commit
cd eval && uv run tier-a --quick --allow-stale-sut         # before review
```
Pass criteria:
- **Zero recall regression** on the Rust anchors (NameOnly counts as a resolved
  edge; `--quick` M2 precision/recall unchanged or improved, fp not increased).
- `prism nav call-stats` on prism/ruff/ripgrep: `multi_target_exact_sites` drops
  (the Exact collisions become NameOnly). Paste before/after `multi_target_exact_*`
  and the `kind_exact`/`kind_nameonly` `qualified_owner` deltas into the PR.
- No matrix `ok → regression` flips; expected `ok` count preserved.

Independent review: codex (gpt-5.5, xhigh, read-only) on the diff — confirm
recall-safety (no dropped edges), relabel preservation (confidence rides through),
and that single-candidate/trait-CHA arms are untouched.

## 12. Out of scope / follow-ons

- **Scope-graph completeness-gate relaxation** (the path that would *correct* the
  attribution on monorepos like ruff): let the scope graph build and be
  authoritative where coverage is locally complete, instead of disabling the whole
  mechanism when any supported file is missing (`has_complete_file_coverage`,
  `repo_loader.rs:215`). Larger, soundness-sensitive substrate effort. **Considered
  as a follow-on**, per owner direction.
- A distinct `ResolutionKind::OwnerCollision` for cleaner telemetry — rejected here
  (would force updating every relabel site; confidence-only keeps the change to one
  branch). Revisit only if telemetry demand grows.
- Nav `Collision` warning on demoted same-name pools (§7) — optional, deferred.

## 13. Risks

- **Over-demote of a genuine cfg-duplicated single-type method** → NameOnly instead
  of Exact. Recall-safe (edge kept), and arguably more honest (prism cannot pick the
  platform). Accepted.
- **An existing test asserts Exact on a multi-same-owner pool** → flips to NameOnly.
  Expected; update the assertion to reflect the corrected behavior (do not weaken
  the fix to preserve a stale expectation).
- **A consumer keyed on Exact-only that legitimately wanted these edges** → none
  identified; Exact-only rings are precisely where full-confidence FPs should not
  appear. The full-graph (all-confidence) traversals are unchanged.
