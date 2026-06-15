# Phase-IP — Type-Confirmed Receiver Dispatch (Go) — Design

**Date:** 2026-06-14 · **Status:** rev 2 — **dual review folded** (codex gpt-5.5 xhigh, rigor:
4 BLOCKERs; claude opus, soundness: "sound to plan", 1 MAJOR). Records:
`docs/prism-query-layer/phase-ip-spec-review-{codex,claude}-2026-06-14.md`. **Owner decision:**
*earn the Exact* — interface dispatch uses **signature-confirmed** satisfaction so its edges are
honestly `Exact` and enter EFT's `ExactOnly` slices (Option B). Builds on **EFT** (merged
`e7a37e5`) and **pre-IP hardening** (merged `710cb86`: cache GIT_SHA invalidation `v7`,
fingerprint-keyed adjudications). Closes the **Go half** of `s3-deferred §1` + `§2`.

### rev 2 changes (what the review moved)
- **Reuse, don't reinvent (codex #9, claude coupling).** `src/type_providers/go.rs` already has a
  registered `GoTypeProvider` that computes interface satisfaction, flattens embedded interfaces,
  resolves promoted (embedding) methods with direct-wins precedence, and implements
  `resolve_dispatch` with a per-interface RTA fallback — **but nothing consumes it**
  (`resolve_dispatch` has zero call sites). Phase-IP **consumes + upgrades** this provider; the
  proposed `src/dispatch/go.rs` is **deleted from the plan**.
- **Earn Exact via signature confirmation (codex BLOCKER #1, claude MAJOR §6 — Option B).** Today's
  satisfaction is name-only (go.rs:462 `.keys()`) because the two signature extractors emit raw,
  non-comparable text. rev 2 adds a **canonical signature** function (§6) + **receiver-kind-aware**
  method sets (§7) so `Exact` is honest, then admits interface dispatch to `ExactOnly` with a
  **multi-implementer precision gate** (§12/§13).
- **Contract fixes:** empty-vector → explicit drop (codex #2); incremental **full-recompute** of Go
  dispatch from all files (codex #3, claude); direct-method precedence (codex #5); bare-name key
  normalization both sides + qualified fixture (codex #6, claude); §2 factual correction (both).

## 0. Why now

EFT won *precision* (`ExactOnly` traversal; `target-c-method` exact P=R=1.0). This is its **recall**
counterpart. Two real Go call shapes resolve to **nothing** today, both owner-approved
`expected_gap` in the capability matrix:

- **`go/embedded_method`** — `func run(w Wrap) { w.Ping() }`, `Wrap` embeds `Base`, `Ping` on
  `Base`. P6-lite recovers `w: Wrap`, `owner_lookup("Wrap","Ping")` misses → `ExternalReceiver`
  drop (resolution.rs:422).
- **`go/interface_dispatch`** — `func run(r Runner) { r.Go() }`, `r: Runner` (interface), `Fast`
  implements `Go()`. P6-lite recovers `r: Runner`; structural satisfaction has no syntactic `impl`
  to dual-key → drop.

Per `s3-deferred §2`, P6-lite is here *stricter* than dumb R6 (recovery-then-`ExternalReceiver`
loses an edge R6 single-owner demote would keep). Phase-IP completes the type model the receiver
was recovered against.

**In scope:** Go embedding promotion + Go interface dispatch (signature-confirmed,
receiver-kind-aware, multi-implementer, RTA-pruned). **Deferred:** Python inheritance,
`python/from_import_alias`, Rust S3.1 struct-field/return-typed receiver — §14.

## 1. The decisive constraint (drives the architecture)

The success signal is **caddy nav recall** — prism `nav callers/callees` vs the gopls LSP oracle.
**Nav and CPG resolve through the same ladder** `CallGraph::resolve_call_site`. A build-time
CPG-only post-pass (mirror Step-9 CHA) would enrich the petgraph but **nav re-resolves on
`cg.call_graph` and never reads CPG edges** (EFT §0; verified by both reviews — `direct_callers`
queries.rs:247-262, `direct_callees` queries.rs:362-389). So dispatch **must flow through
`resolve_call_site`** to move the metric. Build-time post-pass **rejected** as the primary
mechanism.

## 2. Architecture — consume the existing GoTypeProvider at the ladder

Build order (verified by both reviews):

```
CodePropertyGraph::build_impl (build.rs:134-137)
  ├─ dfg = DataFlowGraph::build(files)
  ├─ cg  = CallGraph::build(files)            ← resolver state; NO TypeRegistry yet
  └─ assemble_graph(cg, dfg, files, type_db)
        └─ Step 5: materialize Call/Return edges via cg.resolve_call_site(...)   ← runs HERE
CpgContext::build (context.rs:63-65)          ← TypeRegistry + GoTypeProvider registered AFTER
```

`resolve_call_site` runs in Step 5 (before the registry) **and** is the nav query path, so it
cannot reach the `TypeRegistry`. **But the provider is registry-independent**: `GoTypeProvider::
from_parsed_files(files)` (go.rs:102) needs only parsed files. So `CallGraph::build` **constructs
and consumes the provider directly**, precomputing dispatch into CallGraph-owned maps that
`resolve_call_site` reads and that serialize with `CallGraph` (`#[serde]`, call_graph.rs:47;
GIT_SHA `v7` invalidates across the upgrade).

The registry's *registered* `GoTypeProvider` (context.rs:253-257) stays for the future
**algorithm-facing** `DispatchProvider` API (its documented purpose, context.rs:52) — a separate,
post-registry surface. One builder (`from_parsed_files`), two consumers; no divergent logic
(resolves codex #9).

### Approaches (unchanged verdict)

| # | Approach | Verdict |
|---|---|---|
| **1** | **Consume `GoTypeProvider` in `CallGraph::build`; `resolve_call_site` reads precomputed maps.** | **CHOSEN** — only path through the one ladder (nav+CPG+metric); reuses existing provider; registry-independent. |
| 2 | Build-time CPG post-pass (mirror Step-9 CHA). | Rejected — nav never reads CPG edges → caddy nav recall does not move (§1). |
| 3 | Thread `TypeRegistry` into `resolve_call_site`. | Rejected — registry absent at Step-5; invasive. |

## 3. Decisions (owner-locked; rev 2)

| Decision | Choice |
|---|---|
| Scope | Go embedding + Go interface dispatch. Python / Rust S3.1 / from_import_alias deferred (§14). |
| Interface bar | Full multi-implementer, RTA-pruned (all live satisfiers). |
| **Confidence (Option B)** | **Both Exact, earned.** Embedding is a deterministic Go rule → Exact. Interface is Exact **because** satisfaction is signature-confirmed (§6) + receiver-kind-aware (§7). Both enter `ExactOnly` slices; guarded by a multi-implementer precision gate (§12/§13). |
| Dispatch home | CallGraph-internal maps, fed by the existing `GoTypeProvider` (§2). No new `src/dispatch/` module. |
| Satisfaction | **Signature-confirmed**: name + **canonical signature** match (§6); upgrade `compute_satisfaction` (go.rs:455) from name-only. |
| Receiver kind | Value-type vs pointer-type method sets distinguished (§7); pointer-aware RTA. |
| RTA pruning | Reuse the provider's **per-interface intersection-empty → full-set** fallback (go.rs:646-657); replaces rev-1's weaker global rule (codex #7). |
| Resolution kinds | New `EmbeddedPromotion` + `InterfaceDispatch`; both → `Exact` (set by `exact()`, not a kind→confidence table — codex/claude #7). |
| Keys | **Bare type names** (strip pointer + `pkg.` prefix) at extraction AND at the seam (§9); cross-package collision is a noted over-approx (§14). |
| Empty result | `resolve_dispatch` returning empty → **explicit** `ExternalReceiver` drop; empty vectors never stored (codex #2). |
| Cache | No `CACHE_VERSION` bump (`CpgEdge` unchanged; new CallGraph fields `#[serde(default)]`; GIT_SHA `v7` invalidates). Incremental **full-recomputes** Go dispatch from all files (§10). |
| Acceptance | Matrix-only + `--quick` pre-commit (§12). caddy re-baseline + 57-site re-adjudication + precision gate are human-triggered (§13). |

## 4. Mechanism A — Go embedding (owner-index promotion)

Go embedding promotes the embedded type's method set onto the outer type — **statically and
unconditionally**. Reuse the provider's promotion logic (`collect_promoted_methods_from`,
go.rs:529 — already transitive + direct-wins via `or_insert`); expose it so `CallGraph::build`
writes promoted aliases into the owner index (`methods: (owner_key, method_name) → [FunctionId]`,
the existing trait dual-key shape, call_graph.rs:61-64):

1. For each struct `S` and each transitively promoted `(m, fid)` from the provider (fid is the
   **defining** type's method): **only if `S` has no direct `m`** (direct-wins, codex #5), insert
   `methods[(S,m)] += fid` and record `promoted_method_keys += (S,m)`.
2. Go equal-depth ambiguity: the provider's closure takes the first/shallowest; rev 2 keeps a
   uniquely-shallowest provider and **drops** a method offered by two embeds at equal depth
   (matches the multi-owner drop). *(Fold note: the provider's current `collect_promoted_methods_
   from` uses `or_insert` and does not detect equal-depth ambiguity — §12 test 1c pins it; the
   plan adds the ambiguity guard.)*

The existing P6-lite seam then resolves `w.Ping()` via `owner_lookup("Wrap","Ping")` → Exact;
the seam labels it `EmbeddedPromotion` when `(recv_ty,name) ∈ promoted_method_keys`. No change to
`resolve_dispatch` for embedding (it is owner-index completeness, not interface dispatch).

`method_owners[fid]` stays the defining type. **Incremental:** aliases are fully recomputed from
all files (§10), so the `remove_files`-keys-on-`fid.file`-only stale-alias hazard (codex GAPS,
claude) cannot occur.

## 5. Mechanism B — Go interface (signature-confirmed multi-implementer dispatch)

`CallGraph` gains `interface_impls: BTreeMap<(String /*iface*/, String /*method*/), Vec<FunctionId>>`,
precomputed in `CallGraph::build`:

1. `let go = GoTypeProvider::from_parsed_files(files)` with **upgraded** signature-confirmed
   satisfaction (§6) + receiver-kind-aware method sets (§7).
2. `let live = live_types::collect_live_types(files, &∅)` (pointer-aware, §7).
3. For each interface `I` and method `m ∈ I`: `v = go.resolve_dispatch(I, m, &live)` (the existing
   impl, go.rs:624 — concrete branch unused here, interface branch with per-interface RTA
   fallback). **Store only if `!v.is_empty()`** (codex #2).

**Resolution seam** (resolution.rs:412-424), on `owner_lookup(recv_ty, name)` **miss**:

```rust
None => {
    let key = (normalize_go_key(recv_ty), name.to_string());      // bare-name, §9
    match self.interface_impls.get(&key) {
        Some(ids) if !ids.is_empty() =>
            ResolutionOutcome::hit(exact(ids.iter().collect(), ResolutionKind::InterfaceDispatch)),
        _ => ResolutionOutcome::dropped(DropReason::ExternalReceiver),  // explicit, codex #2
    }
}
```

`exact(...)` already accepts N callees (used by R1 trait dual-key; resolution.rs:167-178) and
`hit` over an N-set composes with Step-5 materialization (one `Call(Exact)` edge per callee,
build.rs:362) and nav re-resolution (call_resolve.rs:15-26) — verified by both reviews. Step-9 CHA
is `type_db`-gated (C++) and never runs for Go — no interaction.

## 6. Canonical signature (the Option-B core; codex #8)

Today `extract_method_signature` (interface decls, go.rs:356) and `extract_func_signature`
(concrete methods, go.rs:436) emit **raw node text including parameter names**
(`"(p []byte) -> (int, error)"` vs `"(b []byte) -> (int, error)"`) — non-comparable, hence the
name-only fallback. rev 2 adds **one** `canonical_sig(params_node, result_node) -> String` applied
**identically** to both, comparing **types only**:

- **Drop names.** Keep the ordered list of parameter *types* and result *types*; param/result names
  are dropped.
- **Expand grouped params.** `F(a, b int)` → `[int, int]`; `F(a int, b string)` → `[int, string]`.
- **Variadic.** `...T` is canonicalized to a distinct `...T` token (a method with `...int` does
  **not** satisfy an interface method typed `[]int`, per Go).
- **Unnamed params.** `F(int, error)` already type-only — same canonical form as named.
- **Receiver excluded.** The method receiver is not part of the comparable signature.
- **Returns included.** Result types participate (`(int, error)`); zero/one/many normalized to an
  ordered type list; named results dropped to types.
- **Type canonicalization.** Strip whitespace; preserve structural type forms (`*T`, `[]T`,
  `map[K]V`, `chan T`, `func(...)...`, `interface{...}`); **bare-name** the leading identifier of
  named/qualified types (`pkg.T` → `T`, §9). Canonical string e.g. `([]byte)->(int,error)`.

`compute_satisfaction` (go.rs:455) changes: `T` satisfies `I` iff for every `m ∈ M_I` there is a
method named `m` on `T` (per §7 method set) whose `canonical_sig` **equals** `I`'s `m`
`canonical_sig`. (Map values become canonical sigs; matching moves from `.keys()` to keys+values.)

**Soundness:** signature confirmation removes the name-collision false-satisfier class (codex #1),
making the `Exact` label honest. Residual over-approx (bare-name type collision across packages) is
noted (§14) and measured by the precision gate (§13).

## 7. Receiver-kind correctness (claude §4 / codex #1)

Go method sets are asymmetric: a value `T`'s set has only value-receiver methods; `*T`'s set has
both. `GoMethod.is_pointer_receiver` is already tracked (go.rs:60). rev 2:

- **Two method sets per type** in satisfaction: `set_value(T)` = `{m : !is_pointer_receiver}`;
  `set_ptr(T)` = all of `T`'s methods (+ promoted, §4). `T` satisfies `I` via `set_value(T)`;
  `*T` satisfies via `set_ptr(T)`.
- **Pointer-aware RTA.** Today `scan_go` (live_types.rs:158) strips `&` so `&T{}` and `T{}` both
  register as `T` — losing receiver kind. rev 2 (Go-scoped change): record `T` for value
  composite literals and `*T` for `&T{}` / `new(T)` so liveness aligns with the satisfying receiver
  kind. The **dispatch target FunctionId is identical** for `T`/`*T` (same method body); receiver
  kind only governs *whether* the type is admitted as a satisfier.
- A type whose only satisfying receiver kind is never instantiated is pruned by RTA (the §5 fallback
  still applies per-interface).

This closes the "value type falsely satisfies a pointer-method interface as Exact" hole.

## 8. Confidence + EFT reconciliation

Both mechanisms mint **Exact**, now *earned*:

- **Embedding** — a deterministic language rule; unconditionally Exact.
- **Interface** — Exact **because** satisfaction is signature-confirmed (§6) + receiver-kind-aware
  (§7). The fan-out to N live satisfiers is N **provably-possible** targets — sound may-analysis,
  the same justification as Step-9 CHA (build.rs:569). This is *not* the NameOnly→Exact laundering
  EFT guards against; it is type confirmation.
- Interface edges therefore legitimately enter `ExactOnly` slices (barrier/vertical/threed/spiral)
  — the recall win lands there too. **Risk (claude MAJOR):** a wide interface (e.g. `error`) has
  many live satisfiers → a large *correct* Exact fan-out. This is sound but can be noisy; rev 2
  does **not** cap it (a cap would drop real targets), and instead **measures** it via the
  multi-implementer precision gate (§13) and a multi-implementer barrier-slice precision test
  (§12). A fan-out-width telemetry/threshold is a noted future lever (§14).

## 9. Wiring & data structures

| Item | Location | Change |
|---|---|---|
| `interface_impls`, `promoted_method_keys` | call_graph.rs:47 | add `#[serde(default)] pub interface_impls: BTreeMap<(String,String),Vec<FunctionId>>` and `#[serde(default)] pub promoted_method_keys: BTreeSet<(String,String)>`; init in `empty()`/`build()`/`build_skeleton()`/`build_direct_subset()`. |
| Consume provider | call_graph.rs:206 (`build`) | construct `GoTypeProvider::from_parsed_files(files)`; write promoted aliases (§4) + precompute `interface_impls` (§5). |
| Signature engine | src/type_providers/go.rs | add `canonical_sig`; rewrite the two extractors to delegate; upgrade `compute_satisfaction` (§6); add receiver-kind sets (§7); expose `promoted_methods(type)`. |
| Pointer-aware RTA | src/live_types.rs `scan_go` | record `T` vs `*T` (§7). |
| Resolution consult | resolution.rs:412-424 | embedding label via `promoted_method_keys`; interface consult via `interface_impls` with explicit empty→drop (§5). |
| Key normalization | resolution.rs (new `normalize_go_key`) + go.rs extraction | bare-name (strip pointer + `pkg.`) applied to interface/type keys AND recovered receiver types, so both sides agree (codex #6). |
| Resolution kinds | resolution.rs `ResolutionKind` | add `EmbeddedPromotion`, `InterfaceDispatch` + their `as_str()` arms (the only exhaustive match; `call-stats` histogram auto-updates — codex/claude #7). |

No `CpgEdge`, `query.rs`, or nav `--confidence` change — interface edges are ordinary `Call(Exact)`
and inherit EFT machinery.

## 10. Cache & build-order interactions

- **No `CACHE_VERSION` bump.** `CpgEdge` unchanged; new `CallGraph` fields `#[serde(default)]`.
- **GIT_SHA covers the resolver change** (cpg_cache.rs:297, `v7`; `wrong_git_sha_misses` test names
  Phase-IP as the use case). Nav cache delegates to `cpg_cache::load_cache` (navigation/cache.rs:45)
  → transitively GIT_SHA-keyed.
- **Incremental = full recompute (codex #3, claude).** Embedding/satisfaction/RTA are
  whole-program. `build_direct_subset` sees only changed files (call_graph.rs:777), so Go dispatch
  must **not** be computed from the subset. In the merge path (`build_incremental`, build.rs:166),
  after CG merge, **recompute `interface_impls` + promoted aliases from the full merged file set**
  via `GoTypeProvider::from_parsed_files(all_files)` (idempotent; cheap vs parse). This also removes
  the `remove_files` stale-alias hazard (fid.file-only removal can't strand a promoted alias).

## 11. Failure modes

| Mode | Behavior |
|---|---|
| Interface, 0 live satisfiers (live set non-empty) | per-interface fallback → full satisfier set (go.rs:653); not dropped (avoids codex #7 FN). |
| Interface, satisfier set empty (no in-repo implementer) | `interface_impls` has no entry → explicit `ExternalReceiver` drop (codex #2). |
| Embedded method ambiguous at equal depth | not promoted (§4.2) → falls through to existing R6/drop. |
| Value type, pointer-receiver interface method | not admitted via `set_value` (§7); admitted only if `*T` is live. |
| Same name, different canonical signature | excluded by §6 (no false Exact). |
| Bare-name type collision across packages | admitted (over-approx); measured by precision gate (§13); precise keys deferred (§14). |
| Warm cache from pre-IP binary | rejected by GIT_SHA; rebuilt. |
| Non-Go repos | maps empty; zero behavior change (regression guard §12). |

## 12. Testing & acceptance

**Unit / fixture (pre-commit, no LSP):**
1. **Embedding** (tests/lang/go/): (a) `w.Ping()` → `Base::Ping`, kind `EmbeddedPromotion`, Exact;
   (b) transitive `A→B→C`; (c) **equal-depth ambiguity** → dropped; (d) direct method shadows
   promoted (`S.m` wins over embedded `E.m`).
2. **Interface** (tests/lang/go/): (a) `r.Go()` → `Fast::Go`, kind `InterfaceDispatch`, Exact;
   (b) multi-implementer (`Fast`+`Slow`) → both; (c) RTA: `live={Fast}`, `Slow` uninstantiated →
   only `Fast`; (d) per-interface empty-intersection fallback → full set.
3. **Signature confirmation (§6):** (a) param-name-only difference still satisfies; (b) grouped
   params `F(a,b int)` vs `F(int,int)` satisfy; (c) variadic `...int` ≠ `[]int` → no satisfy;
   (d) **return-type mismatch → no satisfy** (the name-only-era false positive); (e) arity mismatch
   → no satisfy.
4. **Receiver kind (§7):** value type with a pointer-receiver interface method does **not** satisfy
   unless `*T` is live.
5. **Qualified interface (codex #6):** a cross-file `pkg.Reader`-shaped receiver resolves via
   bare-name normalization (or documents the bound).
6. **Capability matrix flip:** `eval/fixtures/go/{embedded_method,interface_dispatch}/expected.toml`
   `status: known_fail → pass`; update rationale comments; matrix asserts both `ok`.
7. **Multi-implementer precision (claude MAJOR):** a barrier-slice fixture seeded at an interface
   method with several live implementers asserts the Exact fan-out is exactly the satisfiers (no
   non-satisfier leakage into the `ExactOnly` BFS).
8. **Non-Go regression:** Rust/Python/C resolution byte-identical (no spurious `interface_impls`).
9. **Cache round-trip:** new fields survive serialize/deserialize; cross-GIT_SHA cache rejected.

**Repo workflow (CLAUDE.md):** `cargo fmt`; full `cargo test` (+ `--features mcp`);
`cargo build --release` then `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (both Go
gaps `ok`, no other flips) then `--quick`.

**Success metric:** the two Go capability cases flip `expected_gap → ok`; caddy nav-callers recall
measurably lifts (recorded in the human-triggered re-baseline §13, not a hardcoded gate); **no
regression** — EFT `target-c-method` exact P=R=1.0, default `flip_candidate`; prism/tokio/flask/
click matrix + quick unchanged.

## 13. Human-triggered acceptance (not auto-run)

1. **Full 5-corpus rerun** (`uv run tier-a --corpus all`) — measures recall lift and the interface
   fan-out's precision effect.
2. **Multi-implementer precision gate:** caddy **U-method callers precision must not drop below the
   0.81 baseline** (baseline.md:35) — the guard for admitting Exact interface fan-out into
   `ExactOnly` (claude MAJOR). If it drops, fall back to the §14 fan-out-width lever before
   re-baselining.
3. **Re-adjudicate the 57 EFT-ambiguous caddy interface sites** — now resolved; verdicts re-anchor
   durably via the fingerprint store (pre-IP #2). Dual-adjudicator, record κ.
4. **caddy anchor re-baseline** — deliberate, with the adjudication record (like S2/EFT).

## 14. Out of scope / deferred

- **Python inheritance** (`python/inherited_override`) — next increment, same consume-the-provider
  pattern (`PythonTypeProvider` already registered).
- **`python/from_import_alias`** — import-map alias resolution, not dispatch.
- **Rust S3.1 — struct-field / return-typed receiver** (`s3-deferred §3`) — the prism C-method
  recall lever (0.121); gated/deferred.
- **Precise cross-package canonical type keys** — rev 2 uses bare names (over-approx on cross-pkg
  collisions); promote to package-qualified keys if the §13 precision gate shows material FPs.
- **Fan-out-width lever** — telemetry + optional threshold-demote for very wide interfaces
  (`error`); added only if §13 precision regresses (Option B's intent is uncapped honest Exact).
- **`DispatchProvider` as the algorithm-facing API** — the registered provider's `resolve_dispatch`
  for slice algorithms with `CpgContext.live_types` (its documented purpose) — distinct from the
  resolver-internal consumption this spec adds.
