# Handoff — P17 narrow concrete-receiver routing, fix wave 1

**Written:** 2026-08-23T17:50:09Z · **By:** Codex `/root`
**Workspace:** `/Users/wesleyjinks/code/slicing-p17-narrow` · branch `p17-narrow-concrete-receiver`
**Measured implementation HEAD before this handoff:** `a07ab99` · base `514cfe3`
**Cache schema:** CPG `47`, navigation sidecar `16`

## 0. Gating facts

- The controller amended the owner decision after wave 1: existing `apply_go_embedding_promotion` edges are preserved; only misses from that lane remain deferred. The authoritative branch design is now v8 in `docs/superpowers/specs/2026-08-22-p17-narrow-concrete-receiver-direct-design.md` (`5839378`).
- No subagents were used. No push was performed. The controller owns the push and the gopls oracle.
- The implementation and all requested implementer-owned gates are complete. The only unrun gate is gopls, explicitly reserved to the controller.
- Temporary read-only evidence remains under `/private/tmp`: base worktree `p17-base-514cfe3`, wave-1 worktree `p17-wave1-5bb05e0`, and site-keyed manifest captures. They were retained for controller review.

## 1. Fix-wave commits

| Commit | Item | Result |
|---|---|---|
| `5839378` | `p17-fix1: amend concrete receiver design to v8` | Records the existing-promotion preservation rule, depth-aware supplier choice, and fix-wave poles. |
| `fd1ec86` | `p17-fix1: preserve existing concrete promotions` | Consults the existing owner lookup before deferred drop; adds `concrete_promoted`, telemetry, and depth-aware concrete/interface supplier routing. |
| `4d72a51` | `p17-fix1: preserve unproven receiver behavior` | Restores carried-identity-only R3 manifest input and counts a bare fallback attempt only after `iface_key` succeeds. |
| `431f71d` | `p17-fix1: resolve named func value fields` | Resolves named/aliased function field types through the declaration graph before P5. |
| `86cff77` | `p17-fix1: classify pointer interface aliases` | Routes `type P = *I` as interface; unresolved pointees fail closed. |
| `86ee7a3` | `p17-fix1: fail closed on local type shadows` | Records lexically visible function-local type shadowing on `CallSite`; the shared route returns R3. |
| `0674bf3` | `p17-fix1: compare cold cache creation output` | Compares cold-create manifest/call-stats wires as the fourth cache path. |
| `a07ab99` | `p17-fix1: reject pointer interface selector supply` | Full-suite repair: invalid embedded `*I` cannot become S4 supply, while `type P = *I` receiver routing and pointer-to-concrete promotion remain intact. |

## 2. Behavioral evidence

### Named focused tests

- `cargo test --test lang_go concrete_receiver_ --no-fail-fast`: **43 passed, 0 failed, 0 ignored** after the final pointer-embed pole was added. The earlier pre-pole run was 42/0/0.
- `go_embedded_concrete_method_keeps_existing_promotion`, `go_embedded_transitive_concrete_method_keeps_existing_promotion`, `go_embedded_pointer_receiver_addressable_keeps_existing_promotion`, `go_embedded_method_labeled_on_receiver_var_path`, `go_embedded_interface_field_not_promoted`: **5/0/0**.
- `call_stats_reports_existing_concrete_promotion_and_ambiguity`: **1/0/0**.
- `concrete_receiver_outputs_match_no_cache_cold_create_exact_cpg_and_sidecar_hits`: **1/0/0**.
- `local_interface_shadows_package_concrete_alias_at_call`, `local_concrete_shadows_package_interface_at_call`, `local_type_in_different_function_does_not_shadow_package_alias`: all passed inside the 7/0/0 `local_` filter.
- `pointer_embedded_interface_never_supplies_an_s4_selector`: red before `a07ab99` with false Exact `InterfaceDispatch` to `Wrong.M`, then green.
- Existing `s4_pointer_embedded_interface_never_routes_or_satisfies`: candidate red before `a07ab99`; same-environment base `514cfe3` control green 1/0/0; final candidate green 1/0/0.
- `pointer_to_interface_alias_keeps_interface_dispatch` and `go_embedded_pointer_receiver_addressable_keeps_existing_promotion`: both green after the bounded pointer-embed repair.

### Full gates

| Gate | Result |
|---|---|
| `cargo fmt` and `cargo fmt -- --check` | passed |
| `git diff --check` | passed |
| `cargo test --quiet` | **3,334 passed, 0 failed, 1 ignored** |
| `cargo build --release` | passed |
| `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut` after the release build | **104 passed, 0 failed**; `go/embedded_method` passed |
| gopls oracle | not run; controller-owned by explicit instruction |

The first full-suite attempt was not retained as green: it stopped at the library target with 713 passed, 1 failed, 0 ignored on the invalid embedded `*I` regression. Base passed the same test in the same environment. Commit `a07ab99` fixed that bounded failure; the final full run above is the acceptance result.

## 3. Same-base corpus evidence

All candidate results use the final release implementation with `prism nav --no-cache call-stats`; controls are controller-provided `ctrl514-*.txt` from main `514cfe3`. Values are `base -> candidate (delta)`. Missing direct-kind entries are zero.

| Corpus | interface Exact | constructor Exact | typed-param Exact | var Exact | assertion Exact | return Exact | field-typed Exact | embedded-promotion Exact | P5 NameOnly | NLCF | multi-target sites |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| caddy | 1766 -> 1776 (+10) | 241 -> 258 (+17) | 107 -> 1972 (+1865) | 0 -> 0 | 0 -> 0 | 233 -> 233 | 15 -> 15 | 84 -> 87 (+3) | 0 -> 0 | 3 -> 3 | 21 -> 21 |
| prometheus | 2461 -> 2294 (-167) | 978 -> 1441 (+463) | 770 -> 1349 (+579) | 0 -> 0 | 0 -> 0 | 2135 -> 2135 | 125 -> 125 | 58 -> 58 | 1 -> 1 | 143 -> 143 | 3855 -> 3813 (-42) |
| etcd | 2002 -> 1924 (-78) | 109 -> 116 (+7) | 230 -> 489 (+259) | 0 -> 0 | 0 -> 0 | 1064 -> 1064 | 38 -> 38 | 42 -> 54 (+12) | 0 -> 0 | 578 -> 578 | 515 -> 417 (-98) |
| hugo | 625 -> 654 (+29) | 193 -> 201 (+8) | 440 -> 592 (+152) | 0 -> 0 | 0 -> 0 | 2927 -> 2927 | 92 -> 92 | 46 -> 52 (+6) | 0 -> 0 | 330 -> 330 | 2568 -> 2565 (-3) |

New telemetry and wave-1 route deltas:

| Corpus | direct | existing promoted | deferred promoted, wave 1 -> current | no-selector, wave 1 -> current | R3 sites / hits / edges |
|---|---:|---:|---:|---:|---:|
| caddy | 2470 | 7 | 7 -> 0 | 431 -> 431 | 1978 / 19 / 1339 |
| prometheus | 4876 | 19 | 19 -> 0 | 91 -> 91 | 4295 / 7 / 7 |
| etcd | 1703 | 12 | 12 -> 0 | 80 -> 89 | 6618 / 164 / 196 |
| hugo | 3811 | 11 | 11 -> 0 | 684 -> 688 | 3641 / 6 / 7 |

The promotion-only expectation “interface dispatch unchanged from wave 1” holds for Caddy and Prometheus. Etcd is +20 and Hugo +1 versus wave 1 because of the separately required OX-S2 depth-aware rule, not fallback leakage:

- Etcd: five `concrete_no_selector_drop -> embedded_interface_dispatch` sites, four visible package-scoped implementers each, add 20 Exact edges; fifteen zero-fanout embedded routes become no-selector; seven deferred sites become `concrete_promoted`.
- Hugo: one `concrete_no_selector_drop -> embedded_interface_dispatch` site adds one Exact edge; six zero-fanout embedded routes become no-selector; three deferred sites become `concrete_promoted`.
- No concrete route has interface fanout.

Ripgrep is byte-identical: **3,019 base bytes == 3,019 candidate bytes**.

## 4. R3 manifest parity

The normal manifest intentionally serializes both successful R3 bare hits and proven R2 hits as `interface_dispatch`. For an admissible R3-only control, a temporary environment-gated audit bit was derived from the single shared `go_concrete_receiver_route` verdict, then removed before the final build. Base and candidate records were matched by `(file,start_byte,end_byte,method)` and projected to legacy fields by removing candidate-only route/audit diagnostics.

| Corpus | proof-R3 manifest sites | R3 hit sites / edges | R3 drops | missing in base | changed legacy records |
|---|---:|---:|---:|---:|---:|
| caddy | 90 | 19 / 1339 | 71 | 0 | 0 |
| prometheus | 2497 | 7 / 7 | 2490 | 0 | 0 |
| etcd | 3892 | 164 / 196 | 3728 | 0 | 0 |
| hugo | 1490 | 6 / 7 | 1484 | 0 | 0 |

This covers the OX-S1 branch: current R3 manifest output is byte-equivalent to main for every legacy record, including successful bare-ladder hits.

## 5. Invariants and residual work

- Resolver and manifest consume the one shared `go_concrete_receiver_route`; no inline second proof exists.
- Existing promotion remains the #95/#174/#176 owner-index lane. `concrete_promoted` labels it; `concrete_promoted_deferred_drop` applies only after that lane misses.
- Depth chooses the shallowest selector supplier; equal-depth mixed suppliers terminally drop as `ConcreteReceiverNoSelector`.
- Pointer-to-interface aliases are R2 receiver types, but pointer-to-interface embedded fields are not valid S4 supply.
- R3 uses carried identity only and otherwise preserves the bare ladder byte-for-byte; generic receivers do not increment fallback telemetry when `iface_key` rejects them.
- The declaration graph remains P10-identity/profile keyed, serialized, import-environment aware, transitive, and fail-closed.
- New modules and changed dedicated test files remain under 600 lines (`go_concrete_receiver.rs` 500, `go_selector_supply.rs` 233, `go_func_type.rs` 124; largest changed dedicated test file 440).
- Controller next actions: run the reserved gopls oracle, review this handoff commit, and push. Do not rerun or reinterpret the v7 terminal-promotion behavior; v8 supersedes it.

## 6. Final verdict

**SURVIVED (self-pass, not independent):** proven concrete-recovered receivers do not reach the bare-name interface-dispatch fallback. Evidence: red-first route poles, exact target-file assertions, final 3,334/0/1 Rust suite, 104/104 Tier-A including `go/embedded_method`, four-path cache parity, five-corpus route/telemetry comparison, R3 same-base manifest parity, and non-Go byte identity.
