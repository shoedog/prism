# Phase-IP — Go Struct Embedding Method Promotion — Design

**Date:** 2026-06-15 · **Status:** rev 1 — **split** from the combined Phase-IP dispatch spec
(`2026-06-14-prism-phase-ip-type-confirmed-dispatch-design.md`) after three review rounds. Owner
decision (2026-06-15): **ship embedding first** as the clean, low-risk half; Go *interface* dispatch
becomes its own deferred spec. This spec folds only the **embedding-relevant** round-1/2/3 findings;
all interface findings stay with the interface spec.

Builds on **EFT** (`e7a37e5`: confidence-tagged `CpgEdge::Call/Return`, exact traversal) + **pre-IP**
(`710cb86`: cache GIT_SHA `v7`). Closes the embedding half of `s3-deferred §1`/`§2`.

## 0. Why now & scope

EFT won precision; this is the **recall** counterpart, and embedding is its clean half: Go struct
embedding is a **deterministic** language rule, so promoted-method edges are honestly **Exact** — no
satisfaction guessing, no RTA, no fan-out. It flips one owner-approved `expected_gap`:

- **`go/embedded_method`** — `func run(w Wrap) { w.Ping() }`, `Wrap` embeds `Base`, `Ping` defined on
  `Base`. P6-lite recovers `w: Wrap` (typed param — confirmed recoverable, ast.rs:351-359), but
  `owner_lookup("Wrap","Ping")` misses because `Ping` is keyed under its defining type `Base` →
  `ExternalReceiver` drop (resolution.rs:422). Per `s3-deferred §2` this is stricter than the dumb R6
  path. Embedding promotion completes the owner index so the existing seam resolves it.

**In scope:** promotion of **concrete** methods from **embedded struct** fields (transitive),
resolved at the existing P6-lite owner-lookup seam, minted **Exact**.

**Out of scope → the interface spec** (`…-type-confirmed-dispatch-design.md`): all interface
dispatch, **embedded-interface** promotion (`type S struct { io.Reader }` — promoting an *interface*'s
method set is interface dispatch), signature canonicalization, RTA/liveness, multi-implementer
fan-out, the caddy interface acceptance gate. **Also deferred:** Python, `from_import_alias`, Rust
S3.1, generic structs (recorded gap, §7).

## 1. The decisive constraint

Success signal = **caddy nav recall** (prism `nav callers/callees` vs gopls). Nav and CPG resolve
through the **same ladder** `resolve_call_site`; nav re-resolves on `cg.call_graph` and never reads
CPG edges (EFT §0, verified all three rounds). Embedding flows through `owner_lookup` **inside**
`resolve_call_site`, so nav, CPG, and the metric all get it from one path. No build-time post-pass.

## 2. Architecture — consume the existing GoTypeProvider's promotion at CallGraph build

Build order (verified): `CallGraph::build` (build.rs:136) runs **before** the `TypeRegistry`
(context.rs:63-65); `resolve_call_site` materializes edges in `assemble_graph` Step 5 and is the nav
query path → it cannot reach the registry. **But the provider is registry-independent**
(`GoTypeProvider::from_parsed_files(files)`, go.rs:102), and it already computes transitive struct
embedding (`collect_promoted_methods_from`, go.rs:529, direct-wins via `or_insert`). So
`CallGraph::build` constructs the provider, reads promoted **concrete** method sets, and writes
owner-index aliases that serialize with `CallGraph` and that `resolve_call_site` reads. No new
`src/dispatch/` module. The registry's registered provider is untouched.

## 3. Decisions (owner-locked; rev 1)

| Decision | Choice |
|---|---|
| Scope | Concrete-method promotion from embedded **struct** fields only. Embedded interfaces + all interface dispatch → deferred (§7); generic structs resolve via the `[…]` strip (§6), not deferred. |
| Confidence | **Exact** — deterministic Go promotion rule; enters EFT `ExactOnly` slices honestly. |
| Source | Reuse `GoTypeProvider` (registry-independent) via a new public `promoted_struct_methods() -> Vec<PromotedMethod>` helper (codex r3-BLOCKER3). No reinvented logic. |
| Storage | `CallGraph.promoted_aliases: BTreeMap<(owner_key, method), Vec<FunctionId>>` (carries the alias `FunctionId`s for clean incremental replace; its key set is the `EmbeddedPromotion` label set) + `embedding_gaps: BTreeMap<String,usize>`; both `serde(default)`. Aliases are also written into the existing `methods` map. |
| Precedence | Direct method on `S` wins over a promoted `m` (alias only when `S` has no direct `m`). Equal-depth embedded ambiguity → **not** promoted (drop). |
| Keys | `normalize_go_struct_key` = `owner_key` + strip Go `[…]` generic args (`Wrap[T]`→`Wrap`), for owner keys AND the recovered receiver (**Go-gated** in `recover_receiver`; non-Go keeps `owner_key`). Cross-package `pkg.` normalization is deferred to the interface spec. No admission/pointer-preservation complexity (interface-side). |
| Addressability | Promotion applies to **recovered addressable** receivers (typed params/locals); a value selector may call a pointer-receiver promoted method (Go auto-addresses). Non-addressable bases (temporaries, map index) → not in scope (codex r3-MINOR12). |
| Resolution kind | `EmbeddedPromotion` (telemetry; → `Exact` via `exact()`; kind is not serialized — only `as_str`). |
| Gap contract | A `call-stats` `embedding_gaps` counter for equal-depth **ambiguity** drops (codex r3-MAJOR8, minimal form). Embedded **interface** fields are skipped in `promoted_struct_methods` (no counter); generic structs are **resolved** via the `[…]` strip (not gapped). |
| Cache | **Bump `CACHE_VERSION` v7→v8** — bincode does **not** honor `serde(default)` for the new trailing `CallGraph` field; the version bump (not GIT_SHA alone) is the format-safety mechanism (claude r3-B2). |
| Build paths | Promotion is whole-program: full-repo recompute + **replace-not-merge** across `build`/`build_incremental`/`build_scoped` (§9). |

## 4. Mechanism — owner-index dual-key promotion

Embedding promotes the embedded type's concrete method set onto the outer type — **statically and
unconditionally** — structurally identical to the trait dual-key already in the index (`methods:
(owner_key, method) → [FunctionId]`, "trait impls dual-keyed", call_graph.rs:61-64).

1. **Provider helper (new, public):** `GoTypeProvider::promoted_struct_methods() -> Vec<PromotedMethod>`
   (where `PromotedMethod { struct_name, method, func_id, depth }`) — walks each struct's transitive embedded **struct** closure
   (`collect_promoted_methods_from`, go.rs:529) and returns **every** promoted concrete method
   (including duplicates of the same name reached via different embed paths, each with its depth) so
   `CallGraph::build` can apply direct-wins + equal-depth ambiguity. Each `FunctionId` is built from
   the `GoMethod` file/start/end (go.rs:399-400, identical to `node_line_range` so it matches the
   CallGraph function node, ast.rs:2640). **Embedded interface fields are skipped** (their methods have
   no concrete body — interface dispatch, deferred).
2. **CallGraph::build:** for each struct `S`, for each `(m, fid)` from `promoted_methods(S)`:
   - **Direct-wins:** skip if `S` directly defines `m` (the provider's `resolve_all_concrete_methods`
     already prefers direct via `or_insert`; the alias mirrors that).
   - **Equal-depth ambiguity:** if two embedded paths provide `m` at the same shallowest depth →
     **drop** (no alias; count a gap). (The provider's `or_insert` does not detect this — the helper
     returns depth so `CallGraph::build` can apply the rule.)
   - Else insert `methods[(normalize_go_struct_key(S), m)] += fid` and record it in
     `promoted_aliases[(normalize_go_struct_key(S), m)] += fid`.
3. `method_owners[fid]` stays the **defining** type (`Base`) — only the lookup key is aliased.

Resolution (resolution.rs:412-424): `owner_lookup(normalize_go_struct_key(recv_ty), name)` now hits
the promoted alias → `Exact`, labeled `EmbeddedPromotion` when `(norm(recv_ty), name) ∈
promoted_aliases`. The promoted method resolves to a **single** `FunctionId` (no fan-out) — so
Step-5b interprocedural arg binding (build.rs:382-485) binds one callee, no DataFlow multiplication
(the interface-side concern, deferred).

## 5. Receiver recovery & addressability

P6-lite already recovers typed-param struct receivers (`func run(w Wrap)`, ast.rs:351-359) and
constructor-locals — which covers the fixture and the common case. The promoted method's `FunctionId`
is identical regardless of receiver kind; the only receiver-kind question is **addressability**: Go
lets a value selector `w.Ping()` call a *pointer-receiver* promoted method when `w` is addressable
(params/locals are). rev 1 therefore includes both value- and pointer-receiver promoted methods for
recovered addressable receivers, and does **not** attempt promotion for non-addressable bases
(temporaries, map-index, call-result) — those still drop upstream (out of scope; codex r3-MINOR12).
(The stricter value-vs-pointer **method-set split** is an interface-*satisfaction* concern, deferred.)

## 6. Key normalization (codex r3-BLOCKER1, embedding subset)

The combined spec's "one normalizer for everything" was wrong because interface admission keys must
*preserve* `T`/`*T`. Embedding has no admission keys — it keys by **bare struct name** only. So
embedding defines a single, simple `normalize_go_struct_key(s)`:

- `owner_key` (strips `*`/`&`/`<…>`) then strip Go `[…]` generic args (`Wrap[T]`→`Wrap`), so generic
  structs **resolve** (name-only embedding needs no signature matching). Cross-package `pkg.`
  normalization is deferred to the interface spec.
- Applied at owner-key construction (extraction) **and** to the recovered receiver in
  `recover_receiver`, **Go-gated** (the seam stays language-blind for non-Go: a non-Go receiver keeps
  `owner_key`; a Rust receiver never matches `promoted_aliases`, which is Go-only → no behavior
  change, by emptiness not gating).

The interface-side key contracts (admission `T`/`*T`, dispatchable-interface, generic-gap) are defined
in the interface spec.

## 7. Gap contract (codex r3-MAJOR8, minimal form)

- **Equal-depth ambiguity** — not promoted; counted in `embedding_gaps["ambiguous"]`, surfaced in
  `call-stats` (§10). This is the one meaningful, hard-to-otherwise-observe gap.
- **Embedded interface fields** (`type S struct { io.Reader }`) — skipped in `promoted_struct_methods`
  (interface dispatch, deferred); no counter (it is simply not this increment's job).
- **Generic structs** (`type S[T any] struct{…}`) — **resolved**, not gapped: `normalize_go_struct_key`
  strips `[…]` so name-only embedding promotion works; no special handling needed.

No false aliases are ever created.

## 8. Failure modes

| Mode | Behavior |
|---|---|
| `S` directly defines `m` and embeds `E.m` | direct wins; no alias. |
| Two embeds provide `m` at equal shallowest depth | dropped (gap counter); receiver falls through to R6/drop. |
| Embedded interface field | not promoted (skipped in `promoted_struct_methods`); deferred to interface spec. |
| Generic struct | resolved via `normalize_go_struct_key` `[…]` strip (§6). |
| Non-addressable selector base | not promoted (out of scope); drops upstream. |
| Warm cache from pre-IP / v7 | rejected by `CACHE_VERSION` v8 (§9). |
| Non-Go repos | `promoted_aliases` empty; zero behavior change (regression guard §10). |

## 9. Cache & build paths

- **`CACHE_VERSION` v7→v8.** `CallGraph` is bincode-serialized whole inside `SerializedCpg`
  (cpg_cache.rs:89,183,251); bincode is non-self-describing and ignores `serde(default)` for missing
  trailing fields, so the new `promoted_aliases`/`embedding_gaps` fields require a version bump — that,
  not GIT_SHA alone, is the format-safety mechanism (claude r3-B2). GIT_SHA still covers the resolver
  change.
- **Whole-program, replace-not-merge** (codex r3-MAJOR7 / claude r3-B5):
  - `build` (build.rs:136): promotion over all `files`.
  - `build_incremental` (build.rs:166): after CG `merge`, **clear** `promoted_aliases` and remove
    those aliases from `methods`, then recompute from `from_parsed_files(all merged files)` —
    `remove_files` prunes `methods` by `fid.file` only (call_graph.rs:736-740), so a promoted alias
    whose `fid` lives in an unchanged file would otherwise survive stale.
  - `build_scoped` (context.rs:135): promotion computed over the **full** repo file set (a struct's
    embed may reference an out-of-scope file). The CPG **edge** for a promoted method whose defining
    type is outside the scoped node set is **best-effort** in scoped mode (the node may not exist —
    codex r3-BLOCKER2); **nav re-resolves on the full owner index regardless**, so the caddy metric
    and `prism nav` are correct in all build modes; only diff-scoped CPG *slice* edges are best-effort
    (mirroring the existing scoped indirect-call limitation, build.rs:160-165). Requires threading the
    full `files` into the Go-promotion precompute (not just the scoped `&filtered`).
- **Provider built twice** (CallGraph + registry context.rs:253) — same pure `from_parsed_files`, no
  divergence; embedding extraction is cheap vs parse. Accepted; sharing `Arc<GoTypeData>` is a deferred
  perf opt (build order makes it non-trivial).

## 10. Testing & acceptance (pre-commit, no LSP)

1. **Embedding** (tests/lang/go/): (a) `w.Ping()` → `Base::Ping`, kind `EmbeddedPromotion`, Exact;
   (b) transitive `A→B→C`; (c) equal-depth ambiguity → dropped (gap counter); (d) direct method
   shadows promoted; (e) value selector of an embedded *pointer-receiver* method on an addressable
   param resolves; (f) embedded **interface** field → NOT promoted (skipped; no false edge).
2. **Capability matrix flip:** `eval/fixtures/go/embedded_method/expected.toml` `known_fail → pass`;
   update rationale; matrix asserts `ok`. (`go/interface_dispatch` stays `known_fail` — interface
   spec.)
3. **Key normalization:** a `pkg.Wrap`-shaped / `*Wrap` recovered receiver resolves via
   `normalize_go_struct_key`.
4. **Non-Go regression:** Rust/Python/C resolution byte-identical (no spurious promotions).
5. **Cache:** v8 round-trips; a v7 / cross-GIT_SHA cache is rejected; incremental **replace** drops a
   promoted alias whose embedding was removed (no phantom).

**Repo workflow (CLAUDE.md):** `cargo fmt`; full `cargo test` (+`--features mcp`); `cargo build
--release` then `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (`go/embedded_method`
`ok`, no other flips) then `--quick`.

**Success metric:** `go/embedded_method` flips `expected_gap → ok`; caddy embedded-method-caller
recall measurably lifts (recorded in the §11 re-baseline); **no regression** — EFT `target-c-method`
exact P=R=1.0; prism/tokio/flask/click matrix + quick unchanged.

## 11. Human-triggered acceptance (not auto-run)

1. **Full 5-corpus rerun** (`uv run tier-a --corpus all`) — measures the embedded-method-caller
   recall lift.
2. **caddy re-baseline** if the lift is material — deliberate, with an adjudication record (like
   S2/EFT). Embedding edges are single-target Exact (no fan-out), so there is **no interface
   precision gate** here — that belongs to the interface spec.

## 12. Out of scope / deferred (→ Go interface dispatch spec + later)

- **All Go interface dispatch** — satisfaction (signature canonicalization), RTA/liveness,
  multi-implementer fan-out, the caddy interface acceptance gate + harness FP attribution, anonymous-
  interface + generic-interface gaps, and the scope decision (in-scope typed-param receivers vs
  expanding P6-lite to type-assertion/`var` receivers to own caddy's interface recall). These carry
  nearly all of the round-3 findings; see the interface spec's rev-4 work-list.
- **Embedded-interface promotion** — promoting an embedded *interface*'s method set is interface
  dispatch.
- **Python inheritance, `from_import_alias`, Rust S3.1.**
  (Generic structs are **not** deferred — they resolve via the `[…]` strip, §6.)
