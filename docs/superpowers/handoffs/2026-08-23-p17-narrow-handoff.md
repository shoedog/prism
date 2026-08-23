# Handoff — P17 narrow concrete-receiver routing, fix wave 4

**Written:** 2026-08-23T20:14:47Z · **By:** Codex `/root`
**Workspace:** `/Users/wesleyjinks/code/slicing-p17-narrow` · branch `p17-narrow-concrete-receiver`
**Measured implementation HEAD before this handoff:** `405e220` · base `514cfe3`
**Cache schema:** CPG `47`, navigation sidecar `16`

## 0. Status and scope boundary

- Implemented the requested package-name collision guard for on-demand R2: when the CallSite carries no owner identity and more than one package declares the proven interface bare name, the shared route returns a terminal `R2OnDemandNameCollisionBail` verdict.
- Resolver output is `ExternalReceiver` with zero targets and telemetry `go_r2_on_demand_name_collision_bail=1`. Manifest output is `dispatch_route=unproven_drop`, fanout 0. Carried-identity R2 and unique-name on-demand R2 are unchanged.
- A corpus probe found that 16 of the controller's 20 Prometheus records were already marked `receiver_local_type_shadowed`; they therefore reached the R3 bare ladder before the first implementation's collision check. The terminal collision proof now runs after owner/declaration/profile validation but before the shadow-to-R3 return. All R1/R2 edge-minting remains behind the shadow bail, and non-collision R3 output is unchanged.
- **Unresolved controller/spec discrepancy:** four of the requested 20 Prometheus records are parameters of external `io.Closer`. The corpus contains zero in-repo `type Closer interface` declarations, so those sites have neither an on-demand R2 proof nor a >1-package interface-declaration collision. They remain R3 legacy `interface_dispatch` records. Suppressing them would require a new external/R3 rule, contrary to “R3 unchanged” and “no #16 scope creep.” Exact records are pinned in §4.
- No subagents were used. No push or gopls oracle was run.

## 1. Fix-wave-4 commits

| Commit | Result |
|---|---|
| `d7e9ae0` — `p17-fix4: add on-demand R2 collision poles` | Red resolver/manifest poles for duplicate interface names, unique interface names, and carried identity. The duplicate pole initially minted two false Exact edges. |
| `9883ec0` — `p17-fix4: bail on-demand R2 name collisions` | Adds the shared terminal verdict, package-identity census, resolver telemetry, call-stats key, and manifest label. |
| `405e220` — `p17-fix4: terminally bail shadowed interface collisions` | Moves only the terminal collision check ahead of the existing shadow-to-R3 return and adds the corpus-mirroring same-type rebinding pole. |

Wave-3 commits `1c3c013`, `6118217`, `866c347`, `dcb1f32`, and `f2a6ccb` remain immediately below these commits. They retain the 47/16 cache transition and direct-only same-scope-reuse behavior.

## 2. Hypothesis / probe / result log

### Ordinary on-demand R2 collision

- Hypothesis: the shared consult receives the original carried CallSite owner separately from its internal on-demand lookup. Alternative: a caller passes an on-demand owner as though it were carried. Discriminator: resolver and manifest call sites both pass `site.receiver_owner_identity.as_ref()` directly; the synthetic `var it p.Iterator` pole has `None`, while the `p.NewIterator()` pole has `Some(p.Iterator)`.
- Hypothesis: distinct interface packages can be counted without the lossy bare dispatch table. Alternative: only the bare `interface_impls` projection survives. Discriminator: serialized `go_interface_declarations` is keyed by `(package_dir, package_clause, name)` and contains both synthetic fixture owners.
- Red result: `var it p.Iterator; it.Next()` with `p.Iterator` and `q.Iterator` minted `p.PImpl.Next` and `q.QImpl.Next`, both Exact `InterfaceDispatch`.
- Green result: the site has zero targets, `unproven_drop`, and collision telemetry 1; unique `UniqueIterator` and carried `p.Iterator` stay S4.

### Prometheus shadowed collision

- Initial H1: the named corpus sites unexpectedly carry owner identity, bypassing an on-demand-only guard. Alternative H2: they are on-demand but the declaration census contains only one `Iterator` package. A temporary, uncommitted trace falsified both: `proven=None`, resolved owner `tsdb/chunkenc.Iterator`, and packages `{tsdb/chunkenc, tsdb/chunks}`.
- H3: `receiver_local_type_shadowed` causes the early `Unproven` verdict. Alternative H4: owner/declaration/profile validation fails first. The second temporary trace showed `shadow=true` and immediate shadow exit at the named sites.
- The same-type rebinding pole reproduced the corpus: before `405e220`, it emitted two false Exact edges plus legacy fallback telemetry sites/hits/edges `1/1/2`. After the correction it terminally bails with all legacy fallback counters zero.
- The temporary trace was removed before the final build; `rg 'P17FIX4(DBG|START|EXIT)' src` is empty.

### Hugo `AddIdentity`

- `resources/resource_transformers/tocss/scss/tocss.go:122` has receiver field `ResourceTransformationCtx.DependencyManager identity.Manager`. The corpus has exactly one `Manager` interface declaration, at `identity/identity.go:279`, and `AddIdentity` is declared on it at line 281. The collision guard therefore correctly does not bail.
- The file has `//go:build extended`, while the controller oracle environment reports empty tags. This explains the unresolved gopls definition without making the Prism edge type-incorrect. Final manifest remains `interface_dispatch`, fanout 1, implementer `nopManager`.

## 3. Tests and build gates

### Named poles

- `on_demand_r2_name_collision_bails_with_zero_fanout_and_telemetry`
- `shadowed_on_demand_interface_collision_still_takes_the_terminal_bail`
- `unique_on_demand_interface_name_keeps_s4_dispatch`
- `carried_interface_identity_ignores_the_on_demand_collision_guard`

Focused final results: fix-wave-4 poles **4 passed / 0 failed / 0 ignored**; complete Go binary **189/0/0**.

| Gate | Final result |
|---|---|
| `cargo fmt -- --check` and `git diff --check` | passed |
| `cargo test` | **3,347 passed, 0 failed, 1 ignored** |
| `cargo build --release` | passed after removing temporary diagnostics |
| `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut` immediately after release build | **104 passed, 0 failed**; `go/embedded_method` passed |
| gopls oracle | not run; controller-owned by instruction |

## 4. Manifest verification

Final Prometheus manifest: `/private/tmp/p17-fix4-final-prometheus-manifest.json`.

### Closed Iterator collision records: 16/16

| Records | Count | Final result |
|---|---:|---|
| `storage/fanout_test.go:97/98/124/125/202/203/229/230` | 8 | `unproven_drop`, fanout 0 |
| `rules/manager_test.go:620/621` | 2 | `unproven_drop`, fanout 0 |
| `tsdb/compact_test.go:1127` (two `Next` sites), `:1130`, `:1150` | 4 | `unproven_drop`, fanout 0 |
| `tsdb/head_read.go:751/754` | 2 | `unproven_drop`, fanout 0 |

`FakeChunkSeriesIterator` is absent from the final manifest.

### Four records outside the authorized collision rule

| File:line | Receiver | Final route/fanout | Why not changed |
|---|---|---|---|
| `tsdb/head_append_v2_test.go:1508` | `cl io.Closer` | `interface_dispatch` / 1 (`HTTPResourceClient`) | external interface; no in-repo `Closer` declaration or R2 proof |
| `tsdb/head_append_v2_test.go:1517` | `cl io.Closer` | `interface_dispatch` / 1 (`HTTPResourceClient`) | same |
| `tsdb/head_test.go:4176` | `cl io.Closer` | `interface_dispatch` / 1 (`HTTPResourceClient`) | same |
| `tsdb/head_test.go:4185` | `cl io.Closer` | `interface_dispatch` / 1 (`HTTPResourceClient`) | same |

These four require controller direction: either remove them from the wave-4 collision acceptance population or authorize a separately specified R3/external fail-closed rule.

## 5. Five-corpus call-stats

All candidate values use the final release binary with `prism nav --no-cache call-stats`. Controls are controller-generated `ctrl514-*.txt` from main `514cfe3`. NLCF is `interface_overapprox.NonLocalConstructionFallback`.

### Delta from wave-3 committed output

Every direct Exact kind, P5 NameOnly, NLCF, promotion telemetry, and concrete no-selector telemetry is unchanged.

| Corpus | interface Exact | multi-target sites | legacy R3 sites/hits/edges | collision bails | external drops |
|---|---:|---:|---:|---:|---:|
| ripgrep | 0 -> 0 | 0 -> 0 | 0/0/0 -> 0/0/0 | 0 | 615 -> 615 |
| caddy | 1777 -> 1757 (-20) | 21 -> 21 | 2466/19/1339 unchanged | 20 | 2643 -> 2663 (+20) |
| prometheus | 2752 -> 2310 (-442 edges) | 3904 -> 3794 (-110) | 5471/32/50 -> 5453/16/34 | 188 | 6688 -> 6830 (+142) |
| etcd | 2528 -> 2426 (-102 edges) | 609 -> 606 (-3) | 7152/89/113 unchanged | 166 | 8787 -> 8886 (+99) |
| hugo | 756 -> 667 (-89 edges) | 2583 -> 2582 (-1) | 4232/3/14 unchanged | 199 | 4969 -> 5056 (+87) |

Telemetry is site-counted; `kind_exact.interface_dispatch` is edge-counted, so fanout makes edge reductions larger than bail-site counts.

### Same-base main control to final

Values are `base -> final`.

| Corpus | interface | constructor | typed-param | return | field | embedded | P5 | NLCF | multi-target |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ripgrep | 0 -> 0 | 314 -> 314 | 315 -> 315 | 1269 -> 1269 | 273 -> 273 | 0 -> 0 | 0 -> 0 | 0 -> 0 | 0 -> 0 |
| caddy | 1766 -> 1757 | 241 -> 258 | 107 -> 1978 | 233 -> 233 | 15 -> 16 | 84 -> 87 | 0 -> 0 | 3 -> 3 | 21 -> 21 |
| prometheus | 2461 -> 2310 | 978 -> 1472 | 770 -> 1522 | 2135 -> 2148 | 125 -> 182 | 58 -> 58 | 1 -> 1 | 143 -> 143 | 3855 -> 3794 |
| etcd | 2002 -> 2426 | 109 -> 118 | 230 -> 750 | 1064 -> 1593 | 38 -> 103 | 42 -> 54 | 0 -> 0 | 578 -> 578 | 515 -> 606 |
| hugo | 625 -> 667 | 193 -> 205 | 440 -> 1144 | 2927 -> 2952 | 92 -> 117 | 46 -> 52 | 0 -> 0 | 330 -> 330 | 2568 -> 2582 |

| Corpus | direct | existing promoted | deferred promoted | no-selector | R3 sites/hits/edges | collision bails |
|---|---:|---:|---:|---:|---:|---:|
| caddy | 2477 | 7 | 0 | 433 | 2466/19/1339 | 20 |
| prometheus | 5106 | 19 | 0 | 97 | 5453/16/34 | 188 |
| etcd | 2561 | 12 | 0 | 77 | 7152/89/113 | 166 |
| hugo | 4405 | 11 | 0 | 695 | 4232/3/14 | 199 |

Ripgrep is byte-identical to main: **3,019 bytes == 3,019 bytes** (`cmp` exit 0).

## 6. Per-file limit and retained artifacts

Every file newly added by this PR remains below the 600-line per-file limit. Final measured counts include:

| File | Lines |
|---|---:|
| `src/go_concrete_receiver.rs` | 532 |
| `tests/lang/go/concrete_receiver_fix4_test.rs` | 200 |
| `tests/lang/go/concrete_receiver_fix3_test.rs` | 192 |
| `tests/lang/go/concrete_receiver_fix2_test.rs` | 166 |
| `tests/lang/go/concrete_receiver_alias_test.rs` | 440 |
| `tests/lang/go/concrete_receiver_manifest_test.rs` | 367 |
| `tests/lang/go/concrete_receiver_route_test.rs` | 427 |
| `tests/lang/go/concrete_receiver_unproven_test.rs` | 246 |
| `tests/navigation/go_concrete_cache_test.rs` | 91 |

Final evidence:

- `/private/tmp/p17-fix4-final-callstats-{ripgrep,caddy,prometheus,etcd,hugo}.txt`
- `/private/tmp/p17-fix4-final-prometheus-manifest.json`
- `/private/tmp/p17-fix4-final-hugo-manifest.json`

Not done: push and gopls oracle, by instruction. The four external-`io.Closer` records above remain blocked on a scope/design decision.

**PARTIAL SURVIVAL:** the authorized on-demand R2 name-collision guard, telemetry, tests, 104/104 Tier-A, five-corpus evidence, and all 16 Iterator blocker records are complete. The controller's 20/20 manifest acceptance cannot be claimed without changing R3/external behavior beyond the authorized rule.
