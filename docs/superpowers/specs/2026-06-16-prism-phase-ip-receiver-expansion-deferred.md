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
