# #16 Task 2 RED and Task 3 switch evidence

**Captured:** 2026-08-31T01:43:23Z  
**Lane:** `/Users/wesleyjinks/code/slicing-16-post-provenance` · `a-go-interface-identity-post-provenance`  
**Exact base:** `b7a5cf934a44060de98588837b3c8c75ddffdc37`  
**RED checkpoint:** `c017395860964df02577fd021f8f18c84aade99b`

## Verdict

The owner-authorized v16 replacement floor is satisfied. Public source fixtures reach both production outcomes admitted at the surviving owner-bearing R3 seam. Exact base enters the legacy bare-name arm and mints `decoy.Wrong.Run`; the switched candidate never re-enters that table and instead resolves the registered app callback set or drops terminally. Resolver, manifest, and navigation sidecar agree, including a two-callback same-file edge case.

This evidence authorizes Task 4 verification. It does not replace the mandatory five-corpus conservation/parity rerun, Tier-A, full suite, or implementation review round 2.

## Source validity and oracle disposition

Both fixture packages compile under the Go toolchain. `/private/tmp/p16-source-fixture-v1` proves the one-callback shape; `/private/tmp/p16-source-fixture-multi-v1` proves the two-callback shape. The latter completed `go test ./...` for `example.com/p16fixture/app` and `example.com/p16fixture/decoy` with exit `0`.

Go type identity makes `c := New()` an `app.S` value even though `invoke` contains a later caller-local `type S struct{}` declaration. `c.Run()` is therefore a function-field call on `app.S`. `decoy.Wrong.Run` cannot be its target. Registered `app.worker` or `app.workerA`/`app.workerB` are admissible callback targets; with no registration, the correct modeled result is terminal `ExternalReceiver`.

## Exact-base RED controls

The control worktree is `/private/tmp/slicing-p16-base-b7a5cf93`, detached at exact base with only test overlays and the expected sidecar-pin assertion. It remains intentionally dirty for replay.

| Gate | Selected | Exact-base result | Intended discriminator |
|---|---:|---|---|
| Go public fixture module | 5 | 1 pass, 4 fail | Base resolves `decoy.Wrong.Run` for registered, unregistered, manifest, and two-target cases; the ownerless negative remains green. |
| Registered resolver RED | 1 | fail | Legacy telemetry `sites/hits/edges = 1/1/1`; target is `decoy.Wrong.Run`, not `app.worker`. |
| Unregistered resolver RED | 1 | fail | Base mints `decoy.Wrong.Run` instead of terminal `ExternalReceiver`. |
| Manifest parity RED | 1 | fail | Base reports the unrelated interface route/target rather than the func-field target. |
| Two-target manifest RED | 1 | fail | Base resolves one decoy edge rather than both app callbacks. |
| Navigation sidecar RED | 1 | fail | No-cache/cold base evidence contains `decoy.Run` and lacks `workerA`/`workerB`. |
| Sidecar version pin | 1 | fail | Exact base is version `22`; the watched assertion requires `23`. |

An initial command used `--exact` with an unqualified Rust test name and selected zero tests. It is inadmissible and excluded. Enumeration then found the fully qualified selector and every behavioral probe used a nonzero selected count.

## Candidate GREEN checkpoint

| Gate | Selected | Candidate result |
|---|---:|---|
| Go public fixture module | 5 | 5 passed, 0 failed |
| Navigation sidecar no-cache/cold/hit parity | 1 | 1 passed, 0 failed |
| Direct route/retirement tests | 5 | 5 passed, 0 failed |
| Added promoted/profile/live direct partitions | 2 | 2 passed, 0 failed |
| Sidecar version pin | 1 | 1 passed, 0 failed |

The two-target fixture first exposed a candidate manifest defect: resolver output and exact identities both contained `workerA` and `workerB`, but `fanout` was `1` because the manifest counted one deduplicated owner label. That watched selector failed `0/1`, then passed `1/1` after consult-terminal rows were changed to count exact target identities. Existing interface-route owner-cardinality semantics were left unchanged.

## Production switch

- `resolve_call_site_full` replaces only the surviving owner-bearing R3 `iface_key → interface_impls` branch with `go_proven_interface_outcome(site, owner)`.
- `go_proven_interface_outcome` uses the carried `GoOwnerIdentity`, caller-scoped declaration/signature/profile filtering, live selection, and shared arity; an empty selection continues once through the existing owner-qualified func-field helper.
- `interface_dispatch_manifest` consumes the same outcome and serializes its final target identities. It never re-enters the bare table.
- The sidecar remains resolver-derived. CPG stays `54`; the navigation call-edge sidecar advances `22 → 23`.

## Implementation review round 1

Review cap: `2` rounds.

- `WRONG`: zero.
- `SMELL`: direct coverage did not yet pin guarded promoted supply, signature rejection, per-owner profile uncertainty, or live/profile ordering. Fixed with two direct tests; `2/2` pass.
- `SMELL`: the two-target fixture did not yet cross the navigation sidecar. Fixed by upgrading the sidecar fixture; candidate `1/1` passes and exact base `0/1` fails for the intended decoy edge.
- Collapsed concern: consult partition evidence is not separately folded into resolver call-stats at this seam. Non-default interface evidence cannot reach the production seam because declaration-proven interfaces exit earlier; the source-reachable concrete-owner route is `invalid_drop` with default consult evidence, and the existing func-field helper retains its own partition telemetry. This mechanism cannot produce an incorrect call-stats value in the admitted operating path.

Round 1 therefore ends with zero open `WRONG` and zero open in-scope `SMELL`. Round 2 remains gated on the full verification and current-corpus delta evidence.

## Custody and exclusions

The exact-base worktree is deliberately dirty with copied tests; it is evidence, not a publication worktree. The historical `/Users/wesleyjinks/code/slicing-16c1-sol` clone remains untouched. No full-suite, Clippy, Tier-A, current five-corpus conservation, Go oracle delta, CI, push, or merge claim is made here.
