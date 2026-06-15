# Phase-IP — Type-Confirmed Receiver Dispatch (Go) — Design

**Date:** 2026-06-14 · **Status:** rev 3 — **two dual-review rounds folded** (round 1: codex
rigor 4 BLOCKERs + claude soundness "sound-to-plan"; round 2 **focused on the Option-B §6/§7
surface**: codex + claude both "needs changes", 4+ BLOCKERs each). Records:
`docs/prism-query-layer/phase-ip-spec-review{,2}-{codex,claude}-2026-06-14.md`. **Owner decisions:**
(1) *earn the Exact* — interface dispatch is `Exact` via **signature-confirmed** satisfaction;
(2) **fix Go liveness, keep the empty-intersection fallback Exact** (round-2 keystone) — maximize
RTA coverage so the fallback rarely fires; accept genuinely-wide *live* fan-outs into `ExactOnly`,
guarded by a precision gate + telemetry, not a width cap. Builds on **EFT** (`e7a37e5`) +
**pre-IP** (`710cb86`). Closes the Go half of `s3-deferred §1`/`§2`.

### rev 3 changes (round-2 fold)
- **Owner keystone — keep fallback Exact, fix liveness (§8 NEW).** The empty-intersection fallback
  stays `Exact`; rev 3 makes `scan_go` comprehensive (`new(T)`, `var x T`, factory returns,
  pointer-aware `T`/`*T`) so the fallback fires only on residual RTA gaps. Resolves the §7-vs-fallback
  contradiction (codex r2-B1) by making the fallback **receiver-kind-aware-empty** (fires only when no
  admission key of any kind is live).
- **§6 canon-signature is now a full recursive grammar** (`canon_type`, codex r2-B-grammar / claude
  r2-B1): channel direction preserved, `interface{}`≡`any`, defined per Go type node; **generics/
  type-sets and anonymous interfaces are explicitly out-of-scope → recorded gaps, never false matches**
  (and removed from caddy acceptance, codex r2-MAJOR3).
- **§7 three-key receiver encoding** (codex r2-B2 / claude r2-B2): admission key `T`/`*T`, method-body
  key bare `T`, interface key; strip `*` only after RTA admission. Embedding promotion retains embedded
  pointer kind (codex r2-MAJOR5).
- **Precision guard fixed** (claude r2-B3): the gate is **interface-attributable FPs = 0 on the
  re-adjudicated 57 caddy interface sites**, NOT the already-failing aggregate 0.81; + fan-out
  telemetry; width-lever stays a deferred escape hatch (§15).
- **One `normalize_go_key` everywhere + replace-not-merge across all three build paths** (codex
  r2-B6/MINOR9/GAPS / claude r2-B4): incremental and **`build_scoped`** compute Go dispatch over the
  **full** repo file set; aliases/maps are cleared then repopulated.

## 0. Why now

EFT won *precision* (`ExactOnly` traversal). This is its **recall** counterpart. Two Go shapes
resolve to nothing today, both owner-approved `expected_gap`:
- **`go/embedded_method`** — `w.Ping()`, `Wrap` embeds `Base`, `Ping` on `Base` → `owner_lookup`
  miss → `ExternalReceiver` drop (resolution.rs:422).
- **`go/interface_dispatch`** — `r.Go()`, `r: Runner` (interface), `Fast` implements `Go()` →
  structural satisfaction has no syntactic `impl` to dual-key → drop.

Per `s3-deferred §2`, P6-lite here is stricter than R6 (recovery-then-drop loses an edge R6 would
keep). Phase-IP completes the type model. **In scope:** Go embedding + Go interface dispatch
(signature-confirmed, receiver-kind-aware, comprehensive-RTA, multi-implementer). **Deferred (§15):**
Python; `from_import_alias`; Rust S3.1; Go generics/type-sets; anonymous interfaces.

## 1. The decisive constraint

Success signal = **caddy nav recall** (prism `nav callers/callees` vs gopls). Nav and CPG resolve
through the **same ladder** `resolve_call_site`; nav re-resolves on `cg.call_graph` and **never reads
CPG edges** (EFT §0; verified both rounds — `direct_callers` queries.rs:247-262, `direct_callees`
queries.rs:362-389). Dispatch **must flow through `resolve_call_site`**. A build-time CPG-only
post-pass is **rejected** (would not move nav recall).

## 2. Architecture — consume + upgrade the existing GoTypeProvider

Build order (verified): `CallGraph::build` (build.rs:136) runs **before** the `TypeRegistry`
(context.rs:63-65); `resolve_call_site` materializes edges in `assemble_graph` Step 5 (build.rs:
349-379) and is the nav query path → it cannot reach the registry. **But the provider is
registry-independent** (`GoTypeProvider::from_parsed_files(files)`, go.rs:102), so `CallGraph::build`
constructs + consumes it, precomputing dispatch into CallGraph-owned maps that serialize with
`CallGraph` (GIT_SHA `v7` invalidates). `src/type_providers/go.rs` already computes satisfaction,
flattens embedded interfaces, resolves promoted methods, and implements `resolve_dispatch` — but
**nothing consumes it** (zero call sites). Phase-IP consumes + upgrades it; **no new `src/dispatch/`
module**. The registry's registered provider stays the future algorithm-facing API.

| # | Approach | Verdict |
|---|---|---|
| **1** | Consume `GoTypeProvider` in `CallGraph::build`; `resolve_call_site` reads precomputed maps. | **CHOSEN** — only path through the one ladder; reuses the provider; registry-independent. |
| 2 | Build-time CPG post-pass (Step-9 CHA mirror). | Rejected — nav never reads CPG edges → metric unmoved. |
| 3 | Thread `TypeRegistry` into `resolve_call_site`. | Rejected — registry absent at Step-5; invasive. |

## 3. Decisions (owner-locked; rev 3)

| Decision | Choice |
|---|---|
| Scope | Go embedding + Go **named in-repo** interface dispatch. Anonymous interfaces, generics/type-sets, Python, Rust S3.1 deferred (§15). |
| Confidence | **Both Exact, earned** — embedding (deterministic rule) + interface (signature-confirmed §6, receiver-kind-aware §7, RTA-confirmed §8). |
| RTA fallback | **Keep Exact** (owner decision). Make `scan_go` comprehensive (§8) so the empty-intersection fallback rarely fires; fallback is **receiver-kind-aware-empty** (fires only when no admission key of any kind is live, resolving codex r2-B1). |
| Fan-out | **Uncapped** — genuinely-live wide fan-outs (`error`) enter `ExactOnly` as sound may-targets; guarded by the §14 precision gate + fan-out telemetry, **not** a width cap. Width-lever deferred (§15). |
| Satisfaction | Signature-confirmed (canonical sig §6); upgrade `compute_satisfaction` from name-only (go.rs:462). |
| Receiver kind | Three keys (§7): admission `T`/`*T`, method-body bare `T`, interface key; embedding retains embedded pointer kind. |
| Keys | **One `normalize_go_key`** (strip `*`,`&`,`pkg.`,`[…]`) at every Go store + lookup, incl. recovered receiver (§10). |
| Resolution kinds | `EmbeddedPromotion` + `InterfaceDispatch`; both → `Exact` via `exact()` (kind is telemetry-only — not serialized; only `as_str`). |
| Cache | No `CACHE_VERSION` bump (`CpgEdge` unchanged; new CallGraph fields `#[serde(default)]`; GIT_SHA `v7`). **Replace-not-merge** + full-repo recompute on `build_incremental` and `build_scoped` (§11). |
| Out-of-scope keys | Generics/type-sets + anonymous interfaces → recorded gaps, never false matches; removed from caddy acceptance (§14). |

## 4. Mechanism A — Go embedding (owner-index promotion, receiver-kind-aware)

Reuse the provider's transitive promotion (`collect_promoted_methods_from`, go.rs:529) but **retain
embedded pointer kind** (codex r2-MAJOR5 — today extraction strips embedded `*T`→`T` at go.rs:291).
`CallGraph::build` writes promoted aliases into `methods: (owner_key, method) → [FunctionId]` (the
existing trait dual-key shape):

1. For struct `S`, transitively promoted `(m, fid)` (fid = defining type's method): **only if `S` has
   no direct `m`** (direct-wins, codex r1-#5), insert `methods[(norm(S), m)] += fid`, record
   `promoted_method_keys += (norm(S), m)`.
2. **Go method-set rule for selector calls:** a *value* receiver `w Wrap` (an addressable param) may
   call both value- and pointer-receiver promoted methods (Go auto-addresses addressable values), so
   the owner alias includes both kinds. (Interface *satisfaction* uses the stricter value/pointer
   split, §7 — the two rules are deliberately separate, codex r2-MAJOR5.)
3. Equal-depth ambiguity → **not** promoted (the provider's `or_insert` is augmented with an
   equal-depth detector; §13 test 1c).

Resolution: `owner_lookup(norm(recv_ty), name)` hits the promoted alias → `Exact`, labeled
`EmbeddedPromotion` when `(norm(recv_ty),name) ∈ promoted_method_keys`. `method_owners[fid]` stays the
defining type. No `resolve_dispatch` involvement (embedding is owner-index completeness).

## 5. Mechanism B — Go interface (signature-confirmed multi-implementer)

`CallGraph.interface_impls: BTreeMap<(String /*norm iface*/, String /*method*/), Vec<FunctionId>>`,
precomputed in `CallGraph::build`:

1. `let go = GoTypeProvider::from_parsed_files(files)` with **upgraded** signature-confirmed (§6) +
   receiver-kind-aware (§7) satisfaction.
2. `let live = comprehensive Go live set` (§8, pointer-aware).
3. For each **named in-repo** interface `I` and method `m ∈ I`: `v = go.resolve_dispatch(norm(I), m,
   &live)`. **Store only if `!v.is_empty()`** (explicit drop otherwise — codex r1-#2).

`resolve_dispatch` keeps the empty-intersection fallback (owner decision) but **receiver-kind-aware**:
the intersection matches admission keys (`T`/`*T`, §7); the fallback to the full satisfier set fires
only when **no** admission key of any kind is live for `I` (not when the opposite kind is observed —
codex r2-B1). With comprehensive liveness (§8) this is rare.

Resolution seam (resolution.rs:412-424), on `owner_lookup(norm(recv_ty), name)` **miss**:
```rust
None => match self.interface_impls.get(&(normalize_go_key(recv_ty), name.to_string())) {
    Some(ids) if !ids.is_empty() =>
        ResolutionOutcome::hit(exact(ids.iter().collect(), ResolutionKind::InterfaceDispatch)),
    _ => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
}
```
`exact(...)` accepts N callees (R1 trait dual-key) and the N-hit composes with Step-5 (one
`Call(Exact)` per callee, build.rs:362) and nav re-resolution (call_resolve.rs:15-26). The seam is on
the **P6-lite-recovered branch only** — receivers P6-lite cannot type still drop upstream (codex
r2-m3; out of scope). Step-9 CHA is `type_db`-gated (C++), independent of Go (no shared code path —
codex r2-m1).

## 6. Canonical signature — `canon_type` recursive grammar (codex/claude r2-B1)

Today `extract_method_signature` (go.rs:356) and `extract_func_signature` (go.rs:436) emit **raw text
with param names** → non-comparable, hence the name-only fallback (go.rs:462). rev 3 defines **one
`canon_type(node) -> String`** and **one `canon_sig(params_node, result_node) -> String`** applied
**identically** to interface `method_spec` and concrete `method_declaration`, and to flattened
embedded-interface methods (canonicalize **at extraction** so `collect_interface_methods_from`
(go.rs:486) merges already-canonical sigs; drop a name-collision across embeds with unequal
`canon_sig` — mirrors §4.3 equal-depth drop — claude r2-M1):

- **`canon_sig`** = `(` join(param `canon_type`, `,`) `)` `(` join(result `canon_type`, `,`) `)`.
  Names + blank `_` dropped; grouped params expanded (`a, b int`→`int,int`); zero params/returns →
  empty list; single unparenthesized return → one-element list.
- **`canon_type`** (recursive, per Go type node — the kinds the extractors already enumerate,
  go.rs:369-371):
  - `type_identifier` / `qualified_type` → **bare name** (`pkg.T`→`T`, via `normalize_go_key`).
  - `pointer_type` → `*` + inner; `slice_type` → `[]` + inner; `array_type` → `[N]` + inner.
  - `map_type` → `map[`K`]`V; `channel_type` → **direction-preserving** `chan`/`chan<-`/`<-chan` +
    inner.
  - `function_type` → `func(` params `)` results (recurse `canon_sig`).
  - `interface_type`: `interface{}` / `any` → **`any`** (normalized). A **non-empty anonymous
    interface** → **out of scope** (§3): the enclosing method is non-dispatchable (recorded gap), not
    matched.
  - **Generic instantiation / type parameters** (`generic_type`, `type_arguments` `[…]`, type-set
    `type_elem`): **out of scope** — an interface or method bearing type parameters / type sets is
    **non-dispatchable** (recorded gap, telemetry), never a false match.
- Satisfaction (`compute_satisfaction`, go.rs:455) becomes: `T` satisfies `I` iff for every `m ∈ M_I`
  there is a method named `m` on `T`'s (§7) method set with **equal `canon_sig`**.

**Cross-package note (codex r2-m2):** bare-naming inside `canon_type` means `Read(io.Reader)` and
`Read(bufio.Reader)` canonicalize equal — a second over-approx site (beyond keys); measured by the
§14 gate, precise cross-pkg keys deferred (§15).

## 7. Receiver-kind correctness — three keys (codex/claude r2-B2)

Go method sets are asymmetric (`*T` set ⊇ `T` set). `GoMethod.is_pointer_receiver` is tracked
(go.rs:60). rev 3 fixes the encoding invariant shared by three producers:

- **Admission key** (satisfaction map values + live set): value-receiver satisfier contributes `T`;
  pointer-receiver-only satisfier contributes `*T`.
- **Method-body key** (`data.methods`, FunctionId lookup): bare `T` — strip `*` **only after** RTA
  admission (go.rs:660-672 unchanged; the target FunctionId is identical for `T`/`*T`, verified
  go.rs:399-400 vs ast.rs:2640).
- **Interface key**: `normalize_go_key(I)`.
- Satisfaction: `T` satisfies via `set_value(T)` = `{m : !is_pointer_receiver}` (+ value-promoted);
  `*T` satisfies via `set_ptr(T)` = all (+ promoted). Records the satisfying admission key.
- `resolve_dispatch` intersects admission keys against the §8 live set; fallback per §5.

Invariant (stated so a partial impl can't silently break the intersection): **the satisfaction map,
the live set, and the intersection all use the admission-key alphabet `{T, *T}`; the method-body
lookup uses bare `T` after admission.**

## 8. Comprehensive Go liveness (the Option-B work; codex/claude r2-M2)

`scan_go` (live_types.rs:158) today catches only `composite_literal` and strips `&`, so `new(T)`,
`var x T`, and factory returns are invisible and `&T{}`/`T{}` collapse — making the fallback fire on
idiomatic Go. rev 3 extends `scan_go` (Go-scoped) to the admission-key alphabet:

- `T{}` (composite literal, value) → `T`.
- `&T{}` / `new(T)` → `*T` **and** `T` (both method sets become live; `&T{}` is addressable).
- `var x T` (var_declaration with a concrete type) → `T`.
- **Factory returns:** a function whose result type is `T` / `*T` and whose body constructs it →
  mark `T` / `*T` (align with the existing constructor-local recognition, ast.rs:3899-3920). Sound
  over-approx (may-live), consistent with RTA.
- Qualified/pointer normalization via `normalize_go_key` for the bare part; pointer-ness preserved in
  the admission key.

Coverage is honestly bounded: reflection / cross-module-only instantiation remain invisible → the §5
receiver-kind-aware fallback is the residual safety net (kept Exact per the owner decision;
over-approx measured by §14). Other languages' `scan_*` are unchanged.

## 9. Confidence + EFT reconciliation

Both mechanisms mint **Exact**, earned: embedding (deterministic), interface (signature-confirmed §6
+ receiver-kind-aware §7 + RTA-confirmed §8). The fan-out to N **live** satisfiers is N
provably-possible targets — sound may-analysis (same class as Step-9 CHA, no shared code). Interface
edges enter `ExactOnly` slices (the recall win lands there). **Accepted risk (owner):** a wide live
interface (`error`) yields a large *correct* Exact fan-out into precision-biased slices; rev 3 does
**not** cap it — it is bounded by RTA precision (§8) and **measured** by the §14 gate + fan-out
telemetry, with the §15 width-lever as the escape hatch if the gate trips.

## 10. Wiring & data structures

| Item | Location | Change |
|---|---|---|
| `interface_impls`, `promoted_method_keys` | call_graph.rs:47 | new `#[serde(default)]` fields; init in all constructors. |
| Consume provider | call_graph.rs:206 (`build`) | construct `GoTypeProvider`; write promoted aliases (§4) + `interface_impls` (§5). |
| Signature + satisfaction | go.rs | add `canon_type`/`canon_sig` (§6); rewrite the two extractors to delegate; upgrade `compute_satisfaction`; receiver-kind sets (§7); retain embedded pointer kind; expose `promoted_methods(type)`. |
| Comprehensive liveness | live_types.rs `scan_go` | §8 admission-key alphabet. |
| `normalize_go_key` | resolution.rs (new) | strip `*`,`&`,`pkg.`,`[…]`; the **sole** normalizer for Go method/interface/promoted/embedded keys **and** recovered receiver, applied **before** direct/promoted/interface lookup. (`owner_key` resolution.rs:75 stays for non-Go; it strips `<…>` but not `pkg.`/`[…]` — document the split.) |
| Resolution consult + labels | resolution.rs:412-424 | embedding label via `promoted_method_keys`; interface consult §5; explicit empty→drop. |
| Resolution kinds | resolution.rs `ResolutionKind` | add `EmbeddedPromotion`,`InterfaceDispatch` + `as_str()` arms (only exhaustive match; `call-stats` auto-updates); kind is telemetry-only, confidence rides `exact()`. |
| Fan-out telemetry | navigation/queries.rs (`call-stats`) | record `InterfaceDispatch` fan-out width. |

## 11. Cache & build paths

- **No `CACHE_VERSION` bump** (`CpgEdge` unchanged; new fields `#[serde(default)]`; GIT_SHA `v7`
  invalidates resolver change; nav cache delegates to `cpg_cache::load_cache`, navigation/cache.rs:45).
- **Whole-program over all three build paths.** Embedding/satisfaction/RTA are whole-program:
  - `build` (build.rs:136): provider over all `files`.
  - `build_incremental` (build.rs:166): after CG `merge`, **replace** (clear, then repopulate)
    `interface_impls` and all `promoted_method_keys` aliases in `methods`, recomputed via
    `from_parsed_files(all merged files)` — closes the `remove_files`-by-`fid.file` stale-alias hazard
    (codex r2-MINOR9 / claude r2-B4).
  - `build_scoped` (context.rs:135): Go dispatch computed over the **full** repo file set, not the
    scoped subset (codex r2-GAPS; mirrors `live_types` already collected from all files,
    context.rs:170).

## 12. Failure modes

| Mode | Behavior |
|---|---|
| Interface, ≥1 live satisfier | intersection → live satisfiers, Exact. |
| Interface, no admission key of any kind live | receiver-kind-aware fallback → full satisfier set, Exact (residual-gap net, §8; over-approx measured §14). |
| Interface, no in-repo satisfier | no `interface_impls` entry → explicit `ExternalReceiver` drop. |
| Value type, pointer-method interface | not admitted via `set_value`; admitted only if `*T` live (§7/§8). |
| Same name, different `canon_sig` | excluded (§6) — no false Exact. |
| Generic / type-set / anonymous-interface method | non-dispatchable, recorded gap (§6); never a false match. |
| Embedded `*T` method on value selector | promoted (addressable receiver, §4.2); satisfaction split is stricter (§7). |
| Cross-package bare-name collision | admitted (over-approx, §6 note); measured §14. |
| Warm cache from pre-IP / incremental stale alias | GIT_SHA reject / replace-not-merge (§11). |
| Non-Go repos | maps empty; zero behavior change (regression guard §13). |

## 13. Testing & acceptance (pre-commit, no LSP)

1. **Embedding:** (a) `w.Ping()`→`Base::Ping`, `EmbeddedPromotion`, Exact; (b) transitive `A→B→C`;
   (c) equal-depth ambiguity → dropped; (d) direct shadows promoted; (e) value selector of an
   embedded `*T`-receiver method resolves (§4.2).
2. **Interface:** (a) `r.Go()`→`Fast::Go`, `InterfaceDispatch`, Exact; (b) multi-implementer →
   both; (c) RTA: `live={Fast}`, `Slow` uninstantiated → only `Fast`; (d) receiver-kind-aware
   fallback (no kind live) → full set.
3. **`canon_sig` byte-equality (§6):** interface-side and concrete-side canonical strings are equal
   for: param-name-only diff; grouped `(a,b int)` vs `(int,int)`; **channel direction** `chan<- T`
   ≠ `chan T`; `interface{}`≡`any`; multi-return; single vs parenthesized return; **return-type
   mismatch → no satisfy**; arity mismatch → no satisfy.
4. **Out-of-scope gaps (§6):** a generic/type-set interface and a non-empty anonymous-interface
   method are recorded non-dispatchable (no false Exact, no panic).
5. **Receiver kind (§7):** pointer-receiver-only type satisfies via `*T`; value `T{}`-only does not.
6. **Comprehensive liveness (§8):** `new(T)`, `var x T`, and a factory-returned type are each live
   (and pointer-aware `&T{}`→`*T`).
7. **Capability matrix flip:** `eval/fixtures/go/{embedded_method,interface_dispatch}/expected.toml`
   `known_fail → pass`; update rationale; matrix asserts both `ok`.
8. **Multi-implementer precision (claude r1-MAJOR):** a barrier-slice fixture seeded at an
   interface method with several **live** implementers → the `ExactOnly` fan-out is exactly the live
   satisfiers (no non-satisfier leakage).
9. **Non-Go regression:** Rust/Python/C resolution byte-identical.
10. **Cache:** new fields round-trip; cross-GIT_SHA rejected; incremental **replace** drops a removed
    implementer's edge (no phantom).

**Repo workflow:** `cargo fmt`; full `cargo test` (+`--features mcp`); `cargo build --release` then
`cd eval && uv run tier-a --matrix-only --allow-stale-sut` (both Go gaps `ok`, no other flips) then
`--quick`.

**Success metric:** the two Go cases flip `expected_gap → ok`; caddy nav-callers recall measurably
lifts (recorded in the §14 re-baseline); **no regression** — EFT `target-c-method` exact P=R=1.0,
default `flip_candidate`; prism/tokio/flask/click matrix + quick unchanged.

## 14. Human-triggered acceptance (not auto-run)

1. **Full 5-corpus rerun** (`uv run tier-a --corpus all`).
2. **Precision gate (claude r2-B3, corrected):** at the **re-adjudicated 57 caddy interface sites**,
   **interface-dispatch-attributable FPs in `ExactOnly` = 0** — a *delta* gate on that set, NOT the
   already-failing aggregate 0.81. If it trips, apply the §15 fan-out width-lever before re-baselining.
   Fan-out-width telemetry reviewed for outliers (`error`-class).
3. **Re-adjudicate the 57 EFT-ambiguous caddy interface sites** — now resolved; verdicts re-anchor
   via the fingerprint store (pre-IP #2). Dual-adjudicator, record κ.
4. **caddy anchor re-baseline** — deliberate, with the adjudication record.

## 15. Out of scope / deferred

- **Go generics / type-sets** and **anonymous interfaces** — recorded gaps (§6); anonymous-interface
  caddy sites are **excluded from §14 acceptance** (no overclaim). Synthetic canonical keys for
  anonymous method sets = future increment (codex r2-MAJOR3).
- **Fan-out width-lever** — telemetry exists now (§10); a threshold-demote of very wide interfaces to
  NameOnly is added only if the §14 gate trips (owner: uncapped honest Exact by default).
- **Precise cross-package canonical keys / signatures** — bare-name over-approx now (§6 note); promote
  to package-qualified if §14 shows material FPs.
- **Provider built twice** (CallGraph::build + registry context.rs:253) — same pure `from_parsed_files`
  (no divergence) but 2× the Go AST walk on the full-build path (caddy 564 files; satisfaction cheap
  vs parse). Accepted with this note; hoisting construction above both consumers (to share
  `Arc<GoTypeData>`) is a deferred perf opt (build order makes it non-trivial — codex/claude r2-M3).
- **Python inheritance, `from_import_alias`, Rust S3.1** (`s3-deferred §3`, the prism C-method recall
  lever 0.121).
- **`DispatchProvider` as the algorithm-facing API** — the registered provider's `resolve_dispatch`
  for slice algorithms with `CpgContext.live_types`; distinct from this resolver-internal consumption.
