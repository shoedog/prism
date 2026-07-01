# Phase-IP spec review — ROUND 2 (focused §6/§7) — claude opus — 2026-06-14

Operator subagent (read-only, prism nav). Focused rigor+soundness on the new Option-B surface
(rev 2): §6 canonical signature engine, §7 receiver-kind/pointer-aware RTA, §8/§12/§13 Exact-into-
ExactOnly guard. Round-1 findings already folded (not re-litigated). Codex round-2 companion:
`phase-ip-spec-review2-codex-2026-06-14.md`.

## Claim verification (1–4)

**Claim 1 — §6 canonical-signature feasibility + completeness: CONFIRMED-but-INCOMPLETE.** The two
extractors emit param names today (`extract_method_signature` go.rs:356-378; `extract_func_signature`
go.rs:436-448) and `compute_satisfaction` matches `.keys()` only (go.rs:462-464) — as the spec
states. A single types-only `canonical_sig` is feasible, but the algorithm is under-specified for
real Go shapes (provider has zero generics handling — grep empty). See B1.

**Claim 2 — §7 receiver-kind + pointer-aware RTA: CONFIRMED, one unstated coupling + a named recall
gap.** FunctionId identity holds (both `CallGraph::build` via `all_functions`→`node_line_range`
ast.rs:2640-2642 = `row+1`, and `extract_method` go.rs:399-400 = `row+1`, compute identical lines
from the identical node; `data.methods` is bare-`T`-keyed go.rs:425 so `*T` lives only in
satisfier/live sets). The `new(T)`/`var x T`/factory recall gap is real and only half-named (§7 says
`new(T)` covered, but `scan_go` live_types.rs:158-173 catches only `composite_literal`). See B2/M2.

**Claim 3 — §8/§12/§13 Exact-into-ExactOnly guard: CONFIRMED-INSUFFICIENT.** The interface fixture is
single-implementer (`Fast`); §12.2/§12.7 top out at 2 → wide fan-out never exercised. The §13.2 gate
floor 0.81 is the already-FAILING U-method-callers number (baseline.md:35, "NOT MET … collision FPs
survive even on unique method names"). The gate permits shipping fan-out as long as an already-failing
metric doesn't drop further. See B3.

**Claim 4 — §10 incremental closes the hazard for BOTH maps: hazard CONFIRMED, fix PRESCRIPTIVE-NOT-
PRESENT + under-specified.** `merge` (call_graph.rs:753-770) carries the existing indexes but NOT
`interface_impls`/`promoted_method_keys`; `build_incremental` (build.rs:166-192) calls
`assemble_graph` right after `merge` with no dispatch-recompute step. `files` at build.rs:170 is the
full merged map, so `from_parsed_files(files)` would see all files — hazard closed *if implemented*,
but §10 omits "clear before recompute" and the symmetric `normalize_go_key`. See B4.

## BLOCKER

**B1 — §6 canonical-signature grammar undefined for shapes that occur in caddy → "earned Exact" is
unverifiable, silently mis/under-satisfies.** Silent/ambiguous on: (i) **generics / type params**
(provider has zero handling; `type Ordered interface { ~int|~string }` type-sets, generic methods
`func (s S[T]) Get() T` have no canonical form); (ii) **`interface{}` vs `any`** (Go-identical; must
normalize or `func(any)` won't satisfy `func(interface{})`); (iii) **channel direction**
`chan<- T`/`<-chan T`/`chan T` (distinct in Go matching; §6's "preserve `chan T`" doesn't say
direction kept); (iv) **zero vs `()` returns**, **single `T` vs `(T)`** (node shapes differ between
`method_spec` and `method_declaration`; §6 never says both walk to the same token stream).
Consequence: any shape whose canonical form differs between the two walks → false non-satisfaction →
interface edge silently drops (recall regression on real `io.Reader`/`error`), OR over-normalization
(dropping channel direction) → false Exact into ExactOnly. This is the literal core of Option B and
the least-specified section. **Fix:** specify `canonical_sig` as an explicit recursive type-serializer
with a defined token for every Go type node kind the extractors already enumerate (go.rs:369-371:
`pointer_type`/`slice_type`/`map_type`/`channel_type` *with direction*/`array_type`/`interface_type`/
`function_type`) + a generics decision (canonicalize OR explicitly out-of-scope → drop to a *recorded
gap*, not a false match); add a §12 fixture per shape asserting interface-side and concrete-side
canonical strings are byte-equal.

**B2 — §7 never states the `T`/`*T` encoding contract as an invariant shared across three producers.**
The satisfaction map (§7 `set_value`/`set_ptr`), the live set (`scan_go`), and `resolve_dispatch`'s
`satisfying.intersection(live_types)` (go.rs:649) must agree on the exact string form. Today all three
are bare-`T` (extract_receiver strips at go.rs:425; scan_go strips `&` at live_types.rs:165). §7
changes two of three with no canonical encoding rule. Consequence: if `scan_go` emits `*T` for `&T{}`
but the satisfaction map stores the value-receiver satisfier as `T`, the go.rs:649 intersection misses
→ live satisfying type pruned → recall loss, masked by single-implementer value-receiver fixtures.
**Fix:** state the invariant explicitly (admission tokens: value-receiver satisfier → `T`,
pointer-receiver-only satisfier → `*T`; `scan_go` emits `T` for `T{}` and BOTH `T` and `*T` for
`&T{}`/`new(T)`) + a pointer-receiver-only fixture.

**B3 — §13.2 gate floor (0.81) is the already-failing baseline → the guard admits fan-out without
proving precision.** baseline.md:35: caddy callers/U-method 0.81 = NOT MET. §13.2 = "must not drop
below 0.81." A wide interface (`error.Error()` has dozens of live satisfiers in caddy) mints a
correct-but-huge Exact fan-out into the ExactOnly BFS; §12.7 caps at "several" and never exercises a
20-wide fan-out; the aggregate gate can't catch a new per-slice precision loss that lands above 0.81.
**Fix:** (a) make the gate a *delta on the same sites* — "interface-dispatch-attributable FPs at the
re-adjudicated 57 sites (§13.3) = 0" — bind to that set, not aggregate 0.81; and/or (b) define
fan-out-width behavior NOW (telemetry counter + documented threshold above which interface edges
demote to NameOnly so they leave ExactOnly), not deferred to §14. For a precision-biased ExactOnly
consumer the safe default is bound-first-relax-later.

**B4 — §10 full-recompute omits clear-before-recompute + the symmetric `normalize_go_key`.** §10 says
"recompute from the merged file set" without "empty the cached maps first." If recompute extends onto
cached non-empty maps, a satisfier that stopped satisfying after an edit leaves a stale Exact entry →
incremental and full builds diverge (phantom Exact edge until cold rebuild). Also §10 never restates
that `normalize_go_key` (§9) applies to BOTH the stored key and the recovered `recv_ty` at
resolution.rs:412 — and there are already two normalizers (`owner_key` resolution.rs:75, `peel_type`
resolution.rs:88); a third that disagrees breaks store-vs-lookup key match. **Fix:** §10 → "**replace**
(not merge)"; §9 → state whether `normalize_go_key` *is* `owner_key` (reuse) or why new, and assert
it's the sole normalizer on both paths.

## MAJOR

**M1 — §6 embedded-interface flatten (go.rs:486-505 `methods.extend`) happens on raw sigs; canonicalize
at extraction so flattened maps hold canonical sigs; drop name-collision across embeds with unequal
canonical sigs (mirror §4.2 equal-depth drop).**

**M2 — §7 RTA recall gap (`var x T`, factory returns) under-named, and combined with the empty→full
fallback produces a precision cliff, not recall.** `scan_go` sees only `composite_literal`; factory/
`var`-obtained types are never live. For single-implementer this is benign (per-interface fallback
go.rs:653). But the fallback IS the precision hole: when liveness is incomplete (idiomatic
constructor-based Go), intersection empty → fallback to ALL satisfiers → the B3 blow-up, triggered by
an RTA *miss*. **Fix:** §7 enumerate live coverage honestly (composite literals only) and cross-ref
B3 — the fan-out lever should gate the *fallback* path, not just genuinely-wide interfaces.

**M3 — provider built twice (CallGraph::build §2 + registry context.rs:253); correctness fine (same
pure fn), but 2× AST walk + satisfaction over all Go files on the hot full-build path (caddy 564
files).** **Fix:** state it; accept with a measured cost note, OR build once and share `Arc<GoTypeData>`
— but build order (CallGraph before registry) means sharing requires hoisting construction above both
consumers. Pick one and write it down.

## MINOR
- **m1** §8 "same justification as Step-9 CHA (build.rs:569)" — Step-9 is type_db-gated C++, never runs
  for Go; note "no shared code path."
- **m2** §6 bare-naming inside `canonical_sig` reintroduces cross-pkg collision INTO signature matching
  (`Read(io.Reader)` vs `Read(bufio.Reader)` → same canonical) — a second over-approx site beyond §14's
  key note; acknowledge.
- **m3** Absence: the seam (resolution.rs:412 `None =>`) is reached only when P6-lite recovered a
  receiver (`site.receiver_type.is_some()`); un-recovered `x.m()` never reaches it — state it.
- **m4** Absence: `ResolutionKind` is telemetry-only (not serialized; only `as_str`), `ResolutionConfidence`
  is the Ord/wire field — so "both → Exact via `exact()`" is load-bearing, the kind is cosmetic.

## Most-likely-regretted
1. **The empty→full RTA fallback reused as-is** — converts an RTA miss into a maximal Exact fan-out, and
   Go constructors guarantee RTA misses (M2). Bites on factory-heavy Go (caddy) under the 0.81-floored
   gate (B3).
2. **Deferring the fan-out-width lever to §14** — for a precision-biased ExactOnly consumer, ship
   bounded then relax; not ship-uncapped then measure.

**Verdict: needs changes** — four BLOCKERs, all in the new Option-B surface round 1 never saw
(canonical-sig grammar B1, encoding-contract invariant B2, gate-floor-is-the-failing-baseline B3,
replace-not-merge + single-normalizer B4).
