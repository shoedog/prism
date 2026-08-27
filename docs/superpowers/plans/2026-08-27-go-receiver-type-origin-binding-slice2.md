# Go receiver type-origin binding — Slice 2 eager local owners

**Base:** `1a82bb0de43e2c1bac1eb8717a4166099c4e0c20` (`origin/main`, after Slice 1 implementation PR #207 and custody PR #208)
**Design authority:** `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md`, Slice 2 and §§5–6
**Predecessor handoff:** `docs/superpowers/handoffs/2026-08-27-go-receiver-type-origin-binding-slice1-handoff.md`

## 1. Outcome and boundary

Populate exact Go owner identity for receiver facts whose defining namespace is already proven to be the caller file:

1. Reuse the existing caller-file prerequisite screen's strict owner resolution and declaration-admissibility proof.
2. After an admissible proof, attach that exact owner to typed parameters, constructor locals, local `var` declarations, and type assertions.
3. Never attach the first binding's owner when `proof_shadowed` is true. The legacy shadow/collision bail remains authoritative.
4. Preserve every poison and uncertainty boundary already enforced by Slice 0: type parameters, local type declarations, predeclared names, dot-import uncertainty, unresolved strict imports, and inadmissible declarations still drop.
5. Leave carried cross-file owners from Slice 1 and uncarried cross-file failures unchanged.
6. Keep `CallSite::cmp_key`, equality, classification origins, and owner resolver signatures unchanged.

This slice does not implement Slice 3's blanket absent-provenance terminal predicate, change interface implementation population, widen the legacy resolver, or infer owners from persisted call sites.

## 2. Constructible wrong result and RED

The existing collision fixture defines both `p.Iterator` and `q.Iterator`, then calls `it.Next()` from `func onDemand() { var it p.Iterator; ... }`. On the merged base, the caller-file screen proves `p.Iterator` is admissible but discards the resolved owner. The stored site therefore has `receiver_owner_identity == None`, takes the on-demand bare-name collision guard, and incorrectly returns zero targets instead of the exact `p.PImpl.Next` target.

The paired `rebound` fixture is the negative discriminator: a later `it, err := reset(it)` makes `proof_shadowed == true`. It must retain `owner_identity == None`, zero targets, and the collision-bail count. If Slice 2 attaches the first binding's owner there, dispatch bypasses the shadow guard and becomes unsound.

RED is established before production edits by changing the unshadowed fixture to require exact `p.Iterator` identity/dispatch while leaving the shadowed assertions unchanged. The four-form concrete fixture additionally requires exact `q` owners for typed parameter, constructor local, local `var`, and type assertion recoveries. The cache fixture requires the same typed-parameter owner after no-cache, cold-create, exact-CPG-hit, and sidecar-hit paths.

## 3. File boundary

Expected production files:

- `src/go_receiver_index.rs` — retain the strict owner resolved by `screen_go_receiver_prerequisites` for admissible, unshadowed `CallerFile` classifications.
- `src/cpg_cache.rs` — topology fence `52 -> 53` with pin/history update.
- `src/navigation/call_edge_cache.rs` — paired topology fence `20 -> 21` with pin/history update.

Expected test files:

- `tests/lang/go/concrete_receiver_route_test.rs` — assert exact owners for all four caller-local recovery forms already present in the fixture.
- `tests/lang/go/concrete_receiver_fix4_test.rs` — make the unshadowed collision case an exact-dispatch positive, preserve the shadowed collision-bail negative, and require exact owner on the unique-interface control.
- `tests/navigation/go_concrete_cache_test.rs` — assert the caller-local typed-parameter owner across all four cache paths.

No new test module, `CallSite` field, generated Tier-A report, or baseline is committed.

## 4. Mutation contract

`screen_go_receiver_prerequisites` remains the single membrane:

1. `CrossFileUncarried` returns unchanged.
2. `CarriedOwner` continues to require and validate the already-carried owner.
3. `CallerFile` continues to reject type-parameter bindings and local type declarations before owner resolution.
4. It resolves the static type strictly in `caller_file` and requires an exact, visible declaration.
5. If admissible and `proof_shadowed == false`, it writes that owner into the recovered receiver and returns the existing evidence.
6. If admissible and `proof_shadowed == true`, it leaves the owner absent and returns the classification unchanged.
7. Every failed proof retains its existing terminal materialized drop.

The mutation happens before rematerialization writes either the `calls` or `callers` view, so both stored views receive the same screened classification. No consumer sees the newly attached owner before the prerequisite membrane.

## 5. RED/GREEN order

1. Update only the three existing test fixtures and cache-version pins.
2. Run exact selectors and require constructible failures with nonzero selected tests on the unchanged merged base.
3. Change only the caller-file admissible branch in `screen_go_receiver_prerequisites`.
4. Run the exact RED selectors and owner-prerequisite controls GREEN.
5. Bump both cache topology versions and run the four-path cache battery.
6. Run the full repository and Accuracy Harness gates, followed by a two-round capped self-review.

## 6. Verification gates

Focused:

```bash
cargo test --test lang_go concrete_receiver_route -- --nocapture
cargo test --test lang_go concrete_receiver_fix4 -- --nocapture
cargo test --test lang_go receiver_origin_prereq -- --nocapture
cargo test --test lang_go receiver_owner_carrying -- --nocapture
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

Use `--allow-stale-sut` only after the immediate release rebuild in this worktree. Any failing required gate needs an exact-base same-environment control before attribution. Do not rebaseline. Re-run the established five-corpus public-output control at this exact base and explain every receiver-provenance delta and total-site count.

## 7. Acceptance

- Each admissible unshadowed caller-local recovery stores the exact declaration-backed owner.
- The colliding unshadowed `p.Iterator` site resolves only `p/types.go` with exact interface dispatch and no on-demand collision bail.
- The shadowed `Iterator` site retains no owner, zero fanout, and the collision bail.
- Type parameters, local type shadows, predeclared receivers, unresolved imports, cross-file conflicts, and profileless facts retain their existing drops.
- No duplicate call site appears and total occurrence counts remain unchanged.
- CPG and sidecar versions are paired at `53` and `21`; all four cache paths retain owner and output parity.
- Full-suite totals, Tier-A results, corpus deltas, review verdicts, and exclusions are recorded in the living handoff.
