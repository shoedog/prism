# Phase-IP receiver-expansion (PR-2) — deferred / dismissed work

> Companion to the plan (`docs/superpowers/plans/2026-06-16-prism-phase-ip-receiver-expansion.md`) and spec
> (`…/specs/2026-06-16-prism-phase-ip-receiver-expansion-design.md`). Items intentionally **not** done in PR-2,
> recorded with priority / why-deferred / production-impact / fix-sketch so a future implementor doesn't have
> to rediscover them. Bridge slices append to the table at the end (do-now / dismiss / defer judgement calls).

## Recorded at plan time (dual-review fold, 2026-06-16)

### 1. Package-level `var r Runner` receivers — DEFERRED
- **Priority:** Low.
- **Why deferred:** The Slice-C `var_spec` recovery walks bindings rooted at the enclosing function
  (`ast.rs` `walk_receiver_bindings`, rooted at the fn node). A package-level (file-scope) `var` is a sibling
  of the function, never inside that subtree, so the spec-§4-listed package-level case is not recovered.
- **Production impact:** ~zero for the PR-2 metric target — caddy's 57 interface-dispatch sites are all
  type-assertion receivers, not package-level vars. The narrowing only drops a rare receiver shape.
- **Fix sketch:** add a file-root declaration scan (iterate top-level `var_declaration`/`var_spec` siblings of
  the function) feeding the same `recover_var` path, with intra-file shadowing semantics + a fixture/test.

### 2. Cross-package concrete `x.(pkg.T).M()` owner keys (D2) — DEFERRED
- **Priority:** Low.
- **Why deferred:** `owner_key` (`resolution.rs:79`) strips `&`/`*`/`::` but **not** Go `pkg.`, so a recovered
  concrete `pkg.T` does not match the bare owner-index keys → no `owner_lookup` hit. (An *interface*
  `pkg.Module` still routes correctly, because `iface_key` does strip `pkg.`.) This matches the pre-existing
  D2 deferred-conditional class in the spec (precise cross-package keys).
- **Production impact:** narrow — only *concrete* cross-package type-assertion / var receivers; the interface
  case (the caddy class) is unaffected.
- **Fix sketch:** a Go-aware bare-name normalizer for the concrete path (+ cross-package collision handling),
  or activate D2 precise cross-package keys if the §8 gate report shows this class matters.

### 3. `--receiver-recovery` runtime CLI flag — OPTIONAL (not deferred work, ergonomics)
- **Priority:** Optional.
- **Why not done:** spec §10's "each form independently revertable to `legacy` via the config" is satisfied by
  the build-time `ReceiverRecoveryConfig` (`build_with_receiver_config`). No runtime toggle is promised.
- **Production impact:** none. A flag would only let an operator disable a form without a rebuild after the
  §8b gate report.
- **Fix sketch:** thread `ReceiverRecoveryConfig` from a `--receiver-recovery {legacy|expanded|...}` CLI flag
  through to `build_with_receiver_config`; add only if the gate report motivates a fast revert.

## Bridge-slice additions (do-now / dismiss / defer)

| Slice | Item | Judgement | Rationale |
|-------|------|-----------|-----------|
| D | `slice_candidate` manifest-only class (§5: `for _, r := range xs { r.M() }` range-element receivers, enumerated even though the classifier recovers nothing) | **defer** | The emitter runs on `&CallGraph` (recovered-class sites: typed_param/constructor_local/type_assertion/var_local), which needs no `ParsedFiles`. `slice_candidate` is a net-new intra-procedural AST scan requiring live `ParsedFiles` (a `CpgContext` emitter) — out of proportion to a manifest-only enumeration class with zero metric impact (caddy's 57 sites are type-assertion). The recovered-class denominator (what the §8b gate report measures) does not depend on it. Fix-sketch: a `CpgContext`-based emitter variant that scans `range` clauses for the slice element type, emitting `receiver_class = "slice_candidate"` rows. Reserved `ReceiverRecovery::SliceElem` already lands (Slice F). |

## Whole-branch review deferrals (owner-approved 2026-06-16)

The whole-branch dual review (codex gpt-5.5 xhigh + claude/Opus xhigh) returned 1 BLOCKER + 4 MAJOR + 3
MINOR. The owner approved fixing the BLOCKER + MAJOR 2/3-direction/4/5 + MINOR 6 in a `review-fixes` commit;
the items below are **deferred** (large or re-adjudication-coupled).

### A. Byte-keyed adjudication store + fingerprint re-anchoring (review MAJOR 3, the larger half) — DEFERRED → Slice E
- **Priority:** Important (for Slice-E metric fidelity).
- **Why deferred:** The manifest keys sites by byte-span (`ManifestSite.byte_key`), but the adjudication store
  (`eval/tier_a/adjudication.py`) is **line-keyed** (`site = "file:line"`) with optional fingerprints. PR-2's
  `gate_report` already joins line-keyed + filters `direction == "prism_only"` (the easy correctness half,
  fixed now). The remaining half — keying the *store* by byte-span and using `fingerprint`/`reanchor_map` for
  drift so same-line multi-dispatch sites (`a.(I).M(); b.(J).N()`) don't collide on one verdict — changes the
  durable adjudication record format, which is exactly the territory of **Slice E's re-adjudication** (it
  re-anchors verdicts anyway). Doing it in PR-2 would churn the store twice.
- **Production impact:** none in PR-2 (the gate report is non-gating + provisional; `corrected_fp` is only
  trusted after Slice E). Matters when Slice E adjudicates same-line multi-dispatch corpora.
- **Fix sketch:** add a `site_byte_key` field to `Adjudication` (back-compat, optional); in `gate_report`
  join by `byte_key` first, fall back to line-key + `adjudication.reanchor_map` fingerprint matching for drift.

### B. Manual-fallback type-assertion test (review MINOR 7) — DEFERRED → deferred doc
- **Priority:** Low.
- **Why deferred:** `collect_calls_manual_with_qualifier_and_spans` (`ast.rs`) always pushes `None` for the
  receiver node, so type-assertion recovery is silently unsupported if a Go file ever routes through the
  manual fallback (query-compile failure). Safe (degrades to no-recovery), but undetectable.
- **Production impact:** negligible — Go uses the tree-sitter query path; the fallback is for grammar-load
  edge cases only.
- **Fix sketch:** a test asserting the Go Calls query is present/used for a type-assertion fixture (so the
  invariant is enforced, not comment-only), or surface the receiver node on the manual path too.

### C. Subset-extraction-path parity test (review MINOR 8) — DEFERRED → deferred doc
- **Priority:** Low.
- **Why deferred:** every classifier test exercises `build` / `build_with_receiver_config`; the second wiring
  site `build_direct_subset_with_receiver_config` has no parity/config test, so the two `ReceiverCtx`
  construction sites can drift.
- **Production impact:** low — the two sites are copies of the same block; a drift would surface as a scoped-
  CPG recovery mismatch.
- **Fix sketch:** one parity test routing a P6-lite + type-assertion fixture through the subset path, or
  factor the shared per-site `ReceiverCtx` construction into one helper used by both call sites.

### Re-review deferrals (owner-approved 2026-06-16)

The whole-branch dual re-review (codex gpt-5.5 xhigh + claude/Opus xhigh) of the §8 in-scope manifest + its
(non-gating, provisional) Python gate report returned 1 BLOCKER + 4 MAJOR + 4 MINOR. The owner approved fixing
BLOCKER 1 + MAJOR 2 (gate-report FP truth-table + denominator) and MAJOR 4 (Go-caller gate) and MINOR 8 (the
`fanout` value test) in a `review-fixes` commit; the items below are **deferred**.

#### MAJOR 3 — `walk_receiver_bindings` is not block-scope-aware (`src/ast.rs`) — DEFERRED
- **Priority:** Important (recall).
- **Why deferred:** an out-of-scope inner `var r Other` declared in a sibling/nested block of a function that
  also has a typed param `r` increments the per-name binding count → `>1` → the conservative shadow-bail
  fires → the typed-param recovery is dropped. The result is a recall MISS (a receiver that *should* recover
  does not), **not** a wrong edge — and it is consistent with the pre-existing non-scope-aware `let` /
  `short_var` arms, which already count declarations across the whole function subtree. Rare shape.
- **Production impact:** recall only; no false edges. Negligible for the PR-2 metric target (caddy's 57 sites
  are type-assertion, not param-shadowed-by-inner-`var`).
- **Fix sketch:** a block-scope-aware binding walk that counts only declarations whose lexical scope encloses
  the call site, applied uniformly across **all** arms (param / `let` / `short_var` / `var`); add an
  out-of-scope-`var` regression test pinning that the typed-param recovery survives.

#### MINOR 5 — emit the routing rung instead of inferring it from `fanout` — DEFERRED → Slice E
- **Priority:** Low (reporting fidelity).
- **Why deferred:** `fanout == 0` conflates two distinct outcomes — "concrete owner-resolved receiver" and
  "interface receiver with zero in-repo implementers (dropped)". The FP rate is **unaffected** (the gate
  report's FP numerator is dispatch-only, `fanout > 0`), but `concrete_sites` overstates owner-resolution on
  corpora whose implementers are all out-of-repo. A Slice-E reporting refinement.
- **Production impact:** none on FP/precision; only the `concrete_sites` breakdown is imprecise on
  out-of-repo-only-impl corpora.
- **Fix sketch:** emit an explicit `routing ∈ {owner_resolved, dispatched, dropped}` per manifest site (read
  from `resolve_call_site_full`) and stratify on it, rather than inferring concrete-vs-dispatch from `fanout`.

#### MINOR 6 — default `Expanded` is not a strict superset of legacy/main — DEFERRED (bundle with MAJOR 3)
- **Priority:** Low.
- **Why deferred:** a typed param shadowed by an *earlier* same-name `var` of a different type now bails
  (more correct — the type is genuinely ambiguous), but that is a silent recall flip versus `main`. The
  `slice_a_legacy_parity_*` tests cannot see it because they compare legacy-vs-legacy, not expanded-vs-main.
- **Production impact:** recall only; the flip is in the *more correct* direction (it suppresses a possibly
  wrong recovery). No false edges.
- **Fix sketch:** document the intentional behavior change and add a default-mode (`Expanded`) regression test
  pinning the shadowed-param-bail; bundle the implementation with the MAJOR 3 block-scope-aware walk.

#### MINOR 7 — `ReceiverRecoveryConfig` has redundant representable states — DEFERRED (bundle with CLI flag)
- **Priority:** Low.
- **Why deferred:** `classifier()` switches only on `mode`, so a state like `{mode: Legacy, type_assertion:
  true}` is representable but the per-form booleans are silently ignored when `mode == Legacy`. The footgun is
  only reachable via the `--receiver-recovery` CLI flag, which is itself deferred (item 3 above), so there is
  no live path that constructs the contradictory state today.
- **Production impact:** none today (no caller builds the contradictory config); a latent trap for a future
  CLI-flag implementor.
- **Fix sketch:** derive "legacy" from *all per-form booleans false* and drop the `ReceiverRecoveryMode` enum
  (one source of truth), bundled with the deferred `--receiver-recovery` CLI flag.

### Focused codex re-review deferrals (owner-approved 2026-06-16) — the §8 adjudication-join-precision cluster

The focused codex re-review (codex gpt-5.5 xhigh) of the gate-report/manifest fixes found **no blocker** (the
positive-`prism_fp` rule, the all-class denominator, and the Go-caller gate are correct), one MAJOR, and two
design minors — all in the **non-gating, provisional §8 report**, all the same "make the adjudication join
precise for Slice-E re-adjudication" theme as item **A** (byte-key + fingerprint). The owner approved deferring
the whole cluster to Slice E.

#### MAJOR — `gate_report` join is not seed-scoped (order-dependent) — DEFERRED → Slice E (with item A)
- **Priority:** Important (for Slice-E metric fidelity).
- **Why deferred:** `gate_report` builds `verdict_by_site = {r.site: r.verdict …}` keyed by `file:line` only, but
  the adjudication store is **seed-scoped + per-edge** (`(seed_def, site)`). A manifest site (structural — it has
  no seed) adjudicated under multiple seeds/edges keeps the last JSONL record → JSONL-order-dependent. This is
  the **same adjudication-join-precision cluster as item A**: the manifest is per-call-site while adjudications
  are per-(seed, edge), so reconciling them is fundamentally Slice-E re-adjudication work. The report is
  non-gating + provisional and **all PR-2 sites are `pending`** (no oracle run), so it does not bite in PR-2.
- **Production impact:** none in PR-2 (provisional, all pending). Order-dependence would only affect Slice-E
  numbers, and only for a site adjudicated under multiple seeds with conflicting verdicts.
- **Fix sketch (with item A):** carry `seed_def` + the byte-span key into the §8 report model so the join is
  `(seed_def, byte_key)`-precise, with a deterministic per-site aggregation that surfaces conflicts; fold into
  the Slice-E re-adjudication. Two related design minors fold in here: (i) **share** an adjudication
  verdict-classification helper with `adjudication.py` instead of duplicating the truth table in `gate_report`
  (and fail closed on unknown verdicts); (ii) **centralize** the Go interface-dispatch eligibility predicate
  (resolver + manifest) instead of copying `Language::from_path`, or carry parsed language on `CallSite`.

(Two focused-re-review MINORs were FIXED in `review-fixes-3`, not deferred: the `prism_only_keys` test now uses
distinct fanouts and asserts the denominator-wide `fanout_width`; the design-spec §8b FP rule was updated to the
as-built positive-`prism_fp`-selection wording.)

### Slice-E κ-session finding (2026-06-16) — the single confirmed FP

#### PRECISION — arity-disambiguate same-named interface methods — ✅ DONE (2026-06-16)
- **✅ RESOLVED** by the arity-disambiguation work (plan `docs/superpowers/plans/2026-06-16-prism-arity-disambiguation.md`; codex-reviewed; on branch `phase-ip-arity-disambiguation`). prism now captures method param-arity (`CallGraph.method_arity`) + call arg-count/spread (`CallSite`) and arity-filters interface-dispatch candidates via a shared `arity_filter` helper at **both** the resolution mint **and** `interface_dispatch_manifest`. Caddy `dispatch_precision 0.9994 → 1.0`, `over_approx 1 → 0`, the one FP dropped, **zero recall loss** (manifest delta: only the FP site changed). The cross-language generalization (`owner_lookup:486` same-owner overloads — C++/Java/TS/Python/Rust) remains a corpus-gated backlog item (see the arity plan's "Backlog" + `docs/eval/tier-a/corpus-expansion-backlog.md`).
- **Priority:** Important (was the *only* false edge the whole-corpus caddy dispatch audit found; `dispatch_precision
  = 0.9994`, κ = 1.000, 1 `prism_fp` of 63 dispatch sites).
- **The finding:** at `modules/caddyhttp/headers/headers_test.go:366` the source is the **3-argument**
  `handler.ServeHTTP(rr, req, next)` — the `caddyhttp.MiddlewareHandler` signature. prism's constructor-local
  receiver recovery minted the **2-argument** `HandlerFunc` (the `Handler` satisfier) for it. Both interfaces
  declare a method *named* `ServeHTTP`; prism keys methods by **name only** and does not check the call's
  arity/signature against the candidate, so it conflated the two. Recorded in `eval/adjudications.jsonl`
  (`measurement=interface_dispatch`, fingerprint `498353d980a73060`).
- **Why deferred (not a PR-2 / Slice-E correctness blocker):** test-only site; a single FP against an otherwise
  sound dispatch set; and it is a **name-vs-signature** gap in the resolver's method index, *not* a
  receiver-recovery gap (the receiver class was recovered correctly — `constructor_local`). Fixing it touches the
  per-`(owner, name)` method index / `interface_impls` minting (R1–R7 ladder, `src/resolution.rs`), a different
  surface than the Slice-A–F receiver work.
- **Fix sketch:** when minting `interface_impls` edges for a dispatch site, filter candidate methods whose
  parameter arity (and, where cheaply available, parameter/return shape) does not match the call site — so a
  3-arg `ServeHTTP` site mints only `MiddlewareHandler`, a 2-arg site only `Handler`/`HandlerFunc`. Guard against
  variadic/`...`-spread and Go's implicit-arg cases. Add a fixture mirroring the caddy `Handler` (2-arg) vs
  `MiddlewareHandler` (3-arg) `ServeHTTP` split. The dispatch oracle is the regression check: re-run it post-fix
  and confirm the over_approx site flips to sound (`dispatch_precision` rises to 1.0 on caddy).
