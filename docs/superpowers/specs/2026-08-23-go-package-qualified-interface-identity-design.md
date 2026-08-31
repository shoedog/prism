# #16 — Go package-qualified interface identity (design **v15 — post-provenance owner consult; fixture-authorized continuation**)

**Status:** **ACTIVE at Task 2 under the owner-authorized fixture substitute.** The preserved v12→v13→PARK chain was replayed and v14 resumed after receiver-provenance Slices 0→3. The corrected Task 1 census then selected zero candidate sites across all five pinned corpora. On 2026-08-31 the owner explicitly authorized v15 to retain that zero-population result as mandatory evidence while substituting the source-fixture and exact-base RED floor in §4 for production authorization. Evidence: `docs/superpowers/handoffs/2026-08-31-go-interface-identity-task1-census-evidence.md`. **Base:** `b7a5cf934a44060de98588837b3c8c75ddffdc37` (PR #214 closeout; CPG 54, nav sidecar 22). **Roadmap:** row 16. **Review cap:** 2 rounds.

**Citation policy (the failure mode this rewrite fixes):** cite **symbols**, not line numbers. Line numbers below are advisory as of this writing and MUST be re-verified before use.

## 1. Problem and now-satisfied prerequisite

The surviving Go R3 interface consult (the `iface_key(recv_ty)` arm in `resolve_call_site_full`, mirrored by the `Unproven` arm of `interface_dispatch_manifest`) still reads `CallGraph::interface_impls`, a table keyed by bare interface name. Same-named interfaces in different packages collapse together, so an owner-bearing site can still mint implementers of an unrelated package's interface.

v13 correctly parked because receiver text was not provenance: package-variable text could be recovered in one file and reinterpreted in another file's namespace. That prerequisite is now closed. `CallSite::receiver_owner_identity` is declaration-backed and origin-correct for the admitted producers, and `go_receiver_owner_is_terminally_unproven` drops every recovered Go site lacking that owner before the resolver or manifest reaches this consult. The old identity-establishment and global-index fallback problem is therefore removed from #16's input domain rather than approximated inside #16.

The current route is still structurally reachable: an owner-bearing site classified `GoConcreteReceiverRoute::Unproven` can miss own-method, embedded-interface, and visible-interface fast paths, then reach the bare `interface_impls` lookup. v15 replaces only that surviving table consult.

## 2. Rule — one owner-proven consult

Both production consumers call one shared consult with the exact carried owner:

```rust
fn go_proven_interface_consult<'a>(
    &'a self,
    owner: &GoOwnerIdentity,
    method_name: &str,
    caller_file: &str,
    arg_count: Option<usize>,
    arg_spread: bool,
) -> GoProvenInterfaceConsult<'a>;
```

The consult executes these steps in order:

1. **Require the carried owner.** Callers may invoke the consult only after the shared terminal predicate. There is no `recv_ty` parameter, no caller-namespace re-resolution, no `iface_key`, and no global name index. An ownerless call is a caller-contract defect and must remain a terminal drop, never a fallback.
2. **Select the exact interface declaration.** Use the carried `GoOwnerIdentity` with `go_owner_reference_mode` and `select_interface_signatures_with_mode`. Missing, non-dispatchable, profile-uncertain, or conflicting declarations fail closed for this interface subconsult. Validation is load-bearing here because it selects the declaration whose signatures govern the walk; raw universe membership is not sufficient.
3. **Walk caller-scoped satisfiers.** Extract the existing `go_visible_s4_implementers` walk without changing its current per-owner conflict/uncertainty semantics. Direct method sets come from `go_method_declarations`. Promoted method sets come from the R1(b) selector snapshot under exactly its existing guards: `ProfileUnique`, one declaration, exact variant coherence, no field shadow, unique `FunctionId` join, non-generic method, present canonical signature, and a signature match to the interface requirement. A rejected promoted owner contributes nothing; it does not suppress an independently proven direct owner.
4. **Apply live selection.** Intersect satisfier admission keys with `go_interface_live_types`. A non-empty intersection wins; an empty intersection falls back to the complete exact satisfier set, preserving the existing receiver-kind-aware-empty behavior.
5. **Apply arity.** Use the shared `arity_admits`/`arity_filter` rule inside the consult so resolver and manifest receive the same final target set.
6. **Route the result.** A non-empty selection mints Exact `InterfaceDispatch`. Resolver-only empty or arity-empty results continue to `func_value_field_or_external_drop`; the interface manifest records the interface-subconsult disposition and zero interface fanout. No empty result re-enters `interface_impls`.

`GoProvenInterfaceConsult` carries the final selection, route (`live_hit`, `fallback_hit`, `invalid_drop`, `walk_drop`, or `arity_drop`), all/live/final cardinalities, the partition evidence, and promoted-supply rejection counts. Those fields are the shared source for resolver telemetry, manifest diagnostics, and the census.

## 3. What does not change

- The static `compute_interface_dispatch` table remains built and serialized for existing statistics and the other explicitly proven routes. This slice replaces only the owner-bearing R3 bare-name consult.
- `go_receiver_owner_is_terminally_unproven`, owner production, `CallSite`, `CallSite::cmp_key`, declaration-kind routing, embedded-interface routing, func-value-field routing, and the global owner resolver do not change.
- Existing S4 consumers retain their direct-only behavior. The extracted walk must byte-preserve their outputs; promoted supply is enabled only for the new #16 consult.
- The navigation sidecar continues to derive edges through the resolver. The manifest is the second and only other production consumer.
- CPG stays at version `54`; the nav sidecar advances `22 → 23` because resolved edge outcomes change.

## 4. Census gate before production routing

The first implementation phase is a removable census harness, not the route switch. It emits two keyed ledgers. The **prerequisite ledger** enumerates every recovered Go site in the interface-manifest denominator before the shared terminal predicate and partitions it `ownerless_terminal` | `owner_bearing`; an ownerless row must never invoke the consult. The **candidate ledger** includes only owner-bearing sites whose current control flow survives concrete, embedded-interface, visible-interface, and other terminal screens and actually reaches the legacy `iface_key → interface_impls` arm. For every candidate-ledger row on the five pinned corpora, compare the existing table result with `go_proven_interface_consult` using identical site keys and final arity filtering.

- Each ledger is exhaustive within its stated population. The prerequisite ledger has `ownerless_terminal` | `owner_bearing`. The candidate-ledger interface-subconsult route has `live_hit` | `fallback_hit` | `invalid_drop` | `walk_drop` | `arity_drop`. Keyed counts must reconcile: candidate keys are a subset of owner-bearing prerequisite keys, and every ownerless actual-consult row falsifies the prerequisite and stops this lane.
- Each row carries a separate terminal outcome because `arity_drop`/empty interface results can still mint through `func_value_field_or_external_drop` in the resolver.
- Compare target identities per implementer and per site. A strict subset is a finding even if an aggregate oracle still labels the site `sound`.
- Rebind, do not inherit, the old floors and kill list. Caddy `CaddyModule`/`CertMagicStorage`, the etcd `AuthBackend` population, the eight historical over-approx sites, caddy `metrics.go:56`, and the six historical qualified additions are named sentinels, but their current counts and dispositions must be remeasured on base `b7a5cf93`.
- Every added, removed, or changed target set receives an oracle disposition. Any newly Exact over-approximation, target mismatch, unresolved oracle row, incomplete coverage, or unowned recall loss stops production work.
- The harness must execute the same consult as production or be byte-compared against it on the complete attempt set. A self-failing probe is inadmissible. A zero-selection natural-corpus probe cannot prove behavior, but under the explicit 2026-08-31 owner amendment it is admissible as population evidence only when the following replacement floor is met.
- The replacement floor uses public source fixtures, not post-build graph or resolver-state mutation. On exact base, at least one fixture must produce `go_unproven_receiver_bare_fallback_sites > 0`, mint an incorrect same-bare-package target set, and fail the package-qualified expectation for that intended reason. The candidate must make that same selector green without weakening its target assertion.
- Public selectors must exercise every production consult route: `live_hit`, `fallback_hit`, `invalid_drop`, `walk_drop`, and `arity_drop`. They must cover a positive direct satisfier, guarded promoted supply, a same-bare collision, and the existing func-value-field continuation for empty interface results. Each changed path has a negative or edge case.
- Resolver, interface manifest, and sidecar must agree on every fixture's final target identities and terminal result. Every target-set delta receives a source or gopls oracle disposition. Fixture build constraints or profile witnesses must be source-valid for the profile they claim; an invalid fixture is inadmissible.
- The five pinned corpora remain mandatory controls: their zero candidate population, pinned identities, total call-site conservation, default-output parity, and static-table telemetry stability must be rechecked after the production switch. They are not represented as positive production coverage.
- Preserve the old dirty census clone `/Users/wesleyjinks/code/slicing-16c1-sol` as historical evidence only. Do not build from or overwrite its stale base.

## 5. Red-first fixture matrix

- **Owner identity beats bare collision:** proven `p.I` plus unrelated `q.I` with the same method name mints only `p.I` satisfiers across resolver, manifest, and sidecar; the current table path is the RED.
- **Origin namespace survives cross-file transport:** the v13 `ext.I` decoy counterexample retains the true owner and never mints the caller-file decoy.
- **Recovered positives:** package-variable, typed-parameter, constructor-local, field, and return receivers with proven interface owners retain exact targets; the etcd constructor-return shape is pinned explicitly.
- **Terminal negatives:** predeclared, type-parameter, local-shadow, dot-import, external-qualified, missing-profile, and active-shadow cases remain ownerless and absent from resolver/manifest/sidecar. #16 never attempts to recover them.
- **Promoted supply:** a live promoted `T.Base.M` plus direct `U.M` both mint; profile-conflicting, variant-conflicting, invariant-failing, generic, missing-signature, and signature-mismatched promoted owners contribute nothing while independent exact owners survive.
- **Per-owner direct uncertainty:** an uncertain/conflicting concrete owner is excluded without suppressing an independent exact satisfier; if no exact satisfier survives, the site drops with the corresponding evidence.
- **Live/profile order:** profile filtering precedes live selection, and an empty live intersection falls back only to the exact caller-visible satisfier set.
- **Arity and terminal routing:** arity survivors mint; arity-empty and walk-empty interface results take the existing func-value-field path before terminal drop. No bare-table retry occurs.
- **Cache parity:** cold/hit/refresh/no-cache paths agree, the deserialized live set is non-empty in a Go positive, and sidecar pin `23` is asserted while CPG remains `54`.

Every behavior added or changed needs a test that fails on exact base, plus a negative or edge case for its new path. Exact selector names and pre-change failures are recorded before production edits.

## 6. Acceptance gates

1. v15 review converges within the declared cap of 2 rounds. `WRONG` findings precede `SMELL`; any open-class recurrence parks the lane.
2. The complete five-corpus census and the owner-authorized source-fixture replacement floor in §4 both pass and are attached to the implementation PR.
3. The compiled RED matrix in §5 fails for the intended behavioral reasons on exact base and turns green without weakening expectations.
4. Focused resolver/manifest/sidecar/cache tests pass with exact selected totals.
5. `cargo fmt --check`, `cargo check`, ordinary all-target/all-feature Clippy, and exact `-D warnings` run. Any candidate failure requires an exact-base control in the same environment.
6. `cargo test --no-fail-fast` completes with captured exit status and aggregate totals.
7. Immediately rebuild release, then run Tier-A `--matrix-only --allow-stale-sut` and `--quick --allow-stale-sut`. A nonzero quick exit is read and controlled on exact base; generated eval artifacts are not committed.
8. Five-corpus call-stats/manifests use the actual implementation base and pinned corpus SHAs. Total call-site counts are conserved, ripgrep is byte-identical, static-table telemetry is byte-stable, and every route/target delta matches the census.
9. Four Go oracle deltas have full coverage and zero newly Exact over-approximation, timeout, unresolved, or target mismatch. Re-run the #17b population and gopls/source oracle for every changed sound site.
10. Cache versions are CPG `54`, sidecar `23`; no other serialized schema changes.
11. Round-2 code review reports zero `WRONG` and zero in-scope `SMELL` before publication. Relevant non-coverage CI checks must pass; Coverage is not a wait condition by owner direction.

## 7. Delivery and stop conditions

Work proceeds on `/Users/wesleyjinks/code/slicing-16-post-provenance`, branch `a-go-interface-identity-post-provenance`, from exact base `b7a5cf93`. Preserve the replayed v12/v13/PARK commits; the amendment, census harness, REDs, production switch, verification, and review each receive durable checkpoints.

Stop if the consult needs receiver-text inference, a global-index fallback, owner population, changes to `CallSite` identity, a third independent consumer, static-table retirement, CPG version movement, or a corpus delta outside the enumerated census. Stop and park if review again exposes an open proxy-for-provenance class; A's merged owner is the only identity authority in this design.

## 8. History

v1–v3 (static-table shapes) parked at cap for an open profile-witness class. C1 chosen by the owner 2026-08-23; v4–v8 folded successive review rounds and **v8 merged as the then design-of-record (#195)**. Its re-census passed the amended floor on all four corpora but surfaced a stdlib-receiver false edge; v9/v10 tried to make the identity-invalid fallback safe and were **parked as open-class** (#201). v11 removed the fallback but retained contradictory gates and stale citations and was **parked as an artifact defect** (#202). v12/v13 removed the fallback and demanded positive text evidence, but the cross-file alias counterexample proved text still was not provenance; `ea74558f` parked the artifact and required A first. Receiver-provenance Slices 0→3 plus prerequisite PR #212 then made `receiver_owner_identity` declaration-backed and terminalized every absent owner; PR #213 completed A. v14 resumed the preserved artifact by deleting the refuted identity-establishment layer and consuming only that proven owner. Its corrected Task 1 census found no natural-corpus candidates; the owner's 2026-08-31 v15 amendment admits a bounded source-fixture replacement floor while retaining that zero population as mandatory evidence.
