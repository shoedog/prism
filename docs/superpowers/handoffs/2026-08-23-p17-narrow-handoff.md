# Handoff — P17 narrow concrete-receiver routing, fix wave 5

**Written:** 2026-08-23T20:55:12Z · **By:** Codex `/root`
**Workspace:** `/Users/wesleyjinks/code/slicing-p17-narrow` · branch `p17-narrow-concrete-receiver`
**Measured implementation HEAD before this handoff:** `ea3e72b` · base `514cfe3`
**Cache schema:** CPG `47`, navigation sidecar `16`

## 0. Final status and bounded rule

- Fix wave 5 closes the final p17d oracle item. The four Prometheus `cl.Close()` sites now terminally drop with `DropReason::ExternalReceiver`, telemetry `go_external_receiver_new_recovery_drop=1` per site, manifest route `unproven_drop`, and fanout 0.
- The guard is limited to Go function-literal parameter recovery added by P17 wave 2. Main deliberately treated that parameter as unrecoverable. A serialized `CallSite.receiver_newly_recovered` bit preserves that provenance across cold, exact-CPG, and sidecar paths.
- The shared `go_concrete_receiver_route` returns `ExternalNewRecoveryDrop` only when that bit is set and the recovered type is qualified through an import that cannot be proven in the effective in-repo module set. With an active module graph, absence of an exact `@go-import:` key proves externality; without module identity, an in-repo basename keeps the site out of this terminal route.
- Pre-existing external recovery is unchanged. The pole with closure-local `var cl io.Closer; cl.Close()` still reaches the legacy bare-name ladder and retains its false-but-pre-existing Exact edge. No R3 ladder code changed.
- The wave-5 rule (ratified by the owner via design v9's amendment — see the spec's Status paragraph; the controller ledger records the decision) supersedes the wave-4 handoff's unresolved note for these four sites. Earlier R2 collision, concrete direct, promotion, S4, P5, alias, rebinding, cache, and module-identity behavior remains green.
- No subagents were used. No push or gopls oracle was run.

## 1. Fix-wave-5 commits

| Commit | Result |
|---|---|
| `96c1a9a` — `p17-fix5: add external new-recovery poles` | Red-first resolver/manifest poles. The new function-literal external receiver minted the decoy `Closer.Close`; in-repo and pre-existing external controls pinned unchanged output. |
| `ea3e72b` — `p17-fix5: drop external new-recovery receivers` | Adds serialized provenance, shared terminal routing, telemetry, resolver/manifest parity, schema/default coverage, and four-path cache parity. |

The handoff commit follows these two commits. Cache versions remain exactly 47/16: this is part of the single unmerged P17 schema transition, not a second wave transition.

## 2. Hypothesis / probe / result log

### Root cause and provenance discriminator

- H1: the post-build receiver rematerialization boundary could distinguish new recovery because the old CallSite had no `receiver_type`. Alternative H2: the expanded receiver classifier already recovered the syntax during initial extraction. The first green attempt was falsified: both closure-local `var` poles had `receiver_type=Some(...)` and `receiver_newly_recovered=false` before rematerialization.
- The main-to-branch source diff discriminated the alternatives. Main's `func_literal` parameter branch incremented the binding count and set `found=None`; P17 wave 2 changed that exact branch to `found=Some((declared_type, TypedParam))`. Closure-local `var` recovery was unchanged from main.
- The final provenance helper therefore recognizes a receiver bound by a function-literal parameter containing the call. The new external and new in-repo poles carry the bit; the closure-local `var io.Closer` control does not.

### External import proof

- H1: real module-aware corpora preserve exact effective import-path keys for every loaded in-repo package, while standard-library/dependency imports have no exact key. Alternative H2: exact keys are absent in all builds and only basenames work. Existing module-graph tests plus the five corpora discriminate H1: active module counts and exact in-repo keys are populated, while `io` remains absent.
- For module-unaware synthetic graphs, the fallback is deliberately conservative: a loaded package with the import basename prevents the external classification. This keeps the new in-repo control on S4 while `io.Closer` with no loaded `io` package terminally drops.

### Red / green behavior

- Red: function-literal parameter `cl io.Closer` resolved Exact to `decoy/closer.go`, with legacy fallback telemetry sites/hits/edges `1/1/1`.
- Green: the same site has zero targets, `ExternalReceiver`, new telemetry 1, legacy fallback telemetry `0/0/0`, and manifest `unproven_drop`/fanout 0.
- Negative controls: a function-literal parameter typed as the in-repo interface `Local` still resolves one Exact `InterfaceDispatch`; closure-local `var cl io.Closer` still resolves the legacy decoy with fallback telemetry `1/1/1` and new telemetry 0.

## 3. Tests and build gates

### Named wave-5 poles

- `newly_recovered_qualified_external_receiver_drops_before_bare_ladder`
- `newly_recovered_in_repo_interface_keeps_interface_dispatch`
- `preexisting_recovery_external_receiver_keeps_legacy_bare_output`
- `concrete_receiver_outputs_match_no_cache_cold_create_exact_cpg_and_sidecar_hits` (extended with route/fanout/telemetry assertions for the external site)

Focused results: wave-5 resolver/manifest poles **3 passed / 0 failed / 0 ignored**; complete Go target **192/0/0**; selected CPG-cache tests **63/0/0**; selected navigation/cache tests **16/0/0**.

| Gate | Final result |
|---|---|
| `cargo fmt --all -- --check` and `git diff --check` | passed |
| `cargo test` | **3,350 passed, 0 failed, 1 ignored** |
| `cargo build --release` | passed |
| `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut` immediately after release build | **104 passed, 0 failed**; `go/embedded_method` passed |
| gopls oracle | not run; controller-owned by instruction |

## 4. Prometheus manifest verification

Final artifact: `/private/tmp/p17-fix5-prometheus-manifest.json`.

| File:line | Receiver class | Final route | Fanout / implementers |
|---|---|---|---|
| `tsdb/head_append_v2_test.go:1508` | `typed_param` | `unproven_drop` | `0 / []` |
| `tsdb/head_append_v2_test.go:1517` | `typed_param` | `unproven_drop` | `0 / []` |
| `tsdb/head_test.go:4176` | `typed_param` | `unproven_drop` | `0 / []` |
| `tsdb/head_test.go:4185` | `typed_param` | `unproven_drop` | `0 / []` |

`HTTPResourceClient` is absent from all four records. After removing exactly these four keyed records, normalized wave-4 and wave-5 Prometheus manifests compare equal (`diff` exit 0). This proves every other manifest record is unchanged, including newly recovered external sites that already had zero legacy fanout.

## 5. Five-corpus call-stats

Candidate artifacts are `/private/tmp/p17-fix5-callstats-{ripgrep,caddy,prometheus,etcd,hugo}.txt`, generated by the final release binary with `prism nav --no-cache call-stats`. Same-base controls are controller-generated `ctrl514-*.txt` from main `514cfe3`. NLCF is `interface_overapprox.NonLocalConstructionFallback`.

### Delta from wave 4 (`0c63185`)

All direct Exact kinds, P5, NLCF, multi-target counts, concrete direct/promotion/no-selector telemetry, and collision-bail telemetry are unchanged. Values are wave 4 -> wave 5.

| Corpus | interface Exact | external drops | legacy R3 sites/hits/edges | new external telemetry | Edge-bearing change |
|---|---:|---:|---:|---:|---|
| ripgrep | `0 -> 0` | `615 -> 615` | `0/0/0 -> 0/0/0` | 0 | none; byte-identical |
| caddy | `1757 -> 1757` | `2663 -> 2663` | `2466/19/1339 -> 1998/19/1339` | 468 | none; 468 zero-hit attempts reclassified |
| prometheus | `2310 -> 2306` | `6830 -> 6834` | `5453/16/34 -> 4386/12/30` | 1067 | exactly four `InterfaceDispatch` Exact edges removed; 1,063 additional zero-hit attempts reclassified |
| etcd | `2426 -> 2426` | `8886 -> 8886` | `7152/89/113 -> 6398/89/113` | 754 | none; 754 zero-hit attempts reclassified |
| hugo | `667 -> 667` | `5056 -> 5056` | `4232/3/14 -> 3673/3/14` | 561 | none; 561 zero-hit attempts reclassified |

For Prometheus, `dropped_go_receiver.TypedParam` rises by exactly 4 and `kinds.interface_dispatch`/`kind_exact.interface_dispatch` each fall by exactly 4. No other kind changes. The broader sites-counter migration is expected because the bounded rule terminally classifies every newly recovered external function-literal parameter, including sites whose legacy ladder already had zero hits.

### Same-base main control to final

Values are `base -> final`.

| Corpus | interface | constructor | typed-param | return | field | embedded | P5 NameOnly | NLCF | multi-target |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ripgrep | `0 -> 0` | `314 -> 314` | `315 -> 315` | `1269 -> 1269` | `273 -> 273` | `0 -> 0` | `0 -> 0` | `0 -> 0` | `0 -> 0` |
| caddy | `1766 -> 1757` | `241 -> 258` | `107 -> 1978` | `233 -> 233` | `15 -> 16` | `84 -> 87` | `0 -> 0` | `3 -> 3` | `21 -> 21` |
| prometheus | `2461 -> 2306` | `978 -> 1472` | `770 -> 1522` | `2135 -> 2148` | `125 -> 182` | `58 -> 58` | `1 -> 1` | `143 -> 143` | `3855 -> 3794` |
| etcd | `2002 -> 2426` | `109 -> 118` | `230 -> 750` | `1064 -> 1593` | `38 -> 103` | `42 -> 54` | `0 -> 0` | `578 -> 578` | `515 -> 606` |
| hugo | `625 -> 667` | `193 -> 205` | `440 -> 1144` | `2927 -> 2952` | `92 -> 117` | `46 -> 52` | `0 -> 0` | `330 -> 330` | `2568 -> 2582` |

| Corpus | concrete direct | existing promoted | deferred promoted | no-selector | collision bails | external-new drops | final R3 sites/hits/edges |
|---|---:|---:|---:|---:|---:|---:|---:|
| caddy | 2477 | 7 | 0 | 433 | 20 | 468 | `1998/19/1339` |
| prometheus | 5106 | 19 | 0 | 97 | 188 | 1067 | `4386/12/30` |
| etcd | 2561 | 12 | 0 | 77 | 166 | 754 | `6398/89/113` |
| hugo | 4405 | 11 | 0 | 695 | 199 | 561 | `3673/3/14` |

Ripgrep is byte-identical to main: **3,019 bytes == 3,019 bytes** (`cmp` exit 0).

## 6. Per-file limit and retained artifacts

Every file newly added by P17 remains below the 600-line per-file limit. Final measured counts:

| File | Lines |
|---|---:|
| `src/go_concrete_receiver.rs` | 575 |
| `tests/lang/go/concrete_receiver_fix5_test.rs` | 147 |
| `tests/lang/go/concrete_receiver_fix4_test.rs` | 200 |
| `tests/lang/go/concrete_receiver_fix3_test.rs` | 192 |
| `tests/lang/go/concrete_receiver_fix2_test.rs` | 166 |
| `tests/lang/go/concrete_receiver_alias_test.rs` | 440 |
| `tests/lang/go/concrete_receiver_manifest_test.rs` | 367 |
| `tests/lang/go/concrete_receiver_route_test.rs` | 427 |
| `tests/lang/go/concrete_receiver_unproven_test.rs` | 246 |
| `tests/navigation/go_concrete_cache_test.rs` | 109 |

Retained evidence:

- `/private/tmp/p17-fix5-callstats-{ripgrep,caddy,prometheus,etcd,hugo}.txt`
- `/private/tmp/p17-fix5-prometheus-manifest.json`
- Wave-4 comparison controls: `/private/tmp/p17-fix4-final-callstats-*.txt` and `/private/tmp/p17-fix4-final-prometheus-manifest.json`

Not done: push and gopls oracle, by instruction. No known implementation blocker remains.

**COMPLETE:** the final four external-`io.Closer` false edges are removed, every other manifest record is preserved, all required tests/builds/corpora are recorded, and the branch is ready for controller review/push.
