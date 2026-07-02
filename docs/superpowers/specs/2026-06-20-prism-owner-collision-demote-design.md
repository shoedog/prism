# Demote-not-drop for same-name owner-key collisions — design

**Status:** design-of-record (approved 2026-06-20), PLAN-READY
**Branch (design + diagnostic):** `precision-multitarget-counter`
**Area:** `src/resolution.rs` (`owner_lookup_in_modules`)
**Companion analysis:** [`docs/archive/analysis/prism/owner-key-identity-analysis-2026-06-20.md`](../../archive/analysis/prism/owner-key-identity-analysis-2026-06-20.md)

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

**Non-goals (this change):**
- *Correcting* the attribution to the single right target (needs scope/import
  resolution that is unavailable where this fires — see §12). **But the demote is
  designed to be recoverable to Exact (1.0) as those capabilities land — this is a
  first-class requirement, specified in §14, not a one-way cap.**
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

**What the demote set actually is (review-corrected).** The condition
`pool.len() > 1 && primary_owners.len() == 1` is broader than cross-type same-name
collisions, and that is intentional. Because `methods` is keyed by
`(bare_owner, method)` with no signature/arity (`call_graph.rs:158`, populated at
`:525`) and trait impls are dual-keyed under the concrete owner (`:532`), a
single-primary-owner multi-candidate pool also includes: (a) **N distinct same-named
types** [the headline collision]; (b) a `#[cfg]`-duplicated method of one type;
(c) same-owner **overloads** (e.g. C++ `f(int)`/`f(double)`); (d) an **inherent
method plus a same-named trait-impl method** on one type; (e) several **trait-impl
methods of one concrete owner**. **All are accepted demotes:** each is genuinely
unpickable by a name-only index at this point (no arg-type matching, no
inherent-over-trait preference, no scope context), so NameOnly is the honest
confidence — and each is recoverable to Exact upstream once a capability supplies
the missing discrimination (arity match, inherent-over-trait preference, scope
resolution — §14). The branch is therefore "non-trait multi-candidate owner-key
ambiguity → NameOnly", not narrowly "same-bare-name collision".

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

No other production code changes. **No `CACHE_VERSION` bump** (currently v15) — but
the rationale is *not* "`ResolutionKind` is unchanged" (review-corrected):
`ResolutionConfidence` **is** serialized on CPG `Call`/`Return` edges
(`cpg_cache.rs:46`, cache v6), so these edges' confidence value does change. A bump
is unnecessary because committed builds invalidate the cache via the resolver
**git-SHA** key (`cpg_cache.rs:343`). A stale pre-fix cache can therefore only
persist in **dirty-vs-dirty** dev iteration, which shares the `-dirty` SHA and so
requires `--no-cache` / an explicit rebuild+clear (`cpg_cache.rs:345`) — see the §11
acceptance commands.

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
- `self`/`this`/`cls` receiver branch (`resolution.rs:784`): `QualifiedOwner` →
  `SelfReceiver` (kind only, `:787`).
- Java/C++ implicit-`this` (`resolution.rs:1117`): `QualifiedOwner` → `ImplicitThis`
  (kind only, `:1120`).
- Receiver-typed `InRepo`-miss fallback (`resolution.rs:887`) and `Bare`
  (`resolution.rs:916`) return the `owner_lookup` result directly (no relabel).

Because the relabels key on `kind == QualifiedOwner` and touch only kind, the
NameOnly confidence is preserved through every path. Result: a recovered-receiver
collision surfaces as e.g. `(TypedParam, NameOnly)` — correct. This is why a single
branch at the chokepoint covers all four measured shapes (`type_path`,
`qualifier_field`, receiver fallback, `Self::`) without editing any caller.

## 6. Recall safety

The pool is returned in full; no candidate is dropped. Each edge is emitted at
`ResolutionConfidence::NameOnly` (navigation score 0.6, `confidence_score` in
`src/navigation/queries.rs`) and still appears in `callers`/`callees`/`ego` output. The only behavioral loss is that these
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
5. **Same-owner multi-candidate demotes (discriminating, per review F1):** one type
   with an inherent `m` AND a same-named trait-impl `m` —
   `struct Foo; impl Foo { fn m(&self){} } trait T { fn m(&self); } impl T for Foo { fn m(&self){} }`
   — caller `Foo::m()`. Pool = 2, both primary owner `Foo` → assert **2 candidates,
   all NameOnly, kind `QualifiedOwner`**. Proves the branch covers the accepted
   non-cross-type ambiguity set (§3 (d)/(e)), not just distinct same-named types.

CLI counter (`tests/cli/call_stats_test.rs`):
6. Update `call_stats_reports_multi_target_exact_same_name_owner_collision`: the
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
  edge; Tier-A defaults to `--confidence all` / `sut.py`, so NameOnly stays in M2
  recall; `--quick` M2 precision/recall unchanged or improved, fp not increased).
- `prism nav **--no-cache** call-stats` on prism/ruff/ripgrep: `multi_target_exact_sites`
  drops (the Exact collisions become NameOnly). **`--no-cache` is required** — per §4
  `ResolutionConfidence` is serialized on CPG edges, so a cache-on run on the same
  dirty worktree would serve stale Exact and mask the drop. Paste before/after
  `multi_target_exact_*` and the `kind_exact`/`kind_nameonly` deltas into the PR.
- No matrix `ok → regression` flips; expected `ok` count preserved.

**Cache caveat for the gate (review-added):** `--matrix-only` forces no-cache
(`eval/tier_a/matrix.py:50`), but the `--quick` SUT path defaults cache-on
(`eval/tier_a/sut.py`) and shares the `-dirty` git-SHA across iterations. Before the
`--quick` gate, clear the prism nav cache (or run the SUT `--no-cache`) so it
re-resolves against the patched binary instead of serving stale serialized
confidence edges; `--allow-stale-sut` only covers the SUT-binary staleness, not the
CPG confidence cache.

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

- **Over-demote of a resolvable single-owner multi-candidate** (cfg-dup, overload,
  or inherent-wins-over-trait — §3 (b)/(c)/(d)/(e)) → NameOnly instead of Exact.
  These were already emitted as ≥2-target Exact (a full-confidence over-claim), so
  the demote is not a precision regression versus today; it is recall-safe (edges
  kept) and the more honest confidence, and each is recoverable to Exact once the
  corresponding upstream discrimination is added (§14). Accepted.
- **An existing test asserts Exact on a multi-same-owner pool** → flips to NameOnly.
  Expected; update the assertion to reflect the corrected behavior (do not weaken
  the fix to preserve a stale expectation).
- **A consumer keyed on Exact-only that legitimately wanted these edges** → none
  identified; Exact-only rings are precisely where full-confidence FPs should not
  appear. The full-graph (all-confidence) traversals are unchanged.

## 14. Precision recovery (forward compatibility) — REQUIRED PROPERTY

A demoted owner-key ambiguity (the full §3 set — same-name-type collisions,
overloads, inherent/trait same-name duplicates) MUST be **recoverable to Exact
(1.0)** as capabilities are added. The demote is a floor for the *current*
unresolvable residue, never a permanent cap. This is a first-class design
requirement, satisfied here by construction:

**Invariant — the terminal-demote rule.** `owner_lookup_in_modules`'s demote is the
**last rung** of the resolution ladder. Every disambiguating step runs strictly
*upstream* of it, and any one of them that succeeds yields Exact and the site never
reaches the demote:
- authoritative scope-graph narrowing (`rust_scope_graph_resolution`,
  `resolution.rs:679-690`): `T::m` → single true scope → **Exact**;
- module-segment narrowing inside `owner_lookup_in_modules` itself (the
  `module_segs` filter, `resolution.rs:644-660`): reduces the pool to 1 → the
  single-candidate `else` → **Exact**;
- receiver-type resolution via `methods_by_scope` (`resolution.rs:868`): resolves
  before the bare-owner fallback.

So as capabilities land — notably the completeness-gate relaxation follow-on (§12),
which makes the scope graph available on large multi-crate repos like ruff — the
set of sites that reach the demote **shrinks**, and each newly-resolvable site is
emitted at Exact by the upstream capability. The demote never caps a site the
system *can* resolve; it only catches what nothing upstream could.

**Recovery is lossless** because the demote changes *confidence only*, never the
candidate set (§5). The full pool is preserved as NameOnly, so a future capability
re-resolving the same site simply produces a narrower, Exact result — no edge was
discarded that recovery would have to reconstruct, and no state is mutated that
would shadow a later Exact.

**Design rule for future capabilities (binding):** a new disambiguating capability
MUST be inserted *upstream* of the terminal demote — resolve to a scope/identity,
then narrow via `methods_by_scope` / the owner index — and MUST NOT be implemented
by re-promoting an already-demoted pool in place. The terminal demote stays the
residue floor; precision recovery comes from upstream rungs reducing the pool to
one, not from un-demoting.

**Observability of recovery.** The demoted-collision residue is visible in
`call-stats` (the NameOnly population across `kind_nameonly`, plus the
`multi_target_exact_shape` stratification). As a capability lands, the
demoted-collision count decreases with a matching increase in Exact singletons —
recovery is directly measurable corpus-by-corpus, so each capability's precision
recovery can be quantified against this baseline.
