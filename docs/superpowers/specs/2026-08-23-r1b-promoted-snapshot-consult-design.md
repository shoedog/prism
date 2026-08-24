# R1(b) — promoted-selector snapshot consult for deferred-drop sites (design v4)

**Date:** 2026-08-23. **Base:** main @ a77cf10 (resolution-identical to d8f992c; #188 was docs/cosmetic). **Roadmap:** row-17 follow-up "R1(b) on-demand promoted routing"; increment 4 of the next-10 handoff. **Owner constraint (standing):** snapshot-verdict CONSULT ONLY — NO new comparator or admission logic. **Size:** S. **Status:** v4 — folds sol r1 (4W+3S), r2 (1W+3S), r3 (1W+1S; sol's cap classification: "residual-bounded and foldable; no new open territory"). Cap disclosure: r3's fixes are folded under the convergence rule (converging, non-repeating findings at the cap ⇒ fold + one scoped confirm), extending the loop by exactly one scoped confirmation of the two r3 folds — not a full round.

## 0-ter. r3 folds
- **r3-W1:** gate 1 gains resolver+manifest+telemetry NEGATIVES for every newly specified fail-closed branch: declaration counts **0 and 2**, **empty `profile_variants`**, and a **singleton `profile_variants` not containing `target`** — each stays `ConcreteReceiverPromotedDeferred`, increments `invariant_drop`/`variant_drop` respectively, and preserves deferred-drop conservation (constructible via deserialized snapshots — call_graph.rs:548-551, go_promoted_snapshot.rs:49-94).
- **r3-S2:** stale v2 wording swept — the historical fold notes below are annotated; the §2/§5 contract text is the single normative reading.

## 0-bis. r2 folds
- **r2-W1 (gate 3 executability):** `direct_lane_audit.py` as-is cannot run this gate (`--sample` is an int, default kinds exclude `embedded_promotion`, `unknown` doesn't block PASS). Gate 3 now REQUIRES a patched harness invocation: kinds = `embedded_promotion` only, the ENTIRE delta processed (`sampled == new_sites`), `unknown == 0`, and every individual target matching its gopls definition; a proven zero-site delta passes as NO-DATA.
- **r2-S2:** no `assert!` — ordinary `len() != 1` guard ⇒ `invariant_drop` telemetry + unchanged drop (a `debug_assert!` may follow the guard, never replace it).
- **r2-S3:** variant guard is exact-coherence: `profile_variants.len() == 1 && profile_variants.contains(&target)`; empty or inconsistent stored evidence ⇒ drop.
- **r2-S4:** gate 2 pins `go_promoted_snapshot_invariant_drop == 0` on all corpora and requires counter conservation: every unchanged drop still counts `go_concrete_receiver_promoted_deferred`.

## 0. What v1 got wrong (sol r1)

- **W1 — `target` is not profile-neutral**: `SelectionShape` excludes FunctionId and picks the first variant (go_promoted_snapshot/selector_resolution.rs:26-59,163-196); with mutually exclusive `b_linux.go`/`b_windows.go` both FunctionIds sit in `profile_variants` and stable-first would mint the wrong profile's target. **Fold (as tightened by r2-S3/r3): exact coherence — mint only when `profile_variants.len() == 1 && profile_variants.contains(&target)`; every other variant state drops** — caller-compatible selection would be new logic; deferred.
- **W2 — v1's receiver-shape admission was NEW logic**: the materialized lane has no call-site value/pointer predicate (installs every candidate, type_providers/go.rs:2517-2531, call_graph.rs:2917-2952; consult is an `EmbeddedPromotion` presence test, resolution.rs:1272-1277), and `recv_ty` strips `*` (resolution.rs:526-557) while Go addressability makes pointer methods legal on value receivers. **Fold: rule REMOVED — no receiver-shape check, exactly like the materialized lane.**
- **W3 — the interface manifest/oracle cannot adjudicate non-interface promoted methods** (denominator predicate queries.rs:685-688; concrete routes keep empty `impls`). **Fold: gate 3 is an exhaustive resolver-delta target audit, not a sampled one.**
- **W4 — kind naming**: materialized promotion mints `ResolutionKind::EmbeddedPromotion` (`embedded_promotion`); `concrete_promoted` is only a manifest route string. Fold: mint `EmbeddedPromotion`, manifest route `concrete_promoted_snapshot`; gates track both.
- **S5**: `ProfileUnique` does not imply identical `promoted_methods` across declarations; the deferred route today cannot reach multi-declaration owners (declaration-kind index rejects them first) — **consult applies an ordinary `declarations.len() != 1` guard ⇒ `invariant_drop` + unchanged drop** (r2-S2/r3: never a panicking assert; §2 is normative).
- **S6**: negative fixtures must first assert the pre-consult route is `ConcretePromotedDeferred` (own-method/field/supply conflicts are rejected earlier and would bypass the consult).
- **S7**: telemetry must be plumbed through `ResolutionTelemetry` → `call_stats` accumulation → emission; the conflict counter is pinned in gate 2.

## 0-quater. v5 amendment (2026-08-23, from the #16-C1 r2 review — sol finding, controller-verified in code)

Two additions to this design's scope, folded BEFORE the implementation's first review wave:

1. **Equal-depth field/method repair (WRONG in shipped slice-4 code, this lane's substrate):** `selector_resolution.rs` treats a same-name field as shadowing only when `field_depth < shallowest_depth` (strictly less, :236), while Go semantics and the static provider reject at shallower-OR-EQUAL (`*fd <= candidate.promoted.depth`, type_providers/go.rs:2578). Input `A.M` + `B struct{ M int }` + `T struct{ A; B }`: Go says `T.M` is ambiguous; the snapshot selects `A.M`. Repair: `<` → `<=` (equal depth ⇒ shadowed/ambiguous outcome), with a red-first equal-depth fixture (snapshot yields no usable promoted row; consult drops). **Because this changes serialized snapshot content, this PR's cache transition becomes CPG 48→49 AND sidecar 17→18** (supersedes §4's CPG-unchanged claim).
2. **Generic/signature guard on the consult mint:** promoted rows carry no `generic` bit or canonical signature (go_promoted_snapshot.rs:83-95 — none tracked), while the materialized promotion path rejects generic/missing-signature methods. The consult therefore joins `promoted.target` back to `go_method_declarations` by unique `FunctionId` and requires `!generic && signature.is_some()`; join-miss or guard-fail ⇒ drop (unchanged), counted. Negatives: generic-receiver promoted method; join-miss.

**Ox parallel-review folds (r-v4, APPROVE + 3 SMELL):** (a) the sidecar version constant is `NAV_CALL_EDGE_CACHE_VERSION` at `src/navigation/call_edge_cache.rs:53` (the earlier `navigation/mod.rs:79-106` citation was stale — bump the right constant); (b) the PATCHED gate-3 audit harness (or its diff against the lane-artifact original) is COMMITTED under `eval/tools/` in the PR — gate 3 must be reproducible from the repo alone; (c) conservation disambiguated: `ProfileConflict` consult drops (step 2) count BOTH `go_promoted_snapshot_conflict_drop` AND `go_concrete_receiver_promoted_deferred` (additive fields; the conservation invariant is `promoted_deferred(main) == promoted_deferred(candidate) + snapshot_hits(candidate)` per corpus).

## 1. Problem (unchanged)

`ConcretePromotedDeferred { owner }` sites take an unconditional `DropReason::ConcreteReceiverPromotedDeferred` (resolution.rs:2349-2358; route minted go_concrete_receiver.rs:532). Slice 4 (#184) shipped `CallGraph.go_promoted_selector_snapshot` — per `GoOwnerIdentity`: `{ verdict: ProfileUnique|ProfileConflict, declarations: Vec<GoPromotedProfileSnapshot> }` with `promoted_methods { method, target, profile_variants, target_owner, depth, field_shadowed, value_method_set }` + `ambiguous_promoted_methods`, computed under the five profile-safety axes. Serialized (CPG 48), unconsumed by routing. mainD telemetry: snapshot owners caddy 367 / prometheus 1190 / etcd 974 / hugo 972; `go_concrete_receiver_promoted_deferred` counts affectable sites.

## 2. Design — the consult (v4, normative)

At resolution.rs:2349, before dropping:

1. `snapshot = cg.go_promoted_selector_snapshot().owners.get(&owner)`; absent → drop (unchanged).
2. `verdict == ProfileConflict` → drop + `go_promoted_snapshot_conflict_drop` telemetry.
3. `ProfileUnique`: guard `declarations.len() != 1` ⇒ `go_promoted_snapshot_invariant_drop` + unchanged drop (r2-S2 — a guard, never a panicking assert; never expected to fire, pinned 0 in gate 2). Look up `method` in that declaration's `promoted_methods`:
   - absent, `ambiguous_promoted_methods` member, `field_shadowed`, or **NOT (`profile_variants.len() == 1 && profile_variants.contains(&target)`)** (W1 + r2-S3: exact coherence — empty/inconsistent stored evidence also drops) → drop (unchanged, still counted in `go_concrete_receiver_promoted_deferred` — r2-S4 conservation).
   - otherwise → mint Exact to `promoted.target` with `ResolutionKind::EmbeddedPromotion` (W4) + telemetry `go_promoted_snapshot_hits` (site) — **no receiver-shape check (W2), no other predicate: every judgment is a stored snapshot flag.**
4. Manifest mirror (queries.rs:713 arm): identical consult via one shared fn; route string `concrete_promoted_snapshot` on hits (W4).
5. Telemetry plumbing end-to-end (S7): `ResolutionTelemetry` fields `go_promoted_snapshot_{hits,conflict_drop,variant_drop,invariant_drop}` → queries.rs accumulation → `call_stats` emission.

## 3. Non-goals

Unchanged from v1 (no R1(a)/(c)/(d)/(e)/R2/R3 change, no snapshot format change, no #16 interaction). Additionally: no caller-profile-aware variant selection (W1's alternative) — deferred until measured need.

## 4. Cache

No new serialized state; CPG stays 48 (snapshot already serialized, cpg_cache.rs:193-200); resolved targets/kinds change → **nav sidecar 17→18** (navigation/mod.rs:79-106), single transition, four-path cache-parity battery.

## 5. Acceptance gates (v4, normative)

1. Red-first fixtures — every NEGATIVE first asserts the pre-consult route is `ConcretePromotedDeferred` (S6), then asserts resolver AND manifest stay dropped: four cross-profile axes (embedded-target qualifier / ordinary fields / own methods / embedded-alias selector names ⇒ ProfileConflict); `profile_variants > 1` (W1 — the b_linux/b_windows shape from promoted_snapshot_test.rs:392-424 extended to a deferred call site); `field_shadowed`; `ambiguous_promoted_methods`. POSITIVE: profile-unique single-variant promoted method → Exact `EmbeddedPromotion` to the snapshot target (resolver + manifest route `concrete_promoted_snapshot`); depth>1 iff snapshot lists it; a pointer-receiver promoted method on a value receiver RESOLVES (W2 — pinning the no-shape-check behavior). **r3-W1 NEGATIVES (deserialized-snapshot constructions): declarations count 0 and count 2 ⇒ drop + `invariant_drop`; empty `profile_variants` ⇒ drop + `variant_drop`; singleton `profile_variants` NOT containing `target` ⇒ drop + `variant_drop` — each asserting resolver AND manifest stay `ConcreteReceiverPromotedDeferred`/`concrete_promoted_deferred_drop` and that `go_concrete_receiver_promoted_deferred` still increments (conservation).**
2. Same-base 5-corpus control vs mainD: deltas confined to `promoted_deferred`↓, `go_promoted_snapshot_{hits,conflict_drop,variant_drop}` (new, pinned), `embedded_promotion` kind↑ (W4); **`go_promoted_snapshot_invariant_drop == 0` on every corpus; drop-counter conservation holds (r2-S4)**; ripgrep byte-identical.
3. **Exhaustive resolver-delta target audit (W3, r2-W1)**: `--dump-sites` diff (candidate vs mainD control) → a PATCHED `direct_lane_audit.py` (or successor) run with kinds = `embedded_promotion` only, processing the ENTIRE delta: gate requires `sampled == new_sites`, `unknown == 0`, and every individual target's gopls `textDocument/definition` landing inside the prism target span; any miss or unknown = FAIL. A proven zero-site delta passes as NO-DATA. The interface-manifest oracle delta additionally runs (gate_ok TRUE) but is acknowledged blind to non-interface promoted methods.
4. Suite + tier-a `--matrix-only` green; fmt clean; four-path cache-parity battery.

## 6. Risks

- R1: consult/manifest skew → one shared fn (P17 doctrine) + gate 1 dual asserts.
- R2: variant-drop starves the win (many deferred sites profile-variant) → the census in gate 2's counters says how much W1's fail-close costs; caller-aware selection is the measured follow-up if large.
- R3: cache staleness → no format change; parity battery.
