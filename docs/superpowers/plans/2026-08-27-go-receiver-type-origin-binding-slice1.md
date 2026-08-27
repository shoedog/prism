# Go receiver type-origin binding — Slice 1 owner carrying

**Base:** `65697b0e50f2b4617e3a1d57562098d82175c01f` (`origin/main`, after Slice 0 and its custody closeout)
**Design authority:** `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md`, Slice 1 and §§5–6
**Predecessor handoff:** `docs/superpowers/handoffs/2026-08-25-go-receiver-type-origin-binding-slice0-handoff.md`

## 1. Outcome and boundary

Make S3 package-variable receiver provenance expressible without widening any other receiver lane:

1. Replace the naked `unique_visible_type` result with one selection carrying both raw type text and the `GoOwnerIdentity` resolved in each fact's own `defining_file` namespace.
2. Select only facts visible under the caller's exact package clause and build profile. Missing, empty, or unparsed profiles are uncertain and fail closed.
3. Treat different raw texts or different resolved owners for one fresh S3 classification as a conflict. In particular, textually identical `ext.I` facts that resolve to different owners must not collapse into a false unique value.
4. Populate S3 recovery with `owner_identity: Some(owner)` and feed its real `GoPartitionEvidence` into the existing telemetry path.
5. Preserve rematerialization replacement semantics: an incremental defining-file import change A→B replaces A with B; persisted A is not competing evidence.
6. Keep `CallSite::cmp_key` and equality unchanged. The occurrence key remains caller/callee/span/qualifier/type; ambiguity is decided before projection into `CallSite`.

This slice does not implement Slice 2 eager caller-local owner population, Slice 3 blanket absent-provenance drops, interface implementation population, or any legacy owner-resolution widening.

## 2. Constructible wrong result and RED

The committed ignored sentinel declares `var V ext.I` in `app/a.go`, where `ext` names `outside.example/api`, then calls `V.M()` in `app/b.go`, where `ext` names the in-repo decoy package. On base, S3 retains raw `ext.I` with no owner and the resolver mints exact `decoy/types.go:M`.

Measured pre-change RED:

```text
cargo test --test lang_go receiver_origin_prereq_slice1_cross_file_package_var_alias_sentinel -- --ignored --nocapture
1 selected; FAILED
CallSite receiver_type Some("ext.I"), receiver_owner_identity None
ResolutionOutcome exact decoy/types.go:M
```

The test must be unignored and pass because no owner or target can be proven from the defining file. A zero-selected run is inadmissible.

## 3. File boundary

Expected production files:

- `src/go_receiver_index_visibility.rs` — replace `unique_visible_type` with the owner-returning S3 consult and real evidence.
- `src/go_receiver_index.rs` — consume the selection, carry the owner, make failed/ambiguous S3 consults materialized no-recovery, and classify admitted S3 as `CarriedOwner`.
- `src/cpg_cache.rs` — topology fence `51 -> 52` with pin/history update.
- `src/navigation/call_edge_cache.rs` — paired topology fence `19 -> 20` with pin/history update.

Expected test files:

- `tests/lang/go/receiver_origin_prereq_test.rs` — unignore the existing cross-file alias sentinel.
- `tests/lang/go/receiver_owner_carrying_test.rs` — positive owner/target, same-text competing owners, exact `p`/`p_test` package clauses, profile-less drop, and incremental A→B full-build parity.
- `tests/lang/go/main.rs` — register the Slice 1 test module.
- `tests/navigation/go_concrete_cache_test.rs` — assert the S3 owner and outputs match no-cache, cold-create, exact-CPG-hit, and sidecar-hit paths.

No generated Tier-A report or baseline is committed.

## 4. Selection contract

The owner-returning consult has this semantic result:

```rust
struct GoResolvedPackageVarType {
    owner: GoOwnerIdentity,
    raw_type: String,
}

fn unique_visible_package_var_type(...) -> GoPartitionSelection<GoResolvedPackageVarType>
```

For every fact:

1. Require caller and defining profiles.
2. Apply same-package visibility without rewriting the defining package clause. This is a bare package-variable reference: `p` sees `p`, and `p_test` sees `p_test`.
3. Require exact caller and defining profiles; uncertain visibility returns no value with `evidence.uncertain = true`.
4. Resolve `fact.ty` using receiver-strict identity in `fact.defining_file`.
5. Count visible/filtered declarations and distinct visible raw/owner values.
6. Return one value only when both the raw-type set and owner set are singleton. More than one of either is `evidence.conflict = true` and no value.

An S3 key with facts but no selected value produces a materialized no-recovery classification so stale cached owners and legacy caller-namespace rebinding cannot survive. A selected value uses `ReceiverRecovery::VarDecl`, carries `Some(owner)`, returns the consult evidence, and enters the already-landed `CarriedOwner` prerequisite screen.

## 5. RED/GREEN order

1. Unignore the existing sentinel and add the Slice 1 matrix before production edits.
2. Run the exact Slice 1 selector; require at least one constructible failure on base and no harness errors.
3. Implement only the S3 consult and consumer.
4. Run focused language tests, owner-partition controls, and incremental parity.
5. Bump both caches and extend the four-path cache parity battery.
6. Run full verification and the Accuracy Harness required by the repository steering.

## 6. Verification gates

Focused:

```bash
cargo test --test lang_go receiver_origin_prereq_slice1 -- --nocapture
cargo test --test lang_go receiver_owner_carrying -- --nocapture
cargo test --test lang_go owner_partition -- --nocapture
cargo test --test navigation concrete_receiver_outputs_match_no_cache_cold_create_exact_cpg_and_sidecar_hits -- --nocapture
```

Repository:

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
git diff --check
```

Use `--allow-stale-sut` only after the immediate release rebuild in this worktree. Any base-identical exclusion needs a same-environment base control; do not rebaseline.

## 7. Acceptance

- The old exact decoy edge is absent across resolver, manifest, and navigation.
- A valid cross-file package variable stores the exact defining-file owner and resolves only its real target.
- Same-text/different-owner facts fail closed with owner-partition drop telemetry.
- `p` and `p_test` callers select only their exact package-clause facts.
- Missing/unparsed profile evidence fails closed.
- Incremental A→B owner replacement matches a fresh full build with no duplicate callsite.
- The serialized CPG and sidecar retain the owner and output parity.
- Full suite totals, Tier-A results, review verdicts, and exclusions are recorded in the living handoff.
