# DFG migration oracle controls

`cargo test --features mcp,dfg-oracle --test dfg_compaction` runs parity and stored
hash checks. CI runs `-- --skip pins::` so intended output changes in other lanes
can pass parity before the controller re-pins on main. `capture::capture_manifests`
is feature-gated by the test target and ignored by default; it writes both sorted
SHA-256 manifests under the explicit `DFG_PIN_DIR`, never into the source tree.
The capture visitor must equal the named parity population (the manifest rows; 132 cases at b994f11e).

`src/cpg_cache/dfg_observation.rs` is a read-only adapter, with no oracle dependency.
It observes ordered node/edge payload, a field-wise **legacy DFG tuple**, telemetry,
and all four lookup indexes (keys, node IDs, vector order, empty rows). The field
sequence is shared with `test_support::current_storage`; do not serialize the
production DFG object for pins after storage activation. `payload` is the current
codec control; PR A asserts byte equality with the field-wise form on every case.
PR E must retain the field-wise legacy observation while its native codec changes.

## Capture on clean main, then reproduce unchanged pins

Use a new empty scratch directory. Archive main; do not capture on the compaction
branch. After PR A lands, use main's own committed capture test directly:
`cargo test --features mcp,dfg-oracle --test dfg_compaction capture::capture_manifests -- --ignored --nocapture`
(with the same environment and log redirection below). No overlay is needed then.
The overlay below bootstraps the initial capture on pre-A main, where no oracle
feature or capture test exists. No production code, producer, fixture, common generator or assembly is
copied from the branch. Only the committed read-only adapter, capture visitor/test,
feature declaration and test entry are overlaid. `tests/common` and all fixtures
remain those from main. Record the full main SHA and adapter diff with the receipt.

From the compaction checkout, set `capture_root` to an absolute scratch path and
`capture_sha` to the clean main SHA (this rebind: `cc20841b11e68ef758697bf9108a3555fb38e38e`):

```bash
mkdir -p "$capture_root/main"
git archive "$capture_sha" | tar -x -C "$capture_root/main"
python3 tests/dfg_compaction/prepare_capture.py "$capture_root/main"
cd "$capture_root/main"
nice -n 19 env CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR="$capture_root/target" \
  RUST_MIN_STACK=16777216 DFG_PIN_DIR="$capture_root/manifests" \
  cargo test --offline --features mcp,dfg-oracle --test dfg_capture \
  -- --ignored --nocapture > "$capture_root/capture.log" 2>&1
cat "$capture_root/capture.log"
```

If the host denies `nice`, use the owner-authorized `env CARGO_BUILD_JOBS=4`
fallback and record that fact. One cargo process at a time. The overlay helper
refuses an existing adapter/target, so it cannot overwrite a prior capture setup.

Compare **case-to-digest maps** (or sorted non-comment lines) in both generated
manifests with committed pins. Existing payload row order is preserved.
For a first independent index capture, keep the payload hashes as the control.
Then explicitly run the same capture test on the compaction branch with a different
`DFG_PIN_DIR`; both manifests must reproduce clean main exactly. Run `pins::` too.
Never use branch capture to bless changed output.

At every subsequent compaction PR, first recapture on clean main. Unchanged cases
must reproduce. Put changed hashes in a separate first commit named
`re-pin on main <sha>`, listing changed cases and the intervening main commits that
touched DFG/CPG output. Then the compaction change keeps every hash. Headers cite
the capture SHA, this procedure and the committed capture test.
