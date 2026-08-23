# Handoff — P17 narrow concrete-receiver routing, fix wave 3

**Written:** 2026-08-23T19:44:10Z · **By:** Codex `/root`
**Workspace:** `/Users/wesleyjinks/code/slicing-p17-narrow` · branch `p17-narrow-concrete-receiver`
**Measured implementation HEAD before this handoff:** `f2a6ccb` · base `514cfe3`
**Cache schema:** CPG `47`, navigation sidecar `16`

## 0. Gating facts

- The authoritative design is v8. Fix wave 3 updated §5 to state that this PR makes one schema transition, main's 46/15 to 47/16; 48/17 remains reserved for #14 slice 4.
- No subagents were used. No push was performed. The controller owns the push and gopls oracle.
- The ordinary receiver classifier and all non-direct same-scope-reuse sites retain the exact f16663e projection. The exception is post-merge-only and may survive only when the shared `go_concrete_receiver_route` verdict is `ConcreteDirect`.
- Final evidence is retained under `/private/tmp`: the f16663e controls, committed candidate call-stat captures, and the final full-test log.

## 1. Fix-wave-3 commits

| Commit | Item | Result |
|---|---|---|
| `1c3c013` | `p17-fix3: preserve same-scope short declaration reuse` | Adds same-block multi-name `:=` reuse/type-identity proof and direct/changed-type/nested-shadow/nested-reuse poles. |
| `6118217` | `p17-fix3: collapse cache schema to one PR transition` | Pins CPG 47 and sidecar 16 and corrects design §5. |
| `866c347` | `p17-fix3: enforce Go package-key lifecycle` | Enforces the `go_package_basenames` clear/rebuild invariant and adds the edit/reapply regression. |
| `dcb1f32` | `p17-fix3: narrow short declaration reuse to direct routes` | Uses the one shared route verdict so the reuse exception cannot broaden R2, promotion, P5, or no-selector output. |
| `f2a6ccb` | `p17-fix3: preserve non-direct reuse projection` | Isolates the special AST scan to post-merge direct proof and skips non-direct updates byte-for-byte. |

The pushed wave-2 head was `f16663e`; its four fix-wave-2 commits remain immediately below these commits.

## 2. Root-cause and discriminator log

### Item 1 — same-scope `:=` reuse

- Initial hypothesis: `walk_receiver_bindings` counted every later LHS occurrence as a new Go binding, so `x, err := reset(x)` set `go_lexical_rebinding` even though `x` is reused in the same block. Alternative: the wrong edge came from a changed return type rather than binding identity. Discriminator: a same-owner `Reset(*q.C) (*q.C, error)` pole lost direct proof while `Different() (*q.D, error)` also failed closed. The same-owner pole established the binding bug; resolving every reuse RHS through the return-type index distinguishes and rejects the changed-type pole.
- First corpus follow-up hypothesis: remaining Hugo deltas were real same-block reuses of interface/external receivers; alternative: the AST scope predicate still admitted sibling declarations. Live source showed `var contentrc hugio.ReadSeekCloser; contentrc, err := ...` and `reflect.Value` reuses in the same block, proving the first hypothesis. Those were non-R1(a) routes.
- Final fix: the ordinary classifier remains f16663e-identical. A post-merge special scan ignores a qualifying reuse only after its return owner matches the original owner. The resolver then consumes the existing shared route verdict; only `ConcreteDirect` is materialized. Every non-direct verdict skips the update entirely, preserving receiver fields, drop buckets, and R3 output.
- A nested constructor pole exposed a second mechanism: `x := &q.C{}` was counted but not constructor-recovered, and the existing S1 retry on `q.Reset` supplied the type. The special evidence mode now feeds that same retry; no new recovery ladder was added.

### Item 2 — cache versions

The 48/17 values treated review waves as independent schema releases. This PR has one externally visible schema transition, so `CACHE_VERSION` is 47 and `NAV_CALL_EDGE_CACHE_VERSION` is 16. Pin tests and design §5 agree; 48/17 is reserved for the later #14 slice-4 PR.

### Item 3 — package-key lifecycle

`clear_go_func_value_fields` intentionally preserves `go_package_basenames` for exact cached-CPG parity, but a direct func-value reapply after a file edit could otherwise retain stale `@go-import:` keys. The enforced invariant is: exact import-path keys originate only in the full scope-input dispatch build; clearing dispatch clears the entire map; P5 may preserve only a snapshot whose package directories still match current files. A mismatch clears the snapshot and rebuilds the fail-closed basename-only projection. Debug assertions pin both halves.

## 3. Red/green and full gates

### Named focused tests

- `concrete_receiver_fix3_test::{same_scope_multi_name_short_declarations_reuse_the_receiver_binding,changed_static_type_and_nested_shadow_still_fail_closed_to_r3,unrelated_sibling_scope_does_not_enable_a_new_receiver_proof,same_scope_reuse_only_revives_a_proven_concrete_direct_route}`: **4/0/0**. Before item 1, the same-owner pole reached R3; before `dcb1f32`, the interface reuse was not shadowed.
- `concrete_receiver_fix2_test::*`: **3/0/0**, retaining type-switch/inner/range value-rebinding behavior.
- `func_value_reapply_after_edit_discards_stale_import_path_keys`: **1/0/0**. Before `866c347`, the old exact import key survived reapply.
- `cache_versions_are_pinned_for_go_loader_hygiene` and `sidecar_version_is_pinned_for_go_loader_hygiene`: green at **47/16**.
- `concrete_receiver_outputs_match_no_cache_cold_create_exact_cpg_and_sidecar_hits`: green; it compares no-cache, cold-create, exact CPG hit, and sidecar hit wire outputs.

### Full gates

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` and `git diff --check` | passed |
| `cargo test -- --format terse` | **3,343 passed, 0 failed, 1 ignored** across 28 result groups; retained log `/private/tmp/p17-fix3-final-cargo-test.log` |
| `cargo build --release` | passed after the final committed code |
| `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut` immediately after that release build | **104 passed, 0 failed**; `go/embedded_method` passed |
| gopls oracle | not run; controller-owned by explicit instruction |

## 4. Required manifest checks and Etcd oracle classification

- Hugo `tpl/tplimpl/templates.go:205/:212`: `unproven_drop`, fanout 0, no outer-interface implementers.
- Etcd `revision_test.go:114/:126` and `v3_failover_test.go:93`: `concrete_direct`, fanout 0.

The live Etcd fixture and oracle artifact classify the 11 cache sites below as own methods of `*cache.Cache`; the three Cluster sites are the wave-2 qualified-owner fix. No #16/R1(c) site remains among these 14.

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

## 5. Corpus evidence

All values use the final committed release binary and `prism nav --no-cache call-stats`.

### Delta from fix-wave-2 head `f16663e`

| Corpus | Result versus f16663e |
|---|---|
| ripgrep | byte-identical |
| caddy | byte-identical |
| prometheus | `go_concrete_receiver_direct +4`; Exact/kind `return_typed +4`; `dropped_go_receiver.none -4`; `dropped_multi_owner -4`; every interface, promotion, no-selector, and R3 telemetry leaf unchanged |
| etcd | byte-identical |
| hugo | byte-identical |

Thus item 1 is the only numeric mover, and it only restores four direct edges at same-scope-reuse sites.

### Same-base main control (`514cfe3`) to final

Values are `base -> final (delta)`; missing kinds are zero. NLCF is `interface_overapprox.NonLocalConstructionFallback`.

| Corpus | interface Exact | constructor Exact | typed-param Exact | return Exact | field Exact | embedded Exact | P5 NameOnly | NLCF | multi-target |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| caddy | 1766 -> 1777 (+11) | 241 -> 258 (+17) | 107 -> 1978 (+1871) | 233 -> 233 | 15 -> 16 (+1) | 84 -> 87 (+3) | 0 -> 0 | 3 -> 3 | 21 -> 21 |
| prometheus | 2461 -> 2752 (+291) | 978 -> 1472 (+494) | 770 -> 1522 (+752) | 2135 -> 2148 (+13) | 125 -> 182 (+57) | 58 -> 58 | 1 -> 1 | 143 -> 143 | 3855 -> 3904 (+49) |
| etcd | 2002 -> 2528 (+526) | 109 -> 118 (+9) | 230 -> 750 (+520) | 1064 -> 1593 (+529) | 38 -> 103 (+65) | 42 -> 54 (+12) | 0 -> 0 | 578 -> 578 | 515 -> 609 (+94) |
| hugo | 625 -> 756 (+131) | 193 -> 205 (+12) | 440 -> 1144 (+704) | 2927 -> 2952 (+25) | 92 -> 117 (+25) | 46 -> 52 (+6) | 0 -> 0 | 330 -> 330 | 2568 -> 2583 (+15) |

New telemetry keys are absent/zero on main; final counts are:

| Corpus | direct | existing promoted | deferred promoted | no-selector drop | R3 sites / hits / edges |
|---|---:|---:|---:|---:|---:|
| caddy | 2477 | 7 | 0 | 433 | 2466 / 19 / 1339 |
| prometheus | 5106 | 19 | 0 | 97 | 5471 / 32 / 50 |
| etcd | 2561 | 12 | 0 | 77 | 7152 / 89 / 113 |
| hugo | 4405 | 11 | 0 | 695 | 4232 / 3 / 14 |

Ripgrep remains byte-identical to main: **3,019 base bytes == 3,019 final bytes**.

## 6. R3 parity retained from wave 2

The wave-2 temporary route audit matched candidate/base records by `(file,start_byte,end_byte,method)` and removed candidate-only diagnostics before comparing legacy projections.

| Corpus | final proof-R3 sites | hits / edges | matched base records | changed matched legacy records |
|---|---:|---:|---:|---:|
| caddy | 95 | 19 / 1339 | 90 | 0 |
| prometheus | 2852 | 32 / 50 | 2497 | 0 |
| etcd | 4172 | 89 / 113 | 3722 | 0 |
| hugo | 1655 | 3 / 14 | 1422 | 0 |

Wave 3 leaves those R3 telemetry leaves unchanged versus f16663e and restores the exact f16663e CallSite/drop projection for all non-direct reuses.

## 7. Per-file size limit and residual work

Every file newly added by this PR is below the 600-line per-file limit:

| File | Lines |
|---|---:|
| `docs/superpowers/handoffs/2026-08-23-p17-narrow-handoff.md` | 162 |
| `docs/superpowers/specs/2026-08-22-p17-narrow-concrete-receiver-direct-design.md` | 181 |
| `src/go_concrete_receiver.rs` | 500 |
| `src/go_func_type.rs` | 124 |
| `src/go_selector_supply.rs` | 233 |
| `tests/ast/go_package_basename_lifecycle_test.rs` | 67 |
| `tests/lang/go/concrete_receiver_alias_test.rs` | 440 |
| `tests/lang/go/concrete_receiver_fix2_test.rs` | 166 |
| `tests/lang/go/concrete_receiver_fix3_test.rs` | 192 |
| `tests/lang/go/concrete_receiver_manifest_test.rs` | 367 |
| `tests/lang/go/concrete_receiver_qualified_fix2_test.rs` | 113 |
| `tests/lang/go/concrete_receiver_route_test.rs` | 427 |
| `tests/lang/go/concrete_receiver_unproven_test.rs` | 246 |
| `tests/navigation/go_concrete_cache_test.rs` | 91 |

Not done by instruction: gopls oracle and push. The controller should rerun the oracle, push the wave-3 commits, and publish the PR wave.

**SURVIVED (self-pass, not independent):** all enumerated wave-3 findings are closed; only four intended Prometheus direct edges differ from f16663e; the final suite is 3,343/0/1; Tier-A is 104/104; cache pins are 47/16; and the requested corpus evidence is pinned above.
