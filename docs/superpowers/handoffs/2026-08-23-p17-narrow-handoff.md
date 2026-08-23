# Handoff — P17 narrow concrete-receiver routing, fix wave 2

**Written:** 2026-08-23T18:39:05Z · **By:** Codex `/root`
**Workspace:** `/Users/wesleyjinks/code/slicing-p17-narrow` · branch `p17-narrow-concrete-receiver`
**Measured implementation HEAD before this handoff:** `ba8a2be` · base `514cfe3`
**Cache schema:** CPG `49`, navigation sidecar `18`

## 0. Gating facts

- The authoritative design remains v8 (`5839378`): existing `apply_go_embedding_promotion` edges are preserved and only misses remain deferred. Fix wave 2 did not edit the spec.
- No subagents were used. No push was performed. The controller owns the push and the gopls oracle.
- Temporary audit logging and the temporary R3 manifest bit were removed. The final release binary was rebuilt afterward and passed Tier-A 104/104.
- Read-only evidence is retained under `/private/tmp`, including the base and wave-1 worktrees, final manifests, audit manifests, test logs, and five corpus call-stat captures.

## 1. Fix-wave commits

| Commit | Item | Result |
|---|---|---|
| `b7760b0` | `p17-fix2: fail closed on receiver value rebinding` | Applies the proof-shadow bail to type-switch, inner declaration, range, and closure-parameter rebindings; adds resolver/manifest poles; bumps cache schemas to 48/17. |
| `dc342cb` | `p17-fix2: resolve qualified owners by module import path` | Resolves qualified owners by exact effective-module import path before the legacy basename fallback; adds nested-module function-literal and factory-local poles; bumps schemas to 49/18. |
| `55b03e0` | `p17-fix2: pin closure rebinding fail-closed contract` | Updates the existing closure regression to assert retained legacy input, proof-shadow evidence, and zero edges. |
| `ba8a2be` | `p17-fix2: retain proofs only for lexical rebindings` | Separates true Go lexical rebindings from ordinary assignment, preserving the pre-wave assignment-only bail. |

## 2. Root-cause record

### A — Hugo type-switch rebinding

At `tpl/tplimpl/templates.go:205/:212`, `switch in := in.(type)` rebinds `in` in each case. The old recovery used the outer `tpl.Template` parameter for an on-demand R2 proof and minted two false interface implementers. The alternative considered was a manifest-only consumer error; resolver and manifest both consumed the same wrong outer proof, falsifying that alternative. The fix records a lexical value rebind on the `CallSite`; the shared route returns R3, and both final sites are `unproven_drop`, fanout 0.

The first broad suite found two adjacent representation contracts. A closure-param regression expected recovery erasure, while wave 2 intentionally retains the first type only as R3 input; it now asserts `receiver_local_type_shadowed` and zero edges. A second test used ordinary `r = x`, not a declaration rebind; the first implementation over-broadly retained that proof. `ba8a2be` adds a lexical-rebinding discriminator, so assignment-only duplicates keep the old `None` bail.

### B — Etcd qualified `integration.Cluster`

- H1 supplied by the controller: qualified lookup selected the same-bare interface and returned R2.
- H2 supplied by the controller: receiver recovery lost the qualifier and reached R3.
- Discriminating trace: `recv_ty="integration.Cluster"`, but `proven_owner=None` and `receiver_owner=None`. The qualifier was retained, falsifying H2; no R2 owner was selected, falsifying literal H1.
- Settled cause: import alias recovery preserved the qualified type, but owner lookup reduced the import to its last path segment. Etcd has both `tests/framework/integration` and `tests/integration`, so basename lookup was ambiguous and failed to R3; the unchanged R3 ladder then minted the same-bare `interfaces.Cluster` false edges.
- Fix: build exact `@go-import:<effective path>` keys once from the effective module graph, prefer that identity, and retain the legacy basename lookup only when exact evidence is unavailable. Ambiguous exact paths still fail closed.

Final Etcd routes: `revision_test.go:114/:126 Client` and `v3_failover_test.go:93 Endpoints` are `concrete_direct`, fanout 0.

## 3. Red/green and full gates

### Named focused tests

- `concrete_receiver_fix2_test::{value_rebindings_fail_closed_to_r3_without_outer_concrete_direct_edges,type_switch_interface_rebinding_uses_the_legacy_r3_fallback,single_receiver_bindings_keep_r1_and_r2_routes}`: **3 passed, 0 failed, 0 ignored**. Before `b7760b0`, the type-switch/inner/range poles could use the outer proof.
- `concrete_receiver_qualified_fix2_test::qualified_nested_module_receivers_resolve_exact_import_owner`: **1/0/0** covering both a function-literal parameter and a `pkg.NewX` local in a nested module. Before `dc342cb`, the literal-param pole reached R3 and minted two false interface edges.
- `call_graph::go_receiver_typing_tests::s2_closure_param_rebinding_base_inside_closure_fails_closed`: **1/0/0** with the final behavioral contract.
- `resolution_test::var_local_shadowed_binding_bails`: **1/0/0**, proving ordinary assignment did not inherit the new lexical-rebind behavior.
- Adjacent controls: owner-partition **17/0/0**, owner-partition fix-wave **30/0/0**, module-graph fix2 **5/0/0**, and cache-version pins all passed.

### Full gates

| Gate | Result |
|---|---|
| `cargo fmt` and `git diff --check` | passed |
| `cargo test` | **3,338 passed, 0 failed, 1 ignored** across 28 result groups |
| `cargo build --release` | passed after all temporary audit code was removed |
| `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut` immediately after the final release build | **104 passed, 0 failed**; `go/embedded_method` passed |
| gopls oracle | not run; controller-owned by explicit instruction |

The first broad run stopped at 713/1/0 on the stale closure representation assertion. After that bounded update, the second run reached 2,348/1/1 and exposed the assignment-only overreach. The final full run above is the retained acceptance result.

## 4. Required manifest checks and Etcd oracle classification

- Hugo `tpl/tplimpl/templates.go:205` and `:212`: `unproven_drop`, fanout 0, no implementer identities.
- Etcd `revision_test.go:114/:126` and `v3_failover_test.go:93`: `concrete_direct`, fanout 0, no implementer identities.

The controller's item C attributed the 11 cache sites to promoted methods of `*clientv3.Client`. The live input and its own oracle artifact contradict that attribution: every receiver is `c` from `cache.New(...) -> *cache.Cache`, Etcd declares `Cache.RequestProgress` at `cache/cache.go:109`, `Cache.Watch` at `:116`, and `Cache.Get` at `:192`, and the oracle records `gopls_satisfier` as `cache.Cache`. Exact import-path proof therefore correctly makes these R1(a), not R1(c). No #16/R1(c) site exists among these 14.

| File:line | Method | Final route | Classification |
|---|---|---|---|
| `cache_test.go:455` | Watch | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Watch` |
| `cache_test.go:514` | Watch | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Watch` |
| `cache_test.go:569` | Watch | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Watch` |
| `cache_test.go:586` | RequestProgress | `concrete_direct` | #17-fixed R1(a), `cache.Cache.RequestProgress` |
| `cache_test.go:618` | Watch | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Watch` |
| `cache_test.go:637` | Watch | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Watch` |
| `cache_test.go:659` | Watch | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Watch` |
| `cache_test.go:1383` | Get | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Get` |
| `cache_test.go:1453` | Watch | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Watch` |
| `cache_test.go:1507` | Watch | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Watch` |
| `cache_test.go:1559` | Get | `concrete_direct` | #17-fixed R1(a), `cache.Cache.Get` |
| `revision_test.go:114` | Client | `concrete_direct` | B, #17-fixed R1(a), `integration.Cluster.Client` |
| `revision_test.go:126` | Client | `concrete_direct` | B, #17-fixed R1(a), `integration.Cluster.Client` |
| `v3_failover_test.go:93` | Endpoints | `concrete_direct` | B, #17-fixed R1(a), `integration.Cluster.Endpoints` |

## 5. Same-base and wave-1 corpus evidence

All final values use the final release implementation with `prism nav --no-cache call-stats`. Controls are controller-provided `ctrl514-*.txt` from main `514cfe3`. Values are `base -> final (delta)`; missing kinds are zero.

| Corpus | interface Exact | constructor Exact | typed-param Exact | var/assert Exact | return Exact | field Exact | embedded Exact | P5 NameOnly | NLCF | multi-target |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| caddy | 1766 -> 1777 (+11) | 241 -> 258 (+17) | 107 -> 1978 (+1871) | 0/0 -> 0/0 | 233 -> 233 | 15 -> 16 (+1) | 84 -> 87 (+3) | 0 -> 0 | 3 -> 3 | 21 -> 21 |
| prometheus | 2461 -> 2752 (+291) | 978 -> 1472 (+494) | 770 -> 1522 (+752) | 0/0 -> 0/0 | 2135 -> 2144 (+9) | 125 -> 182 (+57) | 58 -> 58 | 1 -> 1 | 143 -> 143 | 3855 -> 3904 (+49) |
| etcd | 2002 -> 2528 (+526) | 109 -> 118 (+9) | 230 -> 750 (+520) | 0/0 -> 0/0 | 1064 -> 1593 (+529) | 38 -> 103 (+65) | 42 -> 54 (+12) | 0 -> 0 | 578 -> 578 | 515 -> 609 (+94) |
| hugo | 625 -> 756 (+131) | 193 -> 205 (+12) | 440 -> 1144 (+704) | 0/0 -> 0/0 | 2927 -> 2952 (+25) | 92 -> 117 (+25) | 46 -> 52 (+6) | 0 -> 0 | 330 -> 330 | 2568 -> 2583 (+15) |

Wave-1-to-wave-2 leaf values are shown as `final (delta from wave 1)`. The broad typed/return/field populations are the requested function-literal recovery and exact import-path proof paths, not an R3 ladder edit.

| Corpus | interface | constructor | typed | return | field | embedded | multi | direct | existing promoted | deferred | no-selector | R3 sites / hits / edges |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| caddy | 1777 (+1) | 258 (0) | 1978 (+6) | 233 (0) | 16 (+1) | 87 (0) | 21 (0) | 2477 (+7) | 7 (0) | 0 (0) | 433 (+2) | 2466 (+488) / 19 (0) / 1339 (0) |
| prometheus | 2752 (+458) | 1472 (+31) | 1522 (+173) | 2144 (+9) | 182 (+57) | 58 (0) | 3904 (+91) | 5102 (+226) | 19 (0) | 0 (0) | 97 (+6) | 5471 (+1176) / 32 (+25) / 50 (+43) |
| etcd | 2528 (+604) | 118 (+2) | 750 (+261) | 1593 (+529) | 103 (+65) | 54 (0) | 609 (+192) | 2561 (+858) | 12 (0) | 0 (0) | 77 (-12) | 7152 (+534) / 89 (-75) / 113 (-83) |
| hugo | 756 (+102) | 205 (+4) | 1144 (+552) | 2952 (+25) | 117 (+25) | 52 (0) | 2583 (+18) | 4405 (+594) | 11 (0) | 0 (0) | 695 (+7) | 4232 (+591) / 3 (-3) / 14 (+7) |

P5 and NLCF are unchanged from wave 1 in every corpus. Ripgrep is byte-identical: **3,019 base bytes == 3,019 final bytes**, and `cmp` returned 0.

## 6. R3 manifest parity

A temporary environment-gated bit derived directly from the one shared route verdict marked final `Unproven` sites. It was removed before the final build. Candidate records were matched to base by `(file,start_byte,end_byte,method)` and projected to legacy fields by deleting candidate-only diagnostics.

| Corpus | final proof-R3 sites | hits / edges | drops | matched base records | changed matched legacy records | new records / edges |
|---|---:|---:|---:|---:|---:|---:|
| caddy | 95 | 19 / 1339 | 76 | 90 | 0 | 5 / 0 |
| prometheus | 2852 | 32 / 50 | 2820 | 2497 | 0 | 355 / 43 |
| etcd | 4172 | 89 / 113 | 4083 | 3722 | 0 | 450 / 0 |
| hugo | 1655 | 3 / 14 | 1652 | 1422 | 0 | 233 / 12 |

Every R3 record present on main is byte-identical in its legacy projection. New records arise because wave 2 newly recovers function-literal and exact qualified receiver inputs; they do not reflect a change to the R3 branch or its carried-identity input.

## 7. Invariants, residual work, and verdict

- Resolver and manifest still consume one shared `go_concrete_receiver_route`; no second proof was added.
- Qualified lookup uses exact effective import path first; unresolved or multiply mapped exact paths fail closed, and legacy basename lookup remains only as fallback when exact evidence is absent.
- R3 consumer inputs and the bare ladder are unchanged. Generic receivers still record 0/0/0 fallback telemetry when `iface_key` rejects them.
- Cache pins are CPG 49 and sidecar 18; the four-path cold/no-cache/exact-CPG/sidecar parity test remains green.
- New modules and dedicated wave-2 tests are under 600 lines (`go_concrete_receiver.rs` 500, `go_selector_supply.rs` 233, fix2 tests 166 and 113).
- Not done by instruction: gopls oracle. The controller should rerun it, review the corrected 14-site Etcd expectation, push the commits, and publish the PR wave.

**SURVIVED (self-pass, not independent):** the two oracle WRONG paths are closed, existing matched R3 output is unchanged, the final suite is 3,338/0/1, Tier-A is 104/104, and all requested corpus/manifest evidence is pinned above.
