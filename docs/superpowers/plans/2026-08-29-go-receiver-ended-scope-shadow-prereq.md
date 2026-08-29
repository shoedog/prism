# Go receiver ended-scope shadow prerequisite

## Objective

Repair the Slice 3 prerequisite falsified by pinned Prometheus `prompb/io/prometheus/client/decoder.go:302`: a supported typed-parameter receiver must not lose its declaration-backed owner because an earlier same-name Go declaration belongs to a lexical scope that does not contain the call.

This is a prerequisite to the Slice 3 terminal-owner predicate. It does not broaden the parked recovery surface for ordinary `=` assignments, non-direct same-scope `:=` reuse, or aliases initialized from fields.

## Exact base and lane

- Base: `7fc719ae21ba130c554c318c3f8306093a804c92` (Slice 2 closeout merge).
- Branch: `a-receiver-provenance-scope-aware-shadow-prereq`.
- Worktree: `/Users/wesleyjinks/code/slicing-a-receiver-provenance-scope-prereq`.
- Review cap: two rounds.

## RED contract

Extend `tests/lang/go/receiver_owner_carrying_test.rs` with a declaration-backed local interface and calls after:

1. an ended nested-block `x := ...` shadow;
2. an ended `if x := ...` initializer shadow;
3. an ended nested-block `var x ...` shadow.

For each post-scope call, require the original typed parameter's exact `GoOwnerIdentity`, `receiver_local_type_shadowed == false`, the expected interface edge, and manifest presence. Exact base must compile and fail these assertions.

Add the negative control in the same fixture: when the call remains inside the nested block, the real inner declaration must still set `receiver_local_type_shadowed`, retain no owner, and avoid an exact-owner proof.

## Production change

Expected production file: `src/ast.rs` only.

Split two currently conflated controls in `walk_receiver_bindings`:

- Go declaration visibility: always count a `short_var_declaration` or `var_spec` only when its enclosing Go binding scope contains the call.
- Same-scope short-declaration reuse: preserve the existing opt-in proof path and its current direct-route boundary; scope filtering must not silently enable it.

Thread both booleans through recursive calls. Do not change assignment handling, return-type proof, owner resolution, call-graph routing, the Slice 3 predicate, `CallSite::cmp_key`, cache keys, or non-Go behavior.

## Verification

1. Prove compiled RED on exact base with the new selectors and retain the failure output.
2. Implement the bounded change; run the new tests plus existing `concrete_receiver_fix3_test` scope/reuse controls.
3. Run `cargo fmt --check`, `cargo test --no-fail-fast`, `cargo clippy --all-targets --all-features`, and `cargo build --release`; compare any Clippy failure to the exact base in the same environment.
4. Because `src/ast.rs` touches CPG construction, run an immediate release rebuild followed by:
   - `cd eval && uv run tier-a --matrix-only --allow-stale-sut`
   - `cd eval && uv run tier-a --quick --allow-stale-sut`
5. Re-run the pinned Prometheus manifest/oracle comparison. Line 302 must retain its sound edge; the explicitly parked ownerless rows must remain unchanged.
6. Run the declared two-round `WRONG`-before-`SMELL` review. Stop on any valid recovered Go positive still made ownerless by this bounded mechanism.

## Publication and successor

After clean verification and review, push/open/merge this prerequisite under the owner's standing authorization. Then rebase the preserved Slice 3 branch onto the merged prerequisite, rerun Slice 3's full verification and capped review, and only then publish Slice 3.

## STOP conditions

- The fix requires changing assignment semantics, non-direct reuse routing, field-alias recovery, owner resolution, or the Slice 3 predicate.
- A declaration whose scope contains the call becomes invisible, or a sibling/ended declaration still poisons the owner.
- Exact-base RED does not compile/select the intended tests, or candidate GREEN relies on a malformed Go fixture.
- Any full-suite/Tier-A/corpus regression lacks a same-environment exact-base control.
