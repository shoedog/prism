# #16 Go interface identity — post-provenance implementation plan

**Design authority:** `docs/superpowers/specs/2026-08-23-go-package-qualified-interface-identity-design.md` v14
**Exact base:** `b7a5cf934a44060de98588837b3c8c75ddffdc37`
**Lane:** `/Users/wesleyjinks/code/slicing-16-post-provenance` · `a-go-interface-identity-post-provenance`
**Owner authority:** successor continuation, push, and merge are authorized after gates; relevant non-coverage CI is mandatory and Coverage is not a wait condition.
**Review caps:** design `2` rounds (complete, plus one disclosed scoped confirmation and one bounded preflight artifact correction); implementation `2` rounds.

## Goal and boundary

Replace only the surviving owner-bearing Go R3 `iface_key(recv_ty) → interface_impls` consult with one caller-scoped consult keyed by the already-proven `receiver_owner_identity`. Resolver and interface manifest must share the final owner/walk/live/arity decision; the navigation sidecar continues to reuse resolver output.

Do not change receiver recovery, owner population, `CallSite`, `cmp_key`, global owner resolution, declaration-kind routing, embedded-interface routing, func-value-field semantics, static-table construction, or CPG serialization. Preserve the old dirty `/Users/wesleyjinks/code/slicing-16c1-sol` clone without writes.

Permitted production files are initially bounded to:

- `src/resolution.rs`
- `src/navigation/queries.rs`
- `src/go_promoted_snapshot.rs`
- `src/navigation/call_edge_cache.rs`

Tests and this lane's docs may be added or updated. Any other production file requires a written plan amendment before editing.

## Task 0 — Durable design and lane custody

1. Preserve the replayed v12, v13, and PARK commits as separate history.
2. Commit v14, this plan, and the live handoff before census code.
3. Record the old clone's branch, base, dirty files, diff size, and untracked-spec digest; do not clean, stash, rebase, or cherry-pick its WIP wholesale.
4. Confirm current worktree/branch/base and a clean tree before Task 1.

## Task 1 — Shared candidate consult and removable census harness

1. Add the compiled v14 consult types and signature without switching production routing:

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

2. Extract the current caller-scoped satisfier walk from `go_visible_s4_implementers`. Preserve its per-owner conflict/uncertainty behavior exactly for existing callers.
3. Add a promoted-snapshot consult that returns both the selected snapshot entry and its unique canonical method declaration. Enforce profile, variant, field-shadow, join, generic, signature-presence, and signature-match guards.
4. Enable promoted supply only for `go_proven_interface_consult`; existing S4 callers remain direct-only.
5. Put the temporary census behind `PRISM_P16_CENSUS`. Emit an exhaustive pre-terminal prerequisite ledger (`ownerless_terminal` | `owner_bearing`) and a separate candidate ledger only for owner-bearing sites that actually reach the legacy bare-table arm. Invoke the consult only for candidate-ledger rows. Normal manifest output must remain byte-stable when the variable is absent.
6. Unit-test route partition, target identity encoding, and the ownerless no-invocation invariant. Compile before running any corpus probe.
7. Build release immediately before the five pinned corpus runs. Preserve keyed current/candidate rows, terminal outcomes, complete target-set diffs, binary SHA, corpus SHAs, commands, exits, and artifact hashes outside the worktree.
8. Oracle-join every changed target set and disposition every historical sentinel named by v14. Stop before Task 2 on incomplete coverage, a new over-approximation, an unexplained recall loss, an ownerless consult invocation, or a delta outside the complete census.

The census harness is a durable checkpoint, then removed or reduced to tests before production publication. Its code is not production authorization.

## Task 2 — Compile and prove the public RED matrix

Add public-behavior tests before switching either consumer. Enumerate registered selectors with `cargo test -- --list`; a zero-selected command is inadmissible. Required behaviors include these exact selector stems:

- `go_proven_interface_owner_beats_bare_collision`
- `go_proven_interface_origin_namespace_decoy_stays_absent`
- `go_proven_interface_recovered_positive_matrix`
- `go_proven_interface_promoted_and_direct_satisfiers_survive`
- `go_proven_interface_promoted_signature_mismatch_drops_only_promoted_owner`
- `go_proven_interface_per_owner_conflict_preserves_exact_satisfier`
- `go_proven_interface_live_profile_order`
- `go_proven_interface_arity_empty_uses_func_value_field_without_bare_retry`
- `interface_manifest_go_proven_interface_parity`
- `navigation_sidecar_go_proven_interface_parity`
- `go_proven_interface_terminal_negative_matrix`

Add the sidecar `22 → 23` pin RED and retain the CPG `54` assertion. Capture each watched failure's selected count, assertion, exit, and exact-base result. A compile failure caused by an undeclared signature is fixed before behavioral RED evidence is accepted.

## Task 3 — Production consumer switch

1. Route the resolver's surviving owner-bearing R3 interface subconsult through `go_proven_interface_consult`.
2. On `live_hit`/`fallback_hit`, mint only the consult's arity-filtered Exact targets.
3. On empty/invalid/walk/arity outcomes, continue once to `func_value_field_or_external_drop`; never retry `interface_impls`.
4. Route the manifest's matching R3 arm through the same consult and derive `dispatch_route`, fanout, and implementer identities from its result.
5. Keep `go_receiver_owner_is_terminally_unproven` before both consumers. Sidecar parity remains resolver-derived; do not add a third predicate or consult.
6. Advance only `NAV_CALL_EDGE_CACHE_VERSION` and its pin to `23`; keep CPG at `54`.
7. Remove the temporary census surface from normal output. Retain only production telemetry explicitly admitted by v14 and tests needed to prevent route drift.
8. Run the exact RED selectors to GREEN and record totals. Any expectation change requires a two-sided exact-base control showing the old assertion is obsolete.

## Task 4 — Full verification and current-base evidence

Run in this order, preserving exits and complete logs:

1. focused resolver/manifest/sidecar/cache selectors;
2. `cargo fmt --check`;
3. `cargo check`;
4. `cargo clippy --all-targets --all-features`;
5. `cargo clippy --all-targets --all-features -- -D warnings` plus same-environment exact-base control if nonzero;
6. `cargo test --no-fail-fast`, with all summaries aggregated;
7. `cargo build --release` and SHA-256 the resulting binary;
8. `cd eval && uv run tier-a --matrix-only --allow-stale-sut` immediately after that build;
9. `cd eval && uv run tier-a --quick --allow-stale-sut`, reading the report and running exact-base control for any invalid/failing result;
10. five-corpus no-cache call-stats and manifests at the pinned SHAs;
11. four Go oracle deltas, the #17b population audit, and source/gopls adjudication for every changed sound site;
12. four-path sidecar cache parity and explicit deserialized-live-set assertion.

Do not rebaseline generated Tier-A or oracle artifacts. Move generated outputs out of the worktree and verify the final status is clean except deliberate source/docs changes.

## Task 5 — Bounded review, publication, and closeout

1. Review round 1 tags every item `WRONG` or `SMELL`; a `WRONG` names the constructible input/state and incorrect result. Fix closed enumerable findings in place.
2. Review round 2 re-audits both consumers, await-free cache behavior, all changed corpus rows, and the shared-consult blast radius. At cap, classify before acting; an open-class proxy-for-provenance recurrence parks the lane.
3. Require zero `WRONG` and zero in-scope `SMELL`, then refresh the handoff with exact evidence and final HEAD.
4. Push, open the PR with the census and all exclusions, require every relevant non-coverage CI check green, and merge under standing authority without waiting for Coverage.
5. Reconcile merge state in a docs-only closeout if the merged handoff still asserts publication is pending.

## Stop conditions

Stop and report before expanding scope if any step requires receiver-text inference, a global identity fallback, owner mutation, `CallSite`/`cmp_key` changes, static-table retirement, a third consumer, CPG movement, a new serialized field, an unenumerated corpus transition, or a review finding that makes the proxy-for-provenance class open again.
