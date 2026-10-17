# DFG migration oracle controls

`cargo test --features mcp,dfg-oracle --test dfg_compaction` runs parity and stored
hash checks. CI runs `-- --skip pins::` so intended output changes in other lanes
can pass parity before the controller re-pins on main. `capture::capture_manifests`
is feature-gated by the test target and ignored by default; it writes both sorted
SHA-256 manifests under the explicit `DFG_PIN_DIR`, never into the source tree.
The capture visitor must equal the named parity population. Accounting and the
population census share `cases::registered_names()`; accounting skips discovered
but unregistered fixtures and does not use stored hash counts. The census tests
(`pins::census_*`) still require every fixture to be registered and captured; they
run with the stored hashes, not in CI's parity job. Fixture changes select parity
through `scripts/ci/dfg-oracle-paths.sh`.

`src/cpg_cache/dfg_observation.rs` is a read-only adapter, with no oracle dependency.
It observes ordered node/edge payload, a field-wise **legacy DFG tuple**, telemetry,
and all four lookup indexes (keys, node IDs, vector order, empty rows). The field
sequence is shared with `test_support::current_storage`; do not serialize the
production DFG object for pins after storage activation. `payload` is the current
codec control; PR A asserts byte equality with the field-wise form on every case.
PR E must retain the field-wise legacy observation while its native codec changes.

## Capture on clean main, then reproduce unchanged pins

Use a new empty scratch directory. Archive main; do not bless output from the
compaction branch. Use main's own committed capture test directly, with no
overlay. No production code, producer, fixture, common generator or assembly is
copied from the branch. Record the full main SHA with the receipt.

From the compaction checkout, set `capture_root` to an absolute scratch path and
`capture_sha` to the clean main SHA (see the manifest headers for the latest
rebind):

```bash
mkdir -p "$capture_root/main"
git archive "$capture_sha" | tar -x -C "$capture_root/main"
cd "$capture_root/main"
nice -n 19 env CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR="$capture_root/target" \
  RUST_MIN_STACK=16777216 DFG_PIN_DIR="$capture_root/manifests" \
  cargo test --offline --features mcp,dfg-oracle --test dfg_compaction \
  capture::capture_manifests -- --ignored --nocapture > "$capture_root/capture.log" 2>&1
cat "$capture_root/capture.log"
```

If the host denies `nice`, use the owner-authorized `env CARGO_BUILD_JOBS=4`
fallback and record that fact. One cargo process at a time.

Only for pre-A main (before the oracle feature and capture test existed), run
`python3 tests/dfg_compaction/prepare_capture.py "$capture_root/main"` before
changing directories and use `--test dfg_capture -- --ignored --nocapture`
instead. This historical bootstrap overlays only read-only capture support and
refuses to overwrite an existing adapter/target; record its diff with the receipt.

Compare **case-to-digest maps** (or sorted non-comment lines) in both generated
manifests with committed pins. Existing payload row order is preserved.
For a first independent index capture, keep the payload hashes as the control.
Then explicitly run the same capture test on the compaction branch with a different
`DFG_PIN_DIR`; both manifests must reproduce clean main exactly. Run `pins::` too.
Never use branch capture to bless changed output.

At every subsequent compaction PR, first recapture on clean main. Unchanged cases
must reproduce. Put changed hashes in a separate first commit named
`re-pin on main <sha>`, listing changed cases and the intervening main commits that
touched DFG/CPG output. Capture before and after each responsible main commit in
the same environment to distinguish its effect from earlier changes or drift.
Include registration for newly captured fixtures with the re-pin. Then the
compaction or harness change keeps every hash. Headers cite the capture SHA, this
procedure and the committed capture test.


## PR E merge control on main b2cd10a1 (2026-10-17)

The committed `capture::capture_manifests` test ran without overlays on a clean
`git archive b2cd10a157bb7d7e75283be6519b6fe2ce485e0f`. Both 133-case maps
reproduced main's committed manifests (last captured at #442, `9459a668`) exactly.
Thus neither intervening #445 (accounting/CI selection) nor #441 (syntax metadata)
introduces pinned-output drift. The merged PR E capture then reproduced both
clean-main maps byte for byte; no hash was blessed from the compaction branch.

Relative to PR E's previous `0a269d5e` pins, each map has 103 unchanged and the
same 30 changed cases already recorded by main for #442; no case was added or
removed. This merge adopts those accepted main hashes and updates provenance.
See [repin-b2cd10a1.md](repin-b2cd10a1.md) for the case list and receipt paths.
Accounting, cases, census and registrations are identical to both parents, with
one `argv-stub.sh` registration. Main's `tests/fixtures/*` CI selector is retained.

The clean-main run printed `nice: setpriority: Operation not permitted` and
continued successfully; subsequent runs used `env CARGO_BUILD_JOBS=4` without
`nice`. Captures used offline dependencies and `RUST_MIN_STACK=16777216`.
