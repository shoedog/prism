# Prism Rust Receiver-Typing — Phase 3 Design (rev 6)

**Status:** Slice-1a BUILT on branch `phase3-receiver-typing` (not merged). **rev 6** folds the EXECUTION findings (§0 head). **rev 5** folds the 4th review's 2 leftover-wording fixes (§4.2
recall-safety + §6 gate) — the reviewer's stated condition for PLAN-READY, the **core in-repo algorithm having
been verified SOUND**. **rev 4** scoped Slice 1 to **in-repo recall only** (3rd review); **rev 3** the 2nd
review's §4-algorithm fixes; **rev 2** re-scoped #2+#4 → **#1+#5** (1st review + stratified re-measure, rev-1
buy ~100× overclaimed). See §0.
**Parent:** [`2026-06-17-prism-rust-receiver-typing-design.md`](2026-06-17-prism-rust-receiver-typing-design.md) §9. Phase-2a shipped on `main` (#106/#107/#108) + minors #42/#43.
**Goal:** drain the largest demoted-edge bucket — `r6_single_owner` (untyped receivers resolved by a
single-owner name-guess) — by **typing more *in-repo* receivers** (method chains + field/complex-type identity)
so they resolve to identity (Exact, **recall**) instead of the name-guess NameOnly demote. External receivers
fail closed to today's residue (unchanged); the external-drop *precision* story needs external-return modeling
and is a documented follow-on (§7).

---

## 0. The rev-1 → rev-2 pivot (audit trail)

**rev 6 — EXECUTION findings (2026-06-18; full detail in memory `project_prism_phase3_receiver_typing`).** Slice-1a (#5 in-repo method chains) was BUILT + recall-safe (T1 `8c55301`, T2 `7b585d9`, T3 `a58d1fd`, BLOCKER fix `e48eb31`, T4 `57eade3`, fixtures `9d2fc46`) and CONFIRMED working in production Cargo nav. Gates: Tier-A `--matrix-only` 22/22 rust ok (0 regressions), `--quick` EXIT 0 / 0 confirmed fp, `cargo test` lib+integration green.
- **THE buy is CORPUS-DEPENDENT** (branch-vs-main `call-stats` across 9 rust repos): **clap +79, axum +44** (builder/router-heavy; incl. 27 recovered multi-owner drops), prism +14, **tokio/serde/CLI ~0** (trait-saturated / compiler-tool). prism+tokio (the only 2 Tier-A rust corpora) are the WORST cases → "marginal" was a 2-repo undersample. **LESSON (after the rev-1 #2 overclaim): sample MULTIPLE DIVERSE corpora before claiming OR dismissing buy.**
- **Candidate re-verdict (codex xhigh sizing):** **#2 DEAD** (concrete single-trait-promotable buy 0/0; the big `trait_cha` is `dyn`/generic = **#3**, the real next-big lever). **#1 modest + precision-mostly** (`Bare` → `owner_lookup(name)` directly; in-repo-unmapped 14 prism / 154 tokio; the `Bare` population is dominated by external types → not addressable; re-measure on field-heavy corpora before pursuing 1b).
- **Pre-existing bug FIXED (`a878d80`):** `crate_roots=[]` for non-Cargo / non-conventional layouts (top-level `main.rs`, single ad-hoc `.rs`) gave a "complete" but EMPTY scope graph → ALL scope-graph receiver typing silently inert, and the eval fixtures passed via the R6 name-guess fallback (non-discriminating). Fix = a SINGLE-file `from_convention` fallback (multi-file non-Cargo stays unrooted — mod-wiring inference deferred).
- **T5 (parallelize the post-pass) SKIPPED as YAGNI:** serial chain typing measured ~0% cold-build cost (tokio +1%, clap +0%); the §4.3 constraint is met without it.
- **The residue is untyped-at-source** (the easy cases — constructor-local / typed-param / free-fn return — already resolve) → receiver-typing is at diminishing returns except #5 (builder chains) + #3 (dyn). **Tier-A action:** add builder-heavy rust corpora (clap/axum) so this buy class is exercised.

rev 1 scoped Slice 1 as **#2 (single-trait NameOnly→Exact) + #4 (wrapper `Arc::clone` fix)**. A codex xhigh
spec review returned **FLAWED / NOT PLAN-READY** with findings that a stratified re-measure then confirmed:
- **#2's real universe is ~20 (prism) / ~66 (tokio)**, not the 2,378 / 7,082 rev 1 claimed. The rev-1 figure
  was the *total* NameOnly count, but `combine_kind`'s single-trait demote (what #2 raises) emits a
  receiver-recovery kind — it is **not** `r6_single_owner` (the R6 residue path, `resolution.rs:1016+`) nor
  `trait_cha` (multi). Measured `nameonly_by_recovery_methodkind`: `TypedParam/trait` = 20 / 66.
- **#4's measured universe is 0** wrapper-`clone` demotes on both corpora.
- #2 had real soundness holes too (would raise `dyn Trait`/trait-scope receivers to Exact; trait-in-scope
  check was name-based not identity-based). #4 wasn't recall-safe (Box/Pin `Clone` is conditional; the wrapper
  kind isn't preserved).
- The big bucket is **`r6_single_owner`** (Rust-only: **1,902 prism / 3,487 tokio**) — untyped receivers. The
  lever is **typing more receivers (#1 + #5)**, not raising the tiny #2 confidence bucket.

rev 2 therefore: Slice 1 = **#1 + #5** (this spec, §4); #2/#4 move to follow-on **with the review's fixes
folded** (§7); the precision gate is made confidence-aware (§6).

**rev 3** folds a 2nd xhigh review (which **affirmed the #1+#5 pivot** but found §4's algorithm under-specified
— FLAWED): 3 BLOCKERs + 1 MAJOR + 2 MINORs, all folded —
(1) chain return typed **only via a single Exact intermediate** (`methods_by_scope` is a multi-candidate `Vec`;
§4.1);
(2) `External` routing **reuses the Phase-2a `extension_methods`-then-drop arm verbatim** (not an unconditional
drop, which would delete in-repo extension edges), and intermediate external/generic re-entry **fails closed to
`None`** (§4.2/§4.4);
(3) the recall-safety claim corrected — Slice 1 is **`None→typed` only**; `Bare→identity` is an edge-changing,
Tier-A-gated follow-on (§4.1/§4.2);
(4) the §2 counts labelled **upper bounds** (resolved edges, not typeable sites);
(5) the external gate named **`is_confidently_external`** (not `canonical_external`); (6) **deterministic
sorted apply** for the parallel post-pass (§4.3).

**rev 4** folds the 3rd review (near-SOUND, 5/6 folds confirmed): one self-introduced BLOCKER — §5's
std-iterator-chain "external drop" contradicted §4.1/§4.4's fail-closed-on-external-intermediate rule
(`return_types`/`field_types` omit external types, so an external intermediate/field/local cannot be typed).
Fixed via **option (a)**: Slice 1 is **in-repo recall only** — every external receiver (local, field, or chain)
fails closed to today's residue (unchanged); ALL external-drop precision defers to the external-return-summary
follow-on (§7). Also fixed the title typo (was "rev 2").

**rev 5** folds the 4th review, which **verified the core in-repo algorithm SOUND** and named its PLAN-READY
condition: 2 leftover-wording spots from the rev-4 sweep — §4.2's recall-safety paragraph + §6's gate bullet
still implied an external drop in Slice 1; both now state in-repo-recall-only / no external drops. + 2 plan-time
items the reviewer suggested (an in-repo-then-external fail-closed fixture §5; an AST-shaped chain walker §4.1).
No design change.

## 1. Scope & method

Phase-2a resolves Rust `x.method()` by the receiver's static type, materialized onto
`CallSite.receiver_outcome` at build time (the `rematerialize_rust_receiver_keys` post-pass) and read at R6.
Receivers the typer cannot type fall to the legacy P6-lite residue, which resolves a single in-repo owner of
the method name and demotes it to NameOnly (`r6_single_owner`). Phase 3 Slice 1 extends the typer to type more
of those receivers. Slices are designed together here (the cohesive recursive-typer architecture) and built as
separate plan/implement sub-slices (§3).

## 2. Candidate analysis (measured — stratified `call-stats`)

Measured 2026-06-18 via `prism nav --no-cache call-stats` (now stratified by confidence × ResolutionKind ×
recovery × method-kind; a Phase-2b telemetry add). prism = dogfood (41,412 sites); tokio = `ecb5125a6787`
(32,905). The addressable universe **by lever**:

| Lever | Bucket (NameOnly demote) | prism | tokio |
|---|---|---|---|
| **#1+#5** type-more-receivers | **`r6_single_owner`** (Rust-only; untyped receiver → single-owner name-guess) | **1,902** | **3,487** |
| **#3** dyn/generic dispatch | **`trait_cha`** (multi-candidate) | 159 | 3,320 |
| **#2** single-trait → Exact | `TypedParam/trait` (combine_kind single-trait; UPPER bd, incl dyn) | 20 | 66 |
| **#4** wrapper `clone` | `StdWrapperPeel` + `clone` | 0 | 0 |

**Composition of `r6_single_owner` (sampled).** Dominantly **method chains** — `bufs.iter().fold(..)`,
`[..].into_iter().filter(..).collect()`, builder chains `Builder::new_multi_thread().worker_threads(..)`,
`self.as_mut().service_write(..)`; plus **field/complex receivers** — `self.service_intervals.poll_tick()`,
`parsed.node_text(..)`, `ctx.cpg.node(first).file()`; plus **local vars of std types** — `line_calls.get(..)`,
`s.as_str()`. A large share of chains/locals end in **std** methods (`collect`/`fold`/`iter`/`get`/`as_str`)
that the residue currently mis-demotes to a *coincidentally same-named in-repo* method — i.e. **wrong edges**.

**These counts are UPPER bounds (folds review rev-2 MAJOR 4):** `r6_single_owner_rust` counts resolved *edges*,
not expression-classified addressable sites — it includes generic/inference/macro/genuinely-untypeable residue
plus correct residue edges. The realized Slice-1 buy is the **in-repo-typeable** subset (chains/fields/lets that
resolve to an in-repo type); sizing it precisely needs expression-form stratification (in-repo-chain vs
in-repo-field vs external vs generic vs macro) — a plan-time measurement (extend the call-stats telemetry).

**Implication (Slice 1 = in-repo recall):** #1+#5's Slice-1 buy is **recall** — typing currently-untyped
in-repo receivers (chains with an in-repo single-Exact intermediate; fields/lets resolving to an in-repo type)
so they reach identity (Exact). External receivers (the std-method chains/locals that make up much of the
sample) **fail closed to residue, unchanged** — `return_types`/`field_types` omit external types, so they can't
be typed (or dropped) without external-return modeling (the §7 precision follow-on). Even as recall-only on the
in-repo subset, it is the largest, highest-value bucket — far above #2 (~20/66) and #4 (~0).

## 3. Phasing (decided)

- **Slice 1 (this spec) — #1 + #5: unified recursive receiver typing.** Designed together (one recursive
  resolver); built as separate plan/implement sub-slices (1a #5 chains, 1b #1 identity/external) if size
  warrants — but one architecture so the recursion is well-abstracted.
- **Follow-on (§7), corrected + with review fixes folded:** #3 dyn/generic (`trait_cha`, the next-biggest,
  tokio-heavy) → #2 single-trait→Exact (small; concrete-only + trait-identity) → #4 wrapper (negligible;
  wrapper-kind preserved) → #6 cfg-alias.

## 4. Slice-1 design — unified recursive receiver typing (#1 + #5)

Extend the existing build-time `RustReceiverTyper` so it resolves a receiver **expression** to a `TypeKey` by
walking the expression structure with bounded recursion, then routes the final method through the existing R6
read path. #1 and #5 are two cases of the same walk.

### 4.1 The recursive receiver-expression resolver
Resolve `type_of(expr) -> Option<ReceiverTypeKey>`:
- **binding / param / typed-let** (today): the declared/annotated type → `resolve_type_path_to_type_scope`.
- **field access `recv.f`** (#1, today partial): `type_of(recv)` → `field_types[(recv_scope, f)]`.
- **method-call `recv.m(..)`** (#5, NEW — the chain case): `type_of(recv)`, then **dispatch `m` on that type and
  propagate its return type ONLY if the dispatch is unambiguous** (folds review rev-2 BLOCKER 1). Concretely:
  look up `methods_by_scope[(recv_scope, m)]`, apply the existing `combine_kind` filters (`has_self`, arity);
  propagate a return type IFF the result is a **single Exact candidate** (inherent-single) whose
  `return_types[fid]` is a **single, cfg-consistent** `TypeKey`. Anything weaker — a NameOnly/trait-demoted
  candidate, a multi-candidate set, conflicting cfg-gated returns, or an `External`/`Bare`/`None` intermediate
  receiver — yields `None` (the chain is not typed; the final call falls to today's residue). This is sound:
  `methods_by_scope` is a `Vec<FunctionId>` mixing inherent + trait-impl candidates with possibly-different
  returns, so only a uniquely-Exact intermediate has a provable return type. (Today's `split_call_expr`,
  `resolution_receiver.rs:475`, rejects dotted method receivers outright; this replaces that rejection with the
  bounded dispatch above.)
- **bounded:** reuse `MAX_RECEIVER_TYPE_DEPTH` + the cycle-visit set (already in the typer) — a chain longer
  than the cap, or a cycle, yields `None` (falls to today's residue; recall-safe).
- **memoized per call site:** cache `type_of` results within a chain so a shared prefix is typed once.
- **Plan-time (AST-shaped walk):** the resolver must walk the receiver **AST**, not the normalized `foo(...)`
  receiver text — today's `receiver_expr` normalizer discards intermediate arg counts and `split_call_expr`
  (`:475`) rejects dotted calls outright — so each intermediate method call's arg count is available to the
  arity filter. An implementation constraint for the plan, not a design change.

**#1 also widens the leaf resolver.** Beyond the chain case (#5), #1 extends `resolve_type_path_to_type_scope`'s
coverage so more receivers type: more let-binding forms (e.g. `let x = foo();` via `return_types[foo]`), and
better path resolution for re-exported / module-qualified / nested-generic types (`mod::Type`; `Vec<Foo>` peels
to an element-agnostic owner). **Transition-class contract (folds review rev-2 BLOCKER 3):** the typer today
emits not only `None` but also `ReceiverTypeKey::Bare` for an unresolved *concrete annotation*
(`resolution_receiver.rs:199`), which R6 resolves via legacy `owner_lookup`. Widening resolution could shift
`Bare→InRepo/External`, which **changes** an existing edge — so #1 is **not** purely additive in general.
**Slice 1 is scoped to the `None→typed` class only:** a widening may move a receiver `None`→typed (additive,
recall-safe), but where the receiver is already `Bare` today, Slice 1 leaves its `owner_lookup` result
unchanged. Promoting `Bare→identity` (collision-safety — the original tokio bare-fallback concern) is an
edge-changing, Tier-A-gated **follow-on** (§7), not Slice 1. Each `None→typed` widening is an independently
testable increment (a natural 1b sub-slice).

### 4.2 Routing the resolved type (reuses the Phase-2a R6 arms verbatim)
The resolved final `ReceiverTypeKey` feeds the **existing, unchanged** R6 arms (Phase-2a) — Slice 1 supplies a
typed outcome where today there is none; it does NOT alter the arms (folds review rev-2 BLOCKER 2):
- **`InRepo(scope)`** → `methods_by_scope` + `combine_kind` (Exact for inherent-single, NameOnly for
  trait/wrapper, etc.) — the chain/field now resolves to identity.
- **`External(canon)`** → the existing External arm: **`extension_methods` lookup FIRST, then drop on empty**
  (`resolution.rs:895`; the `impl Ext for String` extension test, `tests/integration/resolution_test.rs:597`,
  must keep passing). Slice 1 **reuses this arm verbatim** and does NOT newly route residue receivers here:
  typing a receiver to `External` needs external return/field types, which `return_types`/`field_types` omit,
  so external chains/fields/locals in the residue **fail closed to `None`** (§4.4) — not to a drop. (Re-routing
  them to a drop-on-empty is the §7 external-summary follow-on.)
- **`Bare(s)`** → unchanged: legacy `owner_lookup` (per §4.1's transition-class contract).
- **`None`** → unchanged: today's P6-lite residue (`r6_single_owner`).

**Recall-safety (corrected — in-repo recall only).** Slice 1 materializes **only proven `InRepo` outcomes from
currently-`None` receiver expressions** (chains with an in-repo single-Exact intermediate; in-repo fields/lets).
External local/field/chain/generic-output receivers **remain `None`** (§4.4 fail-closed). Receivers already
`Bare`/`External`/typed today keep their Phase-2a routing unchanged. **Slice 1 adds no new external
materialization and removes no edges** — it is purely additive (`None`→in-repo-`Exact`), a recall change
measured by §6 (it is *not* gated on any drop).

### 4.3 Build-time & parallelism (hard constraint: CPG build time must not significantly increase)
The recursion runs in the **post-pass** (`rematerialize_rust_receiver_keys`), which already runs *after* the
global indices (`return_types`/`field_types`/`methods_by_scope`/scope graph) are built — so cross-file chain
lookups are available. That post-pass is **currently serial** (`for (caller, sites) in &self.calls`). Slice 1:
- **Parallelizes the post-pass typing phase** with rayon (map-then-apply): `par_iter` over callers → each
  produces its `Vec<(FunctionId, CallSite, Option<ReceiverOutcome>)>` updates reading the **immutable** shared
  indices (the typer borrows `&self`; no `&mut`/interior mutability in the map → safe for `par_iter`) → collect.
- **Deterministic apply (folds review rev-2 MINOR 6):** sort the collected updates by
  `(caller, CallSite::cmp_key)` before the serial apply, so the materialized graph (and cache/witness bytes) is
  identical regardless of `par_iter` completion order.
- **Bounds** recursion depth (`MAX_RECEIVER_TYPE_DEPTH`) + memoizes, so per-site cost stays O(depth).
- Leaves the hot per-file **extraction** path (already `par_iter`, `:472`/`:557`) untouched.
- **Acceptance — measure, don't promise:** a build-time **benchmark** on the largest corpus (Tier-A `--quick`
  wall-time, post-pass timed) must show no significant regression vs today; the goal is that the parallelized
  post-pass offsets the added recursion. If it regresses, the slice is not done.

### 4.4 External recognition (#1, soundness-critical)
"External" must be *proven*, never guessed: a receiver types to `External(canon)` only via the existing
**`is_confidently_external`** gate (the private rule in `resolution_identity.rs` — a confident `std`/`core`/
`alloc` leading segment + the known bare-std-type list; `canonical_external` only *normalizes* the name and is
**not** the confidence gate — folds review rev-2 MINOR 5). An unresolved receiver → `None` (residue), **not**
External — so we never drop an in-repo edge by mis-labelling it external. **Generic re-entry / fail-closed
(folds review rev-2 BLOCKER 2):** an *intermediate* external call whose return could re-enter an in-repo type
(`Option<Foo>::unwrap()` → `Foo`; iterator `Item = Foo`) cannot be proven today — `peel_type` discards generic
args and `field_types`/`return_types` omit external types — so the chain typer **fails closed to `None`** there
(never drops, never mis-types; the call falls to today's residue). Carrying generic/associated-output data to
prove such re-entry is a documented follow-on, not Slice 1.

### 4.5 Cache / identity
Chain typing resolves to the existing `ReceiverTypeKey` and materializes the existing
`CallSite.receiver_outcome` — no new on-`CallSite` field expected. If a new recovery variant (e.g.
`ChainReturn`) is added for telemetry/kind, it is additive; bump `CACHE_VERSION` only if a materialized field
changes. The new `ResolutionKind`/recovery for chains is telemetry, not a behavior gate.

## 5. Testing (TDD)
- **#5 chains:** in-repo builder chain (`Foo::new().bar()` where `bar` is in-repo) → Exact; two-level in-repo
  chain `a.b().c()` (all in-repo) → Exact; depth-cap / cyclic chain → None (residue, unchanged).
- **#1 field/let:** `self.field.m()` where field is an in-repo type → Exact; `let x = make(); x.m()` where
  `make` returns an in-repo type → Exact; an unresolvable complex/generic type → None (residue, unchanged).
- **Recall-safety (external fail-closed — no wrong drop and no wrong type):** a std-iterator chain
  `v.iter().collect()` → **None / residue, unchanged** (the external intermediate `v.iter()` fails closed); an
  **in-repo-then-external chain** `a().b().c()` where `a()` is in-repo but `b()` returns external → typing stops
  before `c` (→ residue, unchanged); a std local whose binding can't resolve to an indexed in-repo type → None /
  residue, unchanged.
- **Capability fixtures** (`eval/fixtures/rust/…`): `chain_in_repo_exact`, `field_chain_exact`,
  `external_chain_unchanged` + `inrepo_then_external_unchanged` (recall-safety negatives).

## 6. Tier-A gate (confidence-aware — folds review finding #5)
- `tier-a --matrix-only` — new capability fixtures `ok`, 0 regressions (authoritative inert+capability gate).
- **Destination-aware recall read:** the set-based M2 (`confidence="all"`) is blind to a NameOnly→Exact change,
  so Slice 1 adds an **Exact-only** precision read + counts `r6_single_owner_rust` reductions and classifies
  them as **in-repo typed resolutions** (the realized recall buy). External/generic residue must stay
  unchanged; Slice 1 introduces **no external drops** (the precision half is the §7 follow-on, gated separately
  when built).
- **tokio call-stats** main→branch: `r6_single_owner_rust` reduction = the realized #1/#5 buy; confirm the
  reductions are **in-repo Exact** edges (recall) with external/generic residue **unchanged** (no lost correct
  edges; Slice 1 adds no external drops). (tokio's Tier-A *oracle* run is invalid — `oracle_error_rate > 0.10`;
  use the deterministic call-stats diff.)
- **Build-time check** (§4.3): no significant CPG-build regression on the largest corpus.
- Paste flip-candidates into the PR; dual-adjudicator, not self-adjudicated.

## 7. Follow-on slices (documented; corrected numbers + review fixes folded — NOT this spec's build)

> **rev 6 supersession (see §0 head):** the rev-5 candidate buys below were re-measured during execution. **#2 is DEAD** (0/0 concrete single-trait-promotable) — drop it. **#1 is modest + precision-mostly** (re-measure on field-heavy corpora before pursuing 1b). **#3 (dyn/generic) is the real next-big lever.** **crate_roots is DONE** (`a878d80`). The numbers in this section are the rev-5 estimates; §0's rev-6 measurements are authoritative.

- **#3 dyn-Trait / generic-bound dispatch** (`trait_cha`: 159 prism / **3,320 tokio** — the next-biggest):
  recover `dyn Trait` / generic-bound receivers; bridge `rust_provider.satisfaction` (string-keyed trait→impls,
  supertrait-propagated) into the identity world; emit a fanout/`TraitCha` kind. High cost; biggest coverage.
- **#2 single-trait NameOnly→Exact** (~20 prism / ~66 tokio — small): fold the review BLOCKERs — raise **only
  concrete-type-scope receivers** (exclude `dyn`/trait-scope, which is #3 territory); store **`trait_scope`
  identity in `MethodFacts`** (not a bare name) + compare by identity (cache bump); confidence-aware gate. Low
  priority given the small buy.
- **#4 wrapper/Deref** (`Arc::clone`; ~0 measured): preserve the **wrapper kind** in `ReceiverOutcome`; drop
  only **unconditionally** wrapper-owned methods (Arc/Rc `clone`), keep Box/Pin (conditional `Clone` can deref
  to `T`). Lowest priority.
- **External-return / generic-output summaries** (the precision half of #1/#5, split out by review round 3):
  model external method/field return types (a narrow std-summary table + carry generic args through
  `peel_type`) so external chains/locals (`v.iter().collect()`, `s.as_str()`) and generic re-entry
  (`Option<Foo>.unwrap().m()`) can be typed → external **drop-on-empty** (precision) or in-repo re-entry
  (recall). Requires extending `return_types`/`field_types` to carry external/generic outputs.
- **#6 full cfg-alias splitting** (~0): replace the T1.4 omit/condition floor with full cfg-conditioned alias
  resolution.

## 8. Non-goals
- No new language (Python/TS receiver typing is a separate initiative — note: much of prism's own
  `r6_single_owner` is Python `eval/` harness code, addressable only by a Python typer, out of scope here).
- No change to Go dispatch (Phase-IP) or the legacy `receiver_type` residue beyond what typing newly covers.
- recv_mode is not consulted by Slice 1 (it's a #2/#3 applicability input).
